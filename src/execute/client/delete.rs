use std::marker::PhantomData;

use cosmwasm_std::{to_json_binary, Addr, Order, Response, StdResult, Storage, Uint64, WasmMsg};
use cw_storage_plus::{Bound, Deque, Map};

use crate::{
    context::Context,
    error::ContractError,
    lifecycle::{LifecycleArgs, LifecycleExecuteMsg, LifecycleExecuteMsgEnvelope},
    models::ContractFlag,
    msg::IndexType,
    state::{
        ensure_allowed_by_acl, ensure_contract_not_suspended, incr_decr_index_size,
        load_contract_id, remove_from_group, ContractID, CustomIndexMap, CONTRACT_ADDR_2_ID,
        CONTRACT_DYN_METADATA, CONTRACT_GROUP_IDS, CONTRACT_ID_2_ADDR, CONTRACT_INDEX_TYPES,
        CONTRACT_METADATA, CONTRACT_SUSPENSIONS, CONTRACT_TAGS, CONTRACT_USES_LIFECYCLE_HOOKS,
        IX_CODE_ID, IX_CONTRACT_ID, IX_CREATED_AT, IX_CREATED_BY, IX_REV, IX_TAG, IX_UPDATED_AT,
        IX_UPDATED_BY, PARTITION_SIZES, PARTITION_TAG_COUNTS, REL_ADDR_2_ID, REL_ID_2_ADDR,
        VALUES_BINARY, VALUES_BOOL, VALUES_I32, VALUES_STRING, VALUES_TIME, VALUES_U128,
        VALUES_U16, VALUES_U32, VALUES_U64, VALUES_U8,
    },
    util::build_index_storage_key,
};

// Replace the existing config in its entirety.
pub fn on_execute(
    ctx: Context,
    contract_addr: Addr,
) -> Result<Response, ContractError> {
    let Context { deps, info, env } = ctx;
    let action = "delete";

    deps.api.addr_validate(contract_addr.as_str())?;

    let contract_id = load_contract_id(deps.storage, &contract_addr)?;
    let mut resp = Response::new().add_attribute("action", action);

    // If sender isn't the contract itself, only allow sender if auth'd by owner
    // address or ACL.
    if contract_addr != info.sender {
        ensure_allowed_by_acl(&deps, &info.sender, "/table/delete")?;
    } else {
        ensure_contract_not_suspended(deps.storage, contract_id)?;
    };

    if CONTRACT_USES_LIFECYCLE_HOOKS
        .may_load(deps.storage, contract_id)?
        .unwrap_or_default()
    {
        resp = resp.add_message(WasmMsg::Execute {
            contract_addr: contract_addr.clone().into(),
            msg: to_json_binary(&LifecycleExecuteMsgEnvelope::Lifecycle(
                LifecycleExecuteMsg::Teardown(LifecycleArgs {
                    table: env.contract.address.clone(),
                    initiator: info.sender.clone(),
                }),
            ))?,
            funds: vec![],
        });
    }

    delete_from_indices(deps.storage, contract_id)?;
    delete_from_tags(deps.storage, contract_id)?;
    delete_from_relationships(deps.storage, contract_id)?;
    delete_from_partition(deps.storage, &contract_addr, contract_id)?;
    delete_from_groups(deps.storage, contract_id)?;

    Ok(resp)
}

fn delete_from_groups(
    storage: &mut dyn Storage,
    contract_id: ContractID,
) -> Result<(), ContractError> {
    for maybe_group_id in CONTRACT_GROUP_IDS
        .prefix(contract_id)
        .keys(storage, None, None, Order::Ascending)
        .collect::<Vec<StdResult<_>>>()
    {
        let group_id = maybe_group_id?;
        remove_from_group(storage, group_id, contract_id)?;
    }
    Ok(())
}

fn delete_from_partition(
    storage: &mut dyn Storage,
    contract_addr: &Addr,
    id: ContractID,
) -> Result<(), ContractError> {
    let meta = CONTRACT_METADATA.load(storage, id)?;

    // Delete metadatas
    CONTRACT_METADATA.remove(storage, id);
    CONTRACT_DYN_METADATA.remove(storage, id);

    // Remove mappings between contract ID <-> contract Addr
    CONTRACT_ID_2_ADDR.remove(storage, id);
    CONTRACT_ADDR_2_ID.remove(storage, contract_addr);

    // Clear suspension flags
    CONTRACT_SUSPENSIONS.remove(storage, id);

    // Decrement parition size
    PARTITION_SIZES.update(
        storage,
        meta.partition,
        |maybe_n| -> Result<_, ContractError> {
            maybe_n
                .unwrap_or_default()
                .checked_sub(Uint64::one())
                .map_err(|e| ContractError::UnexpectedError {
                    reason: e.to_string(),
                })
        },
    )?;

    // Clear ContractFlags
    let flags_deque_key = format!("_flags_{}", id);
    let flags: Deque<ContractFlag> = Deque::new(flags_deque_key.as_str());

    for _ in 0..flags.len(storage)? {
        flags.pop_front(storage)?;
    }

    Ok(())
}

fn delete_from_tags(
    storage: &mut dyn Storage,
    id: ContractID,
) -> Result<(), ContractError> {
    let meta = CONTRACT_METADATA.load(storage, id)?;
    let p = meta.partition;

    for result in CONTRACT_TAGS
        .prefix(id)
        .keys(storage, None, None, Order::Ascending)
        .collect::<Vec<StdResult<String>>>()
    {
        let tag = result?;

        // Clear index used for finding contract by tags
        IX_TAG.remove(storage, (p, &tag, id));

        // Decrement the global counts for each tag removed (in the contract's current partition)
        let updated_tag_count = PARTITION_TAG_COUNTS.update(
            storage,
            (p, &tag),
            |maybe_n| -> Result<_, ContractError> {
                if let Some(n) = maybe_n {
                    n.checked_sub(1).ok_or(ContractError::UnexpectedError {
                        reason: format!("cannot subtract from invalid 0 count for tag '{}'", tag),
                    })
                } else {
                    Err(ContractError::UnexpectedError {
                        reason: format!(
                            "cannot subtract non-existent tag count for '{}' in partition {}",
                            tag, p
                        ),
                    })
                }
            },
        )?;

        // Remove the partition's tag counter for this tag if count drops to 0.
        if updated_tag_count == 0 {
            PARTITION_TAG_COUNTS.remove(storage, (p, &tag));
        }
    }

    Ok(())
}

fn delete_from_indices(
    storage: &mut dyn Storage,
    id: ContractID,
) -> Result<(), ContractError> {
    let meta = CONTRACT_METADATA.load(storage, id)?;
    let p = meta.partition;

    // Remove from main metadata indices
    IX_CONTRACT_ID.remove(storage, (p, id, id));
    IX_CODE_ID.remove(storage, (p, meta.code_id.into(), id));
    IX_CREATED_AT.remove(storage, (p, meta.created_at.nanos(), id));
    IX_CREATED_BY.remove(storage, (p, meta.created_by.to_string(), id));

    if let Some(up_meta) = CONTRACT_DYN_METADATA.may_load(storage, id)? {
        // Remove from "update" metadata indices
        IX_UPDATED_AT.remove(storage, (p, up_meta.updated_at.nanos(), id));
        IX_UPDATED_BY.remove(storage, (p, up_meta.updated_by.to_string(), id));
        IX_REV.remove(storage, (p, up_meta.rev.into(), id));
    }

    // Remove from custom indices
    for result in CONTRACT_INDEX_TYPES
        .prefix(id)
        .range(storage, None, None, Order::Ascending)
        .collect::<Vec<StdResult<_>>>()
    {
        let (index_name, index_type) = result?;

        CONTRACT_INDEX_TYPES.remove(storage, (id, &index_name));

        incr_decr_index_size(storage, &index_name, false)?;

        let index_storage_key = build_index_storage_key(&index_name);

        match index_type {
            IndexType::String => {
                if let Some(v) = VALUES_STRING.may_load(storage, (id, &index_name))? {
                    VALUES_STRING.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<&String> = Map::new(&index_storage_key);
                    index.remove(storage, (p, &v, id));
                }
            },
            IndexType::Bool => {
                if let Some(v) = VALUES_BOOL.may_load(storage, (id, &index_name))? {
                    VALUES_BOOL.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u8> = Map::new(&index_storage_key);
                    index.remove(storage, (p, if v { 1 } else { 0 }, id));
                }
            },
            IndexType::Timestamp => {
                if let Some(v) = VALUES_TIME.may_load(storage, (id, &index_name))? {
                    VALUES_TIME.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u64> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v.nanos(), id));
                }
            },
            IndexType::Int32 => {
                if let Some(v) = VALUES_I32.may_load(storage, (id, &index_name))? {
                    VALUES_I32.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<i32> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v, id));
                }
            },
            IndexType::Uint8 => {
                if let Some(v) = VALUES_U8.may_load(storage, (id, &index_name))? {
                    VALUES_U8.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u8> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v, id));
                }
            },
            IndexType::Uint16 => {
                if let Some(v) = VALUES_U16.may_load(storage, (id, &index_name))? {
                    VALUES_U16.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u16> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v, id));
                }
            },
            IndexType::Uint32 => {
                if let Some(v) = VALUES_U32.may_load(storage, (id, &index_name))? {
                    VALUES_U32.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u32> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v, id));
                }
            },
            IndexType::Uint64 => {
                if let Some(v) = VALUES_U64.may_load(storage, (id, &index_name))? {
                    VALUES_U64.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u64> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v.u64(), id));
                }
            },
            IndexType::Uint128 => {
                if let Some(v) = VALUES_U128.may_load(storage, (id, &index_name))? {
                    VALUES_U128.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<u128> = Map::new(&index_storage_key);
                    index.remove(storage, (p, v.u128(), id));
                }
            },
            IndexType::Binary => {
                if let Some(v) = VALUES_BINARY.may_load(storage, (id, &index_name))? {
                    VALUES_BINARY.remove(storage, (id, &index_name));
                    let index: CustomIndexMap<&[u8]> = Map::new(&index_storage_key);
                    index.remove(storage, (p, &v.as_slice(), id));
                }
            },
        }
    }

    Ok(())
}

fn delete_from_relationships(
    storage: &mut dyn Storage,
    id: ContractID,
) -> Result<(), ContractError> {
    for result in REL_ID_2_ADDR
        .keys(
            storage,
            Some(Bound::Inclusive((
                (id, "".to_owned(), "".to_owned()),
                PhantomData,
            ))),
            None,
            Order::Ascending,
        )
        .collect::<Vec<StdResult<_>>>()
    {
        let (contract_id, rel_name, account_addr) = result?;

        REL_ID_2_ADDR.remove(
            storage,
            (contract_id, rel_name.clone(), account_addr.clone()),
        );
        REL_ADDR_2_ID.remove(storage, (account_addr, rel_name, id.to_string()));
    }

    Ok(())
}

use cosmwasm_std::{Addr, Order, Response, StdResult, Storage, Uint64};
use cw_storage_plus::Map;

use crate::{
    context::Context,
    error::ContractError,
    msg::{IndexType, PartitionSelector},
    state::{
        decrement_tag_count, ensure_allowed_by_acl, ensure_contract_not_suspended,
        ensure_partition_exists, increment_tag_count, load_contract_id, resolve_partition_id,
        ContractID, CustomIndexMap, PartitionID, CONTRACT_DYN_METADATA, CONTRACT_INDEX_TYPES,
        CONTRACT_METADATA, CONTRACT_TAGS, IX_CODE_ID, IX_CONTRACT_ID, IX_CREATED_AT, IX_CREATED_BY,
        IX_REV, IX_TAG, IX_UPDATED_AT, IX_UPDATED_BY, PARTITION_SIZES, VALUES_BINARY, VALUES_BOOL,
        VALUES_I32, VALUES_STRING, VALUES_TIME, VALUES_U128, VALUES_U16, VALUES_U32, VALUES_U64,
        VALUES_U8, X,
    },
    util::build_index_storage_key,
};

/// Move the contract to a new partition.
pub fn on_execute(
    ctx: Context,
    contract_addr: Addr,
    dst_selector: PartitionSelector,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    let action = "partition";

    deps.api.addr_validate(contract_addr.as_str())?;

    let dst_partition = resolve_partition_id(deps.storage, dst_selector)?;

    // TODO: make this a client contract method

    ensure_allowed_by_acl(&deps, &info.sender, "/table/set-partition")?;
    ensure_partition_exists(deps.storage, dst_partition)?;

    let contract_id = load_contract_id(deps.storage, &contract_addr)?;

    ensure_contract_not_suspended(deps.storage, contract_id)?;

    let meta = CONTRACT_METADATA.load(deps.storage, contract_id)?;

    if meta.partition != dst_partition {
        let src_partition = meta.partition;
        update_contract_partition(deps.storage, contract_id, src_partition, dst_partition)?;
    } else {
        return Err(ContractError::NotAuthorized {
            reason: format!(
                "contract {} already belongs to partition {}",
                contract_addr.as_str(),
                dst_partition
            ),
        });
    }

    Ok(Response::new().add_attribute("action", action))
}

pub fn move_tags(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    src: PartitionID,
    dst: PartitionID,
) -> Result<(), ContractError> {
    for result in CONTRACT_TAGS
        .prefix(contract_id)
        .keys(storage, None, None, Order::Ascending)
        .collect::<Vec<StdResult<_>>>()
    {
        let tag = result?;

        // Subtract the existing tag counts in the src partition, incrementing the
        // tag counts in the dst partition proportionally.
        decrement_tag_count(storage, src, &tag)?;
        increment_tag_count(storage, dst, &tag)?;

        // Move the tag to a new partition in the tags index
        IX_TAG.remove(storage, (src, &tag, contract_id));
        IX_TAG.save(storage, (dst, &tag, contract_id), &X)?;
    }

    Ok(())
}

pub fn update_contract_partition(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    src: PartitionID,
    dst: PartitionID,
) -> Result<(), ContractError> {
    move_standard_indices(storage, contract_id, src, dst)?;
    move_custom_indices(storage, contract_id, src, dst)?;
    move_tags(storage, contract_id, src, dst)?;

    PARTITION_SIZES.update(storage, src, |maybe_n| -> Result<_, ContractError> {
        maybe_n
            .unwrap_or_default()
            .checked_sub(Uint64::one())
            .map_err(|_| ContractError::UnexpectedError {
                reason: "trying to subtract from 0 partition count".to_owned(),
            })
    })?;

    PARTITION_SIZES.update(storage, src, |maybe_n| -> Result<_, ContractError> {
        maybe_n
            .unwrap_or_default()
            .checked_add(Uint64::one())
            .map_err(|e| ContractError::UnexpectedError {
                reason: e.to_string(),
            })
    })?;

    Ok(())
}

fn move_standard_indices(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    src: PartitionID,
    dst: PartitionID,
) -> Result<(), ContractError> {
    // Update core metadata indices
    let meta = CONTRACT_METADATA.load(storage, contract_id)?;

    IX_CONTRACT_ID.remove(storage, (src, contract_id, contract_id));
    IX_CONTRACT_ID.save(storage, (dst, contract_id.into(), contract_id), &X)?;

    IX_CODE_ID.remove(storage, (src, meta.code_id.into(), contract_id));
    IX_CODE_ID.save(storage, (dst, meta.code_id.into(), contract_id), &X)?;

    IX_CREATED_BY.remove(storage, (src, meta.created_by.to_string(), contract_id));
    IX_CREATED_BY.save(storage, (dst, meta.created_by.to_string(), contract_id), &X)?;

    IX_CREATED_AT.remove(storage, (src, meta.created_at.nanos(), contract_id));
    IX_CREATED_AT.save(storage, (dst, meta.created_at.nanos(), contract_id), &X)?;

    // Update dynamic metadata indices
    if let Some(up_meta) = CONTRACT_DYN_METADATA.may_load(storage, contract_id)? {
        IX_UPDATED_BY.remove(storage, (src, up_meta.updated_by.to_string(), contract_id));
        IX_UPDATED_BY.save(
            storage,
            (dst, up_meta.updated_by.to_string(), contract_id),
            &X,
        )?;
        IX_UPDATED_AT.remove(storage, (src, up_meta.updated_at.nanos(), contract_id));
        IX_UPDATED_AT.save(storage, (dst, up_meta.updated_at.nanos(), contract_id), &X)?;
        IX_REV.remove(storage, (src, up_meta.rev.into(), contract_id));
        IX_REV.save(storage, (dst, up_meta.rev.into(), contract_id), &X)?;
    } else {
        // Dynamic metadata doesn't exist until first update. in this case,
        // assume existing values are initial values.
        IX_UPDATED_BY.remove(storage, (src, meta.created_at.to_string(), contract_id));
        IX_UPDATED_BY.save(storage, (dst, meta.created_at.to_string(), contract_id), &X)?;
        IX_UPDATED_AT.remove(storage, (src, meta.created_at.nanos(), contract_id));
        IX_UPDATED_AT.save(storage, (dst, meta.created_at.nanos(), contract_id), &X)?;
        IX_REV.remove(storage, (src, 1, contract_id));
        IX_REV.save(storage, (dst, 1, contract_id), &X)?;
    }

    Ok(())
}

pub fn move_custom_indices(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    src: PartitionID,
    dst: PartitionID,
) -> Result<(), ContractError> {
    let entries: Vec<(String, IndexType)> = CONTRACT_INDEX_TYPES
        .prefix(contract_id)
        .range(storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .collect();

    for (index_name, index_type) in entries.iter() {
        let index_storage_key = build_index_storage_key(index_name);
        match index_type {
            IndexType::String => {
                let index: CustomIndexMap<&String> = Map::new(&index_storage_key);
                let value = VALUES_STRING.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, &value, contract_id));
                index.save(storage, (dst, &value, contract_id), &X)?;
            },
            IndexType::Bool => {
                let index: CustomIndexMap<u8> = Map::new(&index_storage_key);
                let value = VALUES_BOOL.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Timestamp => {
                let index: CustomIndexMap<u64> = Map::new(&index_storage_key);
                let value = VALUES_TIME.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.nanos(), contract_id));
                index.save(storage, (dst, value.nanos(), contract_id), &X)?;
            },
            IndexType::Int32 => {
                let index: CustomIndexMap<i32> = Map::new(&index_storage_key);
                let value = VALUES_I32.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Uint8 => {
                let index: CustomIndexMap<u8> = Map::new(&index_storage_key);
                let value = VALUES_U8.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Uint16 => {
                let index: CustomIndexMap<u16> = Map::new(&index_storage_key);
                let value = VALUES_U16.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Uint32 => {
                let index: CustomIndexMap<u32> = Map::new(&index_storage_key);
                let value = VALUES_U32.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Uint64 => {
                let index: CustomIndexMap<u64> = Map::new(&index_storage_key);
                let value = VALUES_U64.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Uint128 => {
                let index: CustomIndexMap<u128> = Map::new(&index_storage_key);
                let value = VALUES_U128.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.into(), contract_id));
                index.save(storage, (dst, value.into(), contract_id), &X)?;
            },
            IndexType::Binary => {
                let index: CustomIndexMap<&[u8]> = Map::new(&index_storage_key);
                let value = VALUES_BINARY.load(storage, (contract_id, &index_storage_key))?;
                index.remove(storage, (src, value.as_slice(), contract_id));
                index.save(storage, (dst, value.as_slice(), contract_id), &X)?;
            },
        }
    }

    Ok(())
}

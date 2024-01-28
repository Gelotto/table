use crate::{
    context::Context,
    error::ContractError,
    models::DynamicContractMetadata,
    msg::{IndexType, KeyValue, Relationship, RelationshipUpdates, TagUpdates, UpdateParams},
    state::{
        decrement_tag_count, ensure_allowed_by_acl, ensure_contract_not_suspended,
        incr_decr_index_size, increment_tag_count, load_contract_id, ContractID, CustomIndexMap,
        PartitionID, CONFIG_STR_MAX_LEN, CONTRACT_DYN_METADATA, CONTRACT_INDEX_TYPES,
        CONTRACT_METADATA, CONTRACT_TAGS, IX_REV, IX_TAG, IX_UPDATED_AT, IX_UPDATED_BY, NOT_UNIQUE,
        REL_ADDR_2_ID, REL_ID_2_ADDR, UNIQUE, VALUES_BINARY, VALUES_BOOL, VALUES_I32,
        VALUES_STRING, VALUES_TIME, VALUES_U128, VALUES_U16, VALUES_U32, VALUES_U64, VALUES_U8, X,
    },
    util::{build_index_storage_key, pad, trim_padding},
};
use cosmwasm_std::{attr, Addr, Binary, Env, Order, Response, Storage, Timestamp, Uint128, Uint64};
use cw_storage_plus::Map;

pub fn on_execute(
    ctx: Context,
    params: UpdateParams,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let action = "update";

    // Get address of contract whose state we will update. If sender isn't the
    // contract itself, only allow sender if auth'd by owner address or ACL.
    let contract_addr = if params.contract != info.sender {
        ensure_allowed_by_acl(&deps, &info.sender, "/table/update")?;
        deps.api.addr_validate(params.contract.as_str())?
    } else {
        info.sender
    };

    deps.api.addr_validate(params.initiator.as_str())?;

    let initiator = params.initiator;
    let contract_id = load_contract_id(deps.storage, &contract_addr)?;

    ensure_contract_not_suspended(deps.storage, contract_id)?;

    let partition = CONTRACT_METADATA.load(deps.storage, contract_id)?.partition;
    let max_str_len = CONFIG_STR_MAX_LEN.load(deps.storage)? as usize;

    // Update built-in and custom indices
    if let Some(index_updates) = params.values {
        upsert_metadata(deps.storage, &env, partition, &initiator, contract_id)?;
        update_indices(
            deps.storage,
            partition,
            contract_id,
            index_updates,
            max_str_len,
        )?;
    }

    // Update tags
    if let Some(tag_updates) = params.tags.clone() {
        update_tags(
            deps.storage,
            partition,
            contract_id,
            tag_updates,
            max_str_len,
        )?;
    }

    // Update relationships
    if let Some(rel_updates) = params.relationships.clone() {
        update_relationships(deps.storage, contract_id, rel_updates, max_str_len)?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", action)]))
}

fn upsert_metadata(
    storage: &mut dyn Storage,
    env: &Env,
    partition: PartitionID,
    initiator: &Addr,
    contract_id: ContractID,
) -> Result<(), ContractError> {
    let mut maybe_prev_meta: Option<DynamicContractMetadata> = None;

    let meta = CONTRACT_DYN_METADATA.update(
        storage,
        contract_id,
        |maybe_meta| -> Result<_, ContractError> {
            if let Some(mut meta) = maybe_meta {
                maybe_prev_meta = Some(meta.clone());
                meta.rev += Uint64::one();
                meta.updated_at = env.block.time;
                meta.updated_at_height = env.block.height.into();
                meta.updated_by = initiator.clone();
                Ok(meta)
            } else {
                Ok(DynamicContractMetadata {
                    rev: Uint64::one(),
                    updated_at: env.block.time,
                    updated_at_height: env.block.height.into(),
                    updated_by: initiator.clone(),
                })
            }
        },
    )?;

    if let Some(prev_meta) = maybe_prev_meta {
        IX_REV.remove(storage, (partition, prev_meta.rev.into(), contract_id));
        IX_UPDATED_AT.remove(
            storage,
            (partition, prev_meta.updated_at.nanos(), contract_id),
        );
    }

    IX_REV.save(storage, (partition, meta.rev.into(), contract_id), &X)?;
    IX_UPDATED_AT.save(
        storage,
        (partition, meta.updated_at.nanos(), contract_id),
        &X,
    )?;
    IX_UPDATED_BY.save(storage, (partition, initiator.to_string(), contract_id), &X)?;

    Ok(())
}

fn update_tags(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    updates: TagUpdates,
    max_str_len: usize,
) -> Result<(), ContractError> {
    if let Some(tags_to_remove) = &updates.remove {
        for tag_string in tags_to_remove.iter() {
            let tag_string = &pad(&tag_string, max_str_len);
            IX_TAG.remove(storage, (partition, tag_string, contract_id));
            CONTRACT_TAGS.remove(storage, (contract_id, tag_string.clone()));
            decrement_tag_count(storage, partition, tag_string)?;
        }
    }

    if let Some(tags_to_add) = &updates.add {
        for tag in tags_to_add.iter() {
            let tag_string = &pad(&tag.text, max_str_len);
            ensure_uniqueness(storage, partition, &tag_string)?;
            IX_TAG.save(
                storage,
                (partition, tag_string, contract_id),
                &if tag.unique.unwrap_or(false) {
                    UNIQUE
                } else {
                    NOT_UNIQUE
                },
            )?;
            let mut do_increment_tag_count = false;
            CONTRACT_TAGS.update(
                storage,
                (contract_id, tag_string.clone()),
                |dummy| -> Result<_, ContractError> {
                    if dummy.is_none() {
                        do_increment_tag_count = true;
                    }
                    Ok(X)
                },
            )?;
            if do_increment_tag_count {
                increment_tag_count(storage, partition, &tag_string)?;
            }
        }
    }

    Ok(())
}
fn ensure_uniqueness(
    storage: &dyn Storage,
    partition: PartitionID,
    cannonical_tag: &String,
) -> Result<(), ContractError> {
    for entry in IX_TAG
        .prefix((partition, cannonical_tag))
        .range(storage, None, None, Order::Ascending)
        .take(1)
    {
        let (_, uniqueness) = entry?;
        if uniqueness == UNIQUE {
            return Err(ContractError::NotAuthorized {
                reason: format!(
                    "tag {} is unique and already used",
                    trim_padding(&cannonical_tag)
                ),
            });
        }
    }
    Ok(())
}

fn update_relationships(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    updates: RelationshipUpdates,
    max_str_len: usize,
) -> Result<(), ContractError> {
    if let Some(rels) = &updates.remove {
        for rel in rels.iter() {
            remove_relationship(storage, contract_id, &rel, max_str_len)?;
        }
    }

    if let Some(rels) = &updates.add {
        for rel in rels.iter() {
            set_relationship(storage, contract_id, &rel, max_str_len)?;
        }
    }

    Ok(())
}

fn set_relationship(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    rel: &Relationship,
    max_str_len: usize,
) -> Result<(), ContractError> {
    let addr_str = rel.address.to_string();
    let rel_name = pad(&rel.name, max_str_len);
    let uniqueness_u8 = if rel.unique { UNIQUE } else { NOT_UNIQUE };

    // Check if a relationship with the given name already exists for the given
    // address and abort if said relationship is unique.
    for result in REL_ADDR_2_ID
        .prefix((addr_str.clone(), rel_name.clone()))
        .range(storage, None, None, Order::Ascending)
        .take(1)
    {
        let (_, uniqueness) = result?;
        if uniqueness == UNIQUE {
            return Err(ContractError::ValidationError {
                reason: format!("Relationship {} is unique", trim_padding(&rel_name)),
            });
        }
    }

    REL_ADDR_2_ID.save(
        storage,
        (addr_str.clone(), rel_name.clone(), contract_id.to_string()),
        &uniqueness_u8,
    )?;

    REL_ID_2_ADDR.save(
        storage,
        (contract_id, rel_name.clone(), addr_str.clone()),
        &uniqueness_u8,
    )?;
    Ok(())
}

fn remove_relationship(
    storage: &mut dyn Storage,
    contract_id: ContractID,
    rel: &Relationship,
    max_str_len: usize,
) -> Result<(), ContractError> {
    let addr_str = rel.address.to_string();
    let rel_name = pad(&rel.name, max_str_len);
    if let Some(related_addr) =
        REL_ID_2_ADDR.may_load(storage, (contract_id, rel_name.clone(), addr_str.clone()))?
    {
        REL_ADDR_2_ID.remove(
            storage,
            (addr_str.clone(), rel_name.clone(), contract_id.to_string()),
        );
        REL_ID_2_ADDR.remove(
            storage,
            (contract_id, rel_name.clone(), related_addr.to_string()),
        );
    }
    Ok(())
}

fn update_indices(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_updates: Vec<KeyValue>,
    max_str_len: usize,
) -> Result<(), ContractError> {
    // Update each index for the given KeyValue. If the given value is None, use
    // this as a signal to remove the existing entry, if any, from the index.
    for value in index_updates.iter() {
        match value {
            KeyValue::String(key, value) => {
                update_string_index(storage, partition, contract_id, key, value, max_str_len)?
            },
            KeyValue::Bool(key, value) => {
                update_bool_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Timestamp(key, value) => {
                update_timestamp_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Int32(key, value) => {
                update_i32_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Uint8(key, value) => {
                update_u8_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Uint16(key, value) => {
                update_u16_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Uint32(key, value) => {
                update_u32_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Uint64(key, value) => {
                update_u64_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Uint128(key, value) => {
                update_u128_index(storage, partition, contract_id, key, value)?
            },
            KeyValue::Binary(key, value) => {
                update_binary_index(storage, partition, contract_id, key, value)?
            },
        }
    }
    Ok(())
}

fn update_string_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<String>,
    max_str_len: usize,
) -> Result<(), ContractError> {
    let index_storage_key = build_index_storage_key(index_name);
    let index: CustomIndexMap<&String> = Map::new(&index_storage_key);
    let indexed_value_map = VALUES_STRING;

    if let Some(new_val) = maybe_value {
        let new_val = &pad(&new_val, max_str_len);
        let index_key = (partition, new_val, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        let mut maybe_old_val: Option<String> = None;
        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(new_val.clone())
            },
        )?;

        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, &old_val, contract_id));
        }

        index.save(storage, (partition, new_val, contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, index_name), &IndexType::String)?;
        }
        incr_decr_index_size(storage, index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, index_name))? {
        let index_key = (partition, &old_val, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, index_name));
            incr_decr_index_size(storage, index_name, false)?;
        }
    }
    Ok(())
}

fn update_bool_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<bool>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u8> = Map::new(&index_slot);
    let indexed_value_map = VALUES_BOOL;
    let mut maybe_old_bool: Option<bool> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, if *new_val { 1u8 } else { 0u8 }, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_bool = x;
                Ok(*new_val)
            },
        )?;

        if let Some(old_val) = maybe_old_bool {
            index.remove(
                storage,
                (partition, if old_val { 1 } else { 0 }, contract_id),
            );
        }

        index.save(storage, index_key, &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Bool)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, if old_val { 1u8 } else { 0u8 }, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_timestamp_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<Timestamp>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u64> = Map::new(&index_slot);
    let indexed_value_map = VALUES_TIME;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, new_val.nanos(), contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        let mut maybe_old_val: Option<Timestamp> = None;
        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(*new_val)
            },
        )?;

        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val.nanos(), contract_id));
        }

        index.save(storage, (partition, new_val.nanos(), contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(
                storage,
                (contract_id, &index_name),
                &IndexType::Timestamp,
            )?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val.nanos(), contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, (partition, old_val.nanos(), contract_id));
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_u8_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<u8>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u8> = Map::new(&index_slot);
    let indexed_value_map = VALUES_U8;
    let mut maybe_old_val: Option<u8> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, *new_val, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(*new_val)
            },
        )?;
        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val, contract_id));
        }
        index.save(storage, (partition, *new_val, contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint8)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_u16_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<u16>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u16> = Map::new(&index_slot);
    let indexed_value_map = VALUES_U16;
    let mut maybe_old_val: Option<u16> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, *new_val, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(*new_val)
            },
        )?;

        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val, contract_id));
        }

        index.save(storage, (partition, *new_val, contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint16)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_i32_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<i32>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<i32> = Map::new(&index_slot);
    let indexed_value_map = VALUES_I32;
    let mut maybe_old_bool: Option<i32> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, *new_val, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_bool = x;
                Ok(*new_val)
            },
        )?;
        if let Some(old_val) = maybe_old_bool {
            index.remove(storage, (partition, old_val, contract_id));
        }
        index.save(storage, (partition, *new_val, contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Int32)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_u32_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<u32>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u32> = Map::new(&index_slot);
    let indexed_value_map = VALUES_U32;
    let mut maybe_old_bool: Option<u32> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, *new_val, contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_bool = x;
                Ok(*new_val)
            },
        )?;
        if let Some(old_val) = maybe_old_bool {
            index.remove(storage, (partition, old_val, contract_id));
        }
        index.save(storage, (partition, *new_val, contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint32)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val, contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_u64_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<Uint64>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u64> = Map::new(&index_slot);
    let indexed_value_map = VALUES_U64;
    let mut maybe_old_val: Option<Uint64> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, new_val.u64(), contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(*new_val)
            },
        )?;
        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val.into(), contract_id));
        }
        index.save(storage, (partition, (*new_val).into(), contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint64)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val.u64(), contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_u128_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<Uint128>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<u128> = Map::new(&index_slot);
    let indexed_value_map = VALUES_U128;
    let mut maybe_old_val: Option<Uint128> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, new_val.u128(), contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(*new_val)
            },
        )?;

        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val.into(), contract_id));
        }

        index.save(storage, (partition, (*new_val).into(), contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint128)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val.u128(), contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

fn update_binary_index(
    storage: &mut dyn Storage,
    partition: PartitionID,
    contract_id: ContractID,
    index_name: &String,
    maybe_value: &Option<Binary>,
) -> Result<(), ContractError> {
    let index_slot = build_index_storage_key(index_name);
    let index: CustomIndexMap<&[u8]> = Map::new(&index_slot);
    let indexed_value_map = VALUES_BINARY;
    let mut maybe_old_val: Option<Binary> = None;

    if let Some(new_val) = maybe_value {
        let index_key = (partition, new_val.as_slice(), contract_id);
        if index.has(storage, index_key) {
            return Ok(());
        }

        indexed_value_map.update(
            storage,
            (contract_id, &index_name.to_owned()),
            |x| -> Result<_, ContractError> {
                maybe_old_val = x;
                Ok(new_val.clone())
            },
        )?;

        if let Some(old_val) = maybe_old_val {
            index.remove(storage, (partition, old_val.as_slice(), contract_id));
        }

        index.save(storage, (partition, new_val.as_slice(), contract_id), &X)?;
        if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
            CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Uint128)?;
        }
        incr_decr_index_size(storage, &index_name, true)?;
    } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
        let index_key = (partition, old_val.as_slice(), contract_id);
        if index.has(storage, index_key) {
            index.remove(storage, index_key);
            indexed_value_map.remove(storage, (contract_id, index_name));
            CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
            incr_decr_index_size(storage, &index_name, false)?;
        }
    }
    Ok(())
}

use crate::{
  error::ContractError,
  models::DynamicContractMetadata,
  msg::{IndexType, KeyValue, TagUpdates, UpdateParams},
  state::{
    decrement_tag_count, ensure_contract_not_suspended, ensure_is_authorized_owner, increment_tag_count,
    load_contract_id, ContractID, CustomIndexMap, PartitionID, CONTRACT_DYN_METADATA, CONTRACT_INDEX_TYPES,
    CONTRACT_METADATA, INDEX_METADATA, IX_REV, IX_TAG, IX_UPDATED_AT, IX_UPDATED_BY, VALUES_BOOL, VALUES_STRING,
    VALUES_TIME, VALUES_U128, VALUES_U16, VALUES_U32, VALUES_U64, VALUES_U8, X,
  },
  util::build_index_storage_key,
};
use cosmwasm_std::{attr, Addr, DepsMut, Env, MessageInfo, Response, Storage, Timestamp, Uint128, Uint64};
use cw_storage_plus::Map;

pub fn on_execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  params: UpdateParams,
) -> Result<Response, ContractError> {
  let action = "update";

  // Get address of contract whose state we will update. If sender isn't the
  // contract itself, only allow sender if auth'd by owner address or ACL.
  let contract_addr = if let Some(contract_addr) = params.contract {
    ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;
    deps.api.addr_validate(contract_addr.as_str())?
  } else {
    info.sender
  };

  ensure_contract_not_suspended(deps.storage, &contract_addr)?;

  deps.api.addr_validate(params.initiator.as_str())?;

  let contract_id = load_contract_id(deps.storage, &contract_addr)?;
  let partition = CONTRACT_METADATA.load(deps.storage, contract_id)?.partition;
  let initiator = params.initiator;

  // Update built-in and custom indices
  if let Some(index_updates) = params.values {
    update_indices(deps.storage, partition, contract_id, index_updates)?;
    update_metadata(deps.storage, &env, partition, &initiator, contract_id)?;
  }

  // Update tags
  if let Some(tag_updates) = params.tags {
    update_tags(deps.storage, partition, contract_id, tag_updates)?;
  }

  Ok(Response::new().add_attributes(vec![attr("action", action)]))
}

fn update_metadata(
  storage: &mut dyn Storage,
  env: &Env,
  partition: PartitionID,
  initiator: &Addr,
  contract_id: ContractID,
) -> Result<(), ContractError> {
  let mut maybe_prev_meta: Option<DynamicContractMetadata> = None;

  let meta = CONTRACT_DYN_METADATA.update(storage, contract_id, |maybe_meta| -> Result<_, ContractError> {
    if let Some(mut meta) = maybe_meta {
      maybe_prev_meta = Some(meta.clone());
      meta.rev += Uint64::one();
      meta.updated_at = env.block.time;
      meta.updated_at_height = env.block.height.into();
      meta.updated_by = initiator.clone();
      Ok(meta)
    } else {
      Err(ContractError::UnexpectedError {
        reason: "dynamic contract metadata not found".to_owned(),
      })
    }
  })?;

  if let Some(prev_meta) = maybe_prev_meta {
    IX_REV.remove(storage, (partition, prev_meta.rev.into(), contract_id));
    IX_UPDATED_AT.remove(storage, (partition, prev_meta.updated_at.nanos(), contract_id));
  }

  IX_REV.save(storage, (partition, meta.rev.into(), contract_id), &X)?;
  IX_UPDATED_AT.save(storage, (partition, meta.updated_at.nanos(), contract_id), &X)?;
  IX_UPDATED_BY.save(storage, (partition, initiator.to_string(), contract_id), &X)?;

  Ok(())
}

fn update_tags(
  storage: &mut dyn Storage,
  partition: PartitionID,
  contract_id: ContractID,
  updates: TagUpdates,
) -> Result<(), ContractError> {
  for tag in updates.remove.iter() {
    IX_TAG.remove(storage, (partition, tag, contract_id));
    decrement_tag_count(storage, partition, tag)?;
  }

  for tag in updates.add.iter() {
    IX_TAG.save(storage, (partition, tag, contract_id), &X)?;
    increment_tag_count(storage, partition, tag)?;
  }

  Ok(())
}

fn update_indices(
  storage: &mut dyn Storage,
  partition: PartitionID,
  contract_id: ContractID,
  index_updates: Vec<KeyValue>,
) -> Result<(), ContractError> {
  // Update each index for the given KeyValue. If the given value is None, use
  // this as a signal to remove the existing entry, if any, from the index.
  for value in index_updates.iter() {
    match value {
      KeyValue::String(key, value) => update_string_index(storage, partition, contract_id, key, value)?,
      KeyValue::Bool(key, value) => update_bool_index(storage, partition, contract_id, key, value)?,
      KeyValue::Timestamp(key, value) => update_timestamp_index(storage, partition, contract_id, key, value)?,
      KeyValue::Uint8(key, value) => update_u8_index(storage, partition, contract_id, key, value)?,
      KeyValue::Uint16(key, value) => update_u16_index(storage, partition, contract_id, key, value)?,
      KeyValue::Uint32(key, value) => update_u32_index(storage, partition, contract_id, key, value)?,
      KeyValue::Uint64(key, value) => update_u64_index(storage, partition, contract_id, key, value)?,
      KeyValue::Uint128(key, value) => update_u128_index(storage, partition, contract_id, key, value)?,
    }
  }
  Ok(())
}

fn increment_index_size(
  storage: &mut dyn Storage,
  index_name: &String,
  is_positive: bool,
) -> Result<(), ContractError> {
  INDEX_METADATA.update(storage, index_name.clone(), |maybe_meta| -> Result<_, ContractError> {
    if let Some(mut meta) = maybe_meta {
      if is_positive {
        meta.size = meta
          .size
          .checked_add(Uint64::one())
          .map_err(|_| ContractError::UnexpectedError {
            reason: format!("Overflow incrementing index {} size", index_name),
          })?;
      } else {
        meta.size = meta
          .size
          .checked_sub(Uint64::one())
          .map_err(|_| ContractError::UnexpectedError {
            reason: format!("Overflow subtracting index {} size", index_name),
          })?;
      }
      Ok(meta)
    } else {
      Err(ContractError::UnexpectedError {
        reason: format!("Index {} not found", index_name),
      })
    }
  })?;
  Ok(())
}

fn update_string_index(
  storage: &mut dyn Storage,
  partition: PartitionID,
  contract_id: ContractID,
  index_name: &String,
  maybe_value: &Option<String>,
) -> Result<(), ContractError> {
  let index_slot = build_index_storage_key(index_name);
  let index: CustomIndexMap<&String> = Map::new(&index_slot);
  let indexed_value_map = VALUES_STRING;

  if let Some(new_val) = maybe_value {
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
    increment_index_size(storage, index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, index_name))? {
    let index_key = (partition, &old_val, contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, index_name));
      increment_index_size(storage, index_name, false)?;
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
      index.remove(storage, (partition, if old_val { 1 } else { 0 }, contract_id));
    }

    index.save(storage, index_key, &X)?;
    if !CONTRACT_INDEX_TYPES.has(storage, (contract_id, &index_name)) {
      CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Bool)?;
    }
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, if old_val { 1u8 } else { 0u8 }, contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
      CONTRACT_INDEX_TYPES.save(storage, (contract_id, &index_name), &IndexType::Timestamp)?;
    }
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val.nanos(), contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, (partition, old_val.nanos(), contract_id));
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val, contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val, contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val, contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val.u64(), contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
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
    increment_index_size(storage, &index_name, true)?;
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    let index_key = (partition, old_val.u128(), contract_id);
    if index.has(storage, index_key) {
      index.remove(storage, index_key);
      indexed_value_map.remove(storage, (contract_id, index_name));
      CONTRACT_INDEX_TYPES.remove(storage, (contract_id, &index_name));
      increment_index_size(storage, &index_name, false)?;
    }
  }
  Ok(())
}

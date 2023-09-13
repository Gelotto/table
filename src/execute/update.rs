use crate::{
  error::ContractError,
  msg::{IndexType, KeyValue, TagUpdates, UpdateParams},
  state::{
    load_contract_id, DYNAMIC_METADATA, INDEXED_KEYS, INDEX_REV, INDEX_TAG, INDEX_UPDATED_AT, INDEX_UPDATED_BY,
    METADATA, VALUES_BOOL, VALUES_STRING, VALUES_TIME, VALUES_U128, VALUES_U16, VALUES_U32, VALUES_U64, VALUES_U8, X,
  },
};
use cosmwasm_std::{attr, Addr, DepsMut, Env, MessageInfo, Response, Storage, Timestamp, Uint128, Uint64};
use cw_storage_plus::Map;

pub fn on_execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  params: UpdateParams,
) -> Result<Response, ContractError> {
  deps.api.addr_validate(params.initiator.as_str())?;

  let initiator = params.initiator;
  let contract_id = load_contract_id(deps.storage, &info.sender)?;
  let partition = METADATA.load(deps.storage, contract_id)?.partition;

  if let Some(index_updates) = params.values {
    update_indices(deps.storage, partition, contract_id, index_updates)?;
    update_metadata(deps.storage, &env, partition, &initiator, contract_id)?;
  }
  if let Some(tag_updates) = params.tags {
    update_tags(deps.storage, partition, contract_id, tag_updates)?;
  }

  Ok(Response::new().add_attributes(vec![attr("action", "update")]))
}

fn update_metadata(
  storage: &mut dyn Storage,
  env: &Env,
  partition: u16,
  initiator: &Addr,
  contract_id: u64,
) -> Result<(), ContractError> {
  let meta = DYNAMIC_METADATA.update(storage, contract_id, |maybe_meta| -> Result<_, ContractError> {
    if let Some(mut meta) = maybe_meta {
      meta.rev += Uint64::one();
      meta.time = env.block.time;
      meta.height = env.block.height.into();
      meta.initiator = initiator.clone();
      Ok(meta)
    } else {
      Err(ContractError::UnexpectedError {
        reason: "contract update metadata not found".to_owned(),
      })
    }
  })?;

  INDEX_REV.save(storage, (partition, meta.rev.into(), contract_id), &X)?;
  INDEX_UPDATED_AT.save(storage, (partition, meta.time.nanos(), contract_id), &X)?;
  INDEX_UPDATED_BY.save(storage, (partition, initiator.to_string(), contract_id), &X)?;

  Ok(())
}

fn update_tags(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  updates: TagUpdates,
) -> Result<(), ContractError> {
  for tag in updates.remove.iter() {
    INDEX_TAG.remove(storage, (partition, tag, contract_id));
  }
  for tag in updates.add.iter() {
    INDEX_TAG.save(storage, (partition, tag, contract_id), &X)?;
  }
  Ok(())
}

fn update_indices(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
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

fn update_string_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  index_name: &String,
  maybe_value: &Option<String>,
) -> Result<(), ContractError> {
  let index: Map<(u16, &String, u64), u8> = Map::new(index_name.as_str());
  let indexed_value_map = VALUES_STRING;

  if let Some(new_val) = maybe_value {
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
    if !INDEXED_KEYS.has(storage, (contract_id, &index_name)) {
      INDEXED_KEYS.save(storage, (contract_id, &index_name), &IndexType::String)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &index_name))? {
    index.remove(storage, (partition, &old_val, contract_id));
    indexed_value_map.remove(storage, (contract_id, index_name));
    INDEXED_KEYS.remove(storage, (contract_id, &index_name));
  }
  Ok(())
}

fn update_bool_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<bool>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u8, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_BOOL;
  let mut maybe_old_bool: Option<bool> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_bool = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_bool {
      index.remove(storage, (partition, if old_val { 1 } else { 0 }, contract_id));
    }
    index.save(storage, (partition, if *new_val { 1 } else { 0 }, contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Bool)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, if old_val { 1 } else { 0 }, contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_timestamp_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<Timestamp>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u64, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_TIME;

  if let Some(new_val) = maybe_value {
    let mut maybe_old_val: Option<Timestamp> = None;
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_val = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_val {
      index.remove(storage, (partition, old_val.nanos(), contract_id));
    }
    index.save(storage, (partition, new_val.nanos(), contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Timestamp)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val.nanos(), contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_u8_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<u8>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u8, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_U8;
  let mut maybe_old_val: Option<u8> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_val = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_val {
      index.remove(storage, (partition, old_val, contract_id));
    }
    index.save(storage, (partition, *new_val, contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Uint8)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val, contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_u16_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<u16>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u16, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_U16;
  let mut maybe_old_val: Option<u16> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_val = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_val {
      index.remove(storage, (partition, old_val, contract_id));
    }
    index.save(storage, (partition, *new_val, contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Uint16)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val, contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_u32_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<u32>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u32, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_U32;
  let mut maybe_old_bool: Option<u32> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_bool = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_bool {
      index.remove(storage, (partition, old_val, contract_id));
    }
    index.save(storage, (partition, *new_val, contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Uint32)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val, contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_u64_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<Uint64>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u64, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_U64;
  let mut maybe_old_val: Option<Uint64> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_val = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_val {
      index.remove(storage, (partition, old_val.into(), contract_id));
    }
    index.save(storage, (partition, (*new_val).into(), contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Uint64)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val.into(), contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

fn update_u128_index(
  storage: &mut dyn Storage,
  partition: u16,
  contract_id: u64,
  key: &String,
  maybe_value: &Option<Uint128>,
) -> Result<(), ContractError> {
  let index: Map<(u16, u128, u64), u8> = Map::new(key.as_str());
  let indexed_value_map = VALUES_U128;
  let mut maybe_old_val: Option<Uint128> = None;

  if let Some(new_val) = maybe_value {
    indexed_value_map.update(
      storage,
      (contract_id, &key.to_owned()),
      |x| -> Result<_, ContractError> {
        maybe_old_val = x;
        Ok(*new_val)
      },
    )?;
    if let Some(old_val) = maybe_old_val {
      index.remove(storage, (partition, old_val.into(), contract_id));
    }
    index.save(storage, (partition, (*new_val).into(), contract_id), &X)?;
    if !INDEXED_KEYS.has(storage, (contract_id, &key)) {
      INDEXED_KEYS.save(storage, (contract_id, &key), &IndexType::Uint128)?;
    }
  } else if let Some(old_val) = indexed_value_map.may_load(storage, (contract_id, &key))? {
    index.remove(storage, (partition, old_val.into(), contract_id));
    indexed_value_map.remove(storage, (contract_id, key));
    INDEXED_KEYS.remove(storage, (contract_id, &key));
  }
  Ok(())
}

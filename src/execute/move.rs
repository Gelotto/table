use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Order, Response, Storage};
use cw_storage_plus::Map;

use crate::{
  error::ContractError,
  msg::IndexType,
  state::{
    load_contract_id, DYNAMIC_METADATA, INDEXED_KEYS, INDEX_CODE_ID, INDEX_CREATED_AT, INDEX_CREATED_BY, INDEX_REV,
    INDEX_UPDATED_AT, INDEX_UPDATED_BY, METADATA, VALUES_BOOL, VALUES_STRING, VALUES_TIME, VALUES_U128, VALUES_U16,
    VALUES_U32, VALUES_U64, VALUES_U8, X,
  },
};

/// Move the contract to a new partition.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  contract_addr: Addr,
  dst_partition: u16,
) -> Result<Response, ContractError> {
  let contract_id = load_contract_id(deps.storage, &contract_addr)?;
  let meta = METADATA.load(deps.storage, contract_id)?;

  if meta.partition != dst_partition {
    let src_partition = meta.partition;
    update_contract_partition(deps.storage, contract_id, src_partition, dst_partition)?;
  }

  Ok(Response::new().add_attribute("action", "move"))
}

pub fn update_contract_partition(
  storage: &mut dyn Storage,
  contract_id: u64,
  src: u16,
  dst: u16,
) -> Result<(), ContractError> {
  update_builtin_indices_partition(storage, contract_id, src, dst)?;
  update_custom_indices_partition(storage, contract_id, src, dst)?;
  Ok(())
}

fn update_builtin_indices_partition(
  storage: &mut dyn Storage,
  contract_id: u64,
  src: u16,
  dst: u16,
) -> Result<(), ContractError> {
  // Update creation meta indices
  let creation_meta = METADATA.load(storage, contract_id)?;

  INDEX_CODE_ID.remove(storage, (src, creation_meta.code_id.into(), contract_id));
  INDEX_CODE_ID.save(storage, (dst, creation_meta.code_id.into(), contract_id), &X)?;

  INDEX_CREATED_BY.remove(storage, (src, creation_meta.initiator.to_string(), contract_id));
  INDEX_CREATED_BY.save(storage, (dst, creation_meta.initiator.to_string(), contract_id), &X)?;

  INDEX_CREATED_AT.remove(storage, (src, creation_meta.time.nanos(), contract_id));
  INDEX_CREATED_AT.save(storage, (dst, creation_meta.time.nanos(), contract_id), &X)?;

  // Update dynamic meta indices
  let dynamic_meta = DYNAMIC_METADATA.load(storage, contract_id)?;

  INDEX_UPDATED_BY.remove(storage, (src, dynamic_meta.initiator.to_string(), contract_id));
  INDEX_UPDATED_BY.save(storage, (dst, dynamic_meta.initiator.to_string(), contract_id), &X)?;

  INDEX_UPDATED_AT.remove(storage, (src, dynamic_meta.time.nanos(), contract_id));
  INDEX_UPDATED_AT.save(storage, (dst, dynamic_meta.time.nanos(), contract_id), &X)?;

  INDEX_REV.remove(storage, (src, dynamic_meta.rev.into(), contract_id));
  INDEX_REV.save(storage, (dst, dynamic_meta.rev.into(), contract_id), &X)?;

  Ok(())
}

pub fn update_custom_indices_partition(
  storage: &mut dyn Storage,
  contract_id: u64,
  src: u16,
  dst: u16,
) -> Result<(), ContractError> {
  let entries: Vec<(String, IndexType)> = INDEXED_KEYS
    .prefix(contract_id)
    .range(storage, None, None, Order::Ascending)
    .filter_map(|r| r.ok())
    .collect();

  for (index_name, index_type) in entries.iter() {
    match index_type {
      IndexType::String => {
        let index: Map<(u16, &String, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_STRING.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, &value, contract_id));
        index.save(storage, (dst, &value, contract_id), &X)?;
      },
      IndexType::Bool => {
        let index: Map<(u16, u8, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_BOOL.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Timestamp => {
        let index: Map<(u16, u64, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_TIME.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.nanos(), contract_id));
        index.save(storage, (dst, value.nanos(), contract_id), &X)?;
      },
      IndexType::Uint8 => {
        let index: Map<(u16, u8, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_U8.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint16 => {
        let index: Map<(u16, u16, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_U16.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint32 => {
        let index: Map<(u16, u32, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_U32.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint64 => {
        let index: Map<(u16, u64, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_U64.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint128 => {
        let index: Map<(u16, u128, u64), u8> = Map::new(index_name.as_str());
        let value = VALUES_U128.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
    }
  }

  Ok(())
}

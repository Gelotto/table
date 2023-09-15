use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Order, Response, StdResult, Storage, Uint64};
use cw_storage_plus::Map;

use crate::{
  error::ContractError,
  msg::{IndexType, PartitionSelector},
  state::{
    decrement_tag_count, ensure_contract_not_suspended, ensure_is_authorized_owner, ensure_partition_exists,
    increment_tag_count, load_contract_id, load_partition_id_from_selector, CONTRACT_DYN_METADATA,
    CONTRACT_INDEXED_KEYS, CONTRACT_METADATA, CONTRACT_TAGS, IX_CODE_ID, IX_CREATED_AT, IX_CREATED_BY, IX_REV, IX_TAG,
    IX_UPDATED_AT, IX_UPDATED_BY, PARTITION_SIZES, VALUES_BOOL, VALUES_STRING, VALUES_TIME, VALUES_U128, VALUES_U16,
    VALUES_U32, VALUES_U64, VALUES_U8, X,
  },
  util::build_index_name,
};

/// Move the contract to a new partition.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  contract_addr: Addr,
  dst_selector: PartitionSelector,
) -> Result<Response, ContractError> {
  let action = "move";
  deps.api.addr_validate(contract_addr.as_str())?;

  ensure_contract_not_suspended(deps.storage, &contract_addr)?;

  let dst_partition = load_partition_id_from_selector(deps.storage, dst_selector)?;

  ensure_partition_exists(deps.storage, dst_partition)?;

  // If sender isn't the contract itself, only allow sender if auth'd by owner
  // address or ACL.
  if contract_addr != info.sender {
    ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;
  };

  let contract_id = load_contract_id(deps.storage, &contract_addr)?;
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
  contract_id: u64,
  src: u16,
  dst: u16,
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
  contract_id: u64,
  src: u16,
  dst: u16,
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
      .map_err(|e| ContractError::UnexpectedError { reason: e.to_string() })
  })?;

  Ok(())
}

fn move_standard_indices(
  storage: &mut dyn Storage,
  contract_id: u64,
  src: u16,
  dst: u16,
) -> Result<(), ContractError> {
  // Update core metadata indices
  let meta = CONTRACT_METADATA.load(storage, contract_id)?;

  IX_CODE_ID.remove(storage, (src, meta.code_id.into(), contract_id));
  IX_CODE_ID.save(storage, (dst, meta.code_id.into(), contract_id), &X)?;

  IX_CREATED_BY.remove(storage, (src, meta.created_by.to_string(), contract_id));
  IX_CREATED_BY.save(storage, (dst, meta.created_by.to_string(), contract_id), &X)?;

  IX_CREATED_AT.remove(storage, (src, meta.created_at.nanos(), contract_id));
  IX_CREATED_AT.save(storage, (dst, meta.created_at.nanos(), contract_id), &X)?;

  // Update "update" metadata indices
  let up_meta = CONTRACT_DYN_METADATA.load(storage, contract_id)?;

  IX_UPDATED_BY.remove(storage, (src, up_meta.updated_by.to_string(), contract_id));
  IX_UPDATED_BY.save(storage, (dst, up_meta.updated_by.to_string(), contract_id), &X)?;

  IX_UPDATED_AT.remove(storage, (src, up_meta.updated_at.nanos(), contract_id));
  IX_UPDATED_AT.save(storage, (dst, up_meta.updated_at.nanos(), contract_id), &X)?;

  IX_REV.remove(storage, (src, up_meta.rev.into(), contract_id));
  IX_REV.save(storage, (dst, up_meta.rev.into(), contract_id), &X)?;

  Ok(())
}

pub fn move_custom_indices(
  storage: &mut dyn Storage,
  contract_id: u64,
  src: u16,
  dst: u16,
) -> Result<(), ContractError> {
  let entries: Vec<(String, IndexType)> = CONTRACT_INDEXED_KEYS
    .prefix(contract_id)
    .range(storage, None, None, Order::Ascending)
    .filter_map(|r| r.ok())
    .collect();

  for (base_index_name, index_type) in entries.iter() {
    let index_name = build_index_name(base_index_name);
    match index_type {
      IndexType::String => {
        let index: Map<(u16, &String, u64), u8> = Map::new(&index_name);
        let value = VALUES_STRING.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, &value, contract_id));
        index.save(storage, (dst, &value, contract_id), &X)?;
      },
      IndexType::Bool => {
        let index: Map<(u16, u8, u64), u8> = Map::new(&index_name);
        let value = VALUES_BOOL.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Timestamp => {
        let index: Map<(u16, u64, u64), u8> = Map::new(&index_name);
        let value = VALUES_TIME.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.nanos(), contract_id));
        index.save(storage, (dst, value.nanos(), contract_id), &X)?;
      },
      IndexType::Uint8 => {
        let index: Map<(u16, u8, u64), u8> = Map::new(&index_name);
        let value = VALUES_U8.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint16 => {
        let index: Map<(u16, u16, u64), u8> = Map::new(&index_name);
        let value = VALUES_U16.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint32 => {
        let index: Map<(u16, u32, u64), u8> = Map::new(&index_name);
        let value = VALUES_U32.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint64 => {
        let index: Map<(u16, u64, u64), u8> = Map::new(&index_name);
        let value = VALUES_U64.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
      IndexType::Uint128 => {
        let index: Map<(u16, u128, u64), u8> = Map::new(&index_name);
        let value = VALUES_U128.load(storage, (contract_id, &index_name))?;
        index.remove(storage, (src, value.into(), contract_id));
        index.save(storage, (dst, value.into(), contract_id), &X)?;
      },
    }
  }

  Ok(())
}

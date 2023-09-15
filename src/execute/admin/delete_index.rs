use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw_storage_plus::Map;

use crate::{
  error::ContractError,
  msg::IndexType,
  state::{ensure_is_authorized_owner, ContractID, PartitionID, INDEX_METADATA},
  util::build_index_name,
};

pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  index_name: String,
) -> Result<Response, ContractError> {
  let action = "delete_index";

  ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;

  if let Some(meta) = INDEX_METADATA.may_load(deps.storage, index_name.clone())? {
    INDEX_METADATA.remove(deps.storage, index_name.clone());

    let map_name = &build_index_name(&index_name);

    match meta.index_type {
      IndexType::String => {
        let map: Map<(PartitionID, &String, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Bool => {
        let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Timestamp => {
        let map: Map<(PartitionID, u64, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Uint8 => {
        let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Uint16 => {
        let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Uint32 => {
        let map: Map<(PartitionID, u32, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Uint64 => {
        let map: Map<(PartitionID, u64, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
      IndexType::Uint128 => {
        let map: Map<(PartitionID, u128, ContractID), u8> = Map::new(map_name);
        map.clear(deps.storage);
      },
    };
  } else {
    return Err(ContractError::NotAuthorized {
      reason: format!("index metadata does not exist for '{}'", index_name),
    });
  }
  Ok(Response::new().add_attribute("action", "delete_index"))
}

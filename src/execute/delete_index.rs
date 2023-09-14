use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw_storage_plus::Map;

use crate::{
  error::ContractError,
  msg::IndexType,
  state::{ensure_owner_auth, ContractID, PartitionID, INDEX_METADATA},
};

pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  index_name: String,
) -> Result<Response, ContractError> {
  let action = "delete_index";

  ensure_owner_auth(deps.storage, deps.querier, &info.sender, action)?;

  if let Some(meta) = INDEX_METADATA.may_load(deps.storage, index_name.clone())? {
    INDEX_METADATA.remove(deps.storage, index_name.clone());

    let map_label = &format!("_ix_{}", index_name);

    match meta.index_type {
      IndexType::String => {
        let map: Map<(PartitionID, &String, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Bool => {
        let map: Map<(PartitionID, u8, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Timestamp => {
        let map: Map<(PartitionID, u64, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Uint8 => {
        let map: Map<(PartitionID, u8, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Uint16 => {
        let map: Map<(PartitionID, u8, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Uint32 => {
        let map: Map<(PartitionID, u32, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Uint64 => {
        let map: Map<(PartitionID, u64, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
      IndexType::Uint128 => {
        let map: Map<(PartitionID, u128, ContractID), bool> = Map::new(map_label);
        map.clear(deps.storage);
      },
    }
  } else {
    return Err(ContractError::NotAuthorized {
      reason: format!("index metadata does not exist for '{}'", index_name),
    });
  }
  Ok(Response::new().add_attribute("action", "delete_index"))
}

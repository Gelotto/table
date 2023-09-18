use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  msg::GroupUpdates,
  state::{append_group, ensure_sender_is_owner, load_contract_id, remove_from_group, resolve_group_id},
};

pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  updates: GroupUpdates,
) -> Result<Response, ContractError> {
  let action = "update_groups";

  ensure_sender_is_owner(deps.storage, deps.querier, &info.sender, action)?;

  let contract_addr = updates.contract;
  let contract_id = load_contract_id(deps.storage, &contract_addr)?;

  // Remove contract from the given groups.
  if let Some(selectors) = updates.remove {
    for s in selectors.iter() {
      let group_id = resolve_group_id(deps.storage, s.clone())?;
      remove_from_group(deps.storage, group_id, contract_id)?;
    }
  }

  // Add contract to the given groups.
  if let Some(selectors) = updates.add {
    for s in selectors.iter() {
      let group_id = resolve_group_id(deps.storage, s.clone())?;
      append_group(deps.storage, group_id, contract_id)?;
    }
  }

  Ok(Response::new().add_attribute("action", action))
}

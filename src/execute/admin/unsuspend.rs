use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  state::{ensure_sender_is_owner, CONTRACT_ADDR_2_ID, CONTRACT_SUSPENSIONS},
};

/// Load and restore the previous config, provided there is one.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  contract_addr: Addr,
) -> Result<Response, ContractError> {
  let action = "unsuspend";

  // Only owner authority can un-suspend a contract
  ensure_sender_is_owner(deps.storage, deps.querier, &info.sender, action)?;
  if let Some(id) = CONTRACT_ADDR_2_ID.may_load(deps.storage, &contract_addr)? {
    CONTRACT_SUSPENSIONS.remove(deps.storage, id.into());
  }

  Ok(Response::new().add_attribute("action", action))
}

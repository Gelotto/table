use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  state::{ensure_is_authorized_owner, CONTRACT_SUSPENSIONS},
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
  ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;

  CONTRACT_SUSPENSIONS.remove(deps.storage, &contract_addr);

  Ok(Response::new().add_attribute("action", action))
}

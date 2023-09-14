use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  state::{ensure_owner_auth, CONTRACT_SUSPENSIONS},
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
  ensure_owner_auth(deps.storage, deps.querier, &info.sender, action)?;

  CONTRACT_SUSPENSIONS.remove(deps.storage, &contract_addr);

  Ok(Response::new().add_attribute("action", action))
}

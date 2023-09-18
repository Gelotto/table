use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  msg::TableInfo,
  state::{ensure_sender_allowed, TABLE_INFO},
};

// Replace the existing config info object.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  table_info: TableInfo,
) -> Result<Response, ContractError> {
  let action = "update_info";

  ensure_sender_allowed(deps.storage, deps.querier, &info.sender, action)?;

  TABLE_INFO.save(deps.storage, &table_info)?;

  Ok(Response::new().add_attribute("action", action))
}

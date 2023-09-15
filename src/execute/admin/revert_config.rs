use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  msg::Config,
  state::{save_config, CONFIG_BACKUP},
};

/// Load and restore the previous config, provided there is one.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
) -> Result<Response, ContractError> {
  if let Some(prev_config_binary) = CONFIG_BACKUP.may_load(deps.storage)? {
    let prev_config: Config = from_binary(&prev_config_binary)?;
    save_config(deps.storage, &prev_config)?;
  }

  Ok(Response::new().add_attribute("action", "revert_config"))
}

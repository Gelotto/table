use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{error::ContractError, msg::Config, state::save_config};

// Replace the existing config in its entirety.
pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  config: Config,
) -> Result<Response, ContractError> {
  config.validate(deps.api)?;
  save_config(deps.storage, &config)?;
  Ok(Response::new().add_attribute("action", "config"))
}

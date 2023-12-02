use cosmwasm_std::Response;

use crate::{context::Context, error::ContractError, msg::Config, state::save_config};

// Replace the existing config in its entirety.
pub fn on_execute(
    ctx: Context,
    config: Config,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;
    config.validate(deps.api)?;
    save_config(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

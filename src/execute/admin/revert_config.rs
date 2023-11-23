use cosmwasm_std::{from_binary, Response};

use crate::{
    error::ContractError,
    execute::Context,
    msg::Config,
    state::{ensure_sender_allowed, save_config, CONFIG_BACKUP},
};

/// Load and restore the previous config, provided there is one.
pub fn on_execute(ctx: Context) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    ensure_sender_allowed(&deps, &info.sender, "/table/revert-config")?;
    if let Some(prev_config_binary) = CONFIG_BACKUP.may_load(deps.storage)? {
        let prev_config: Config = from_binary(&prev_config_binary)?;
        save_config(deps.storage, &prev_config)?;
    }

    Ok(Response::new().add_attribute("action", "revert_config"))
}

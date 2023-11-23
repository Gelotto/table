use cosmwasm_std::{Addr, Response};

use crate::{
    error::ContractError,
    execute::Context,
    state::{ensure_sender_allowed, CONTRACT_ADDR_2_ID, CONTRACT_SUSPENSIONS},
};

/// Load and restore the previous config, provided there is one.
pub fn on_execute(
    ctx: Context,
    contract_addr: Addr,
) -> Result<Response, ContractError> {
    let action = "unsuspend";
    let Context { deps, info, .. } = ctx;

    // Only owner authority can un-suspend a contract
    ensure_sender_allowed(&deps, &info.sender, "/table/unsuspend")?;
    if let Some(id) = CONTRACT_ADDR_2_ID.may_load(deps.storage, &contract_addr)? {
        CONTRACT_SUSPENSIONS.remove(deps.storage, id.into());
    }

    Ok(Response::new().add_attribute("action", action))
}

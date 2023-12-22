use cosmwasm_std::Response;
use cw_acl::state::OWNER;
use cw_lib::models::Owner;

use crate::{context::Context, error::ContractError, state::ensure_allowed_by_acl};

// Replace the existing config info object.
pub fn on_execute(
    ctx: Context,
    owner: Owner,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    let action = "set_owner";

    ensure_allowed_by_acl(&deps, &info.sender, "/table/set-owner")?;
    deps.api.addr_validate(owner.to_addr().as_str())?;

    OWNER.save(deps.storage, &owner)?;

    Ok(Response::new().add_attribute("action", action))
}

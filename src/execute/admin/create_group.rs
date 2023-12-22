use cosmwasm_std::Response;

use crate::{
    context::Context,
    error::ContractError,
    msg::GroupCreationParams,
    state::{create_group, ensure_allowed_by_acl},
};

pub fn on_execute(
    ctx: Context,
    params: GroupCreationParams,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let action = "create_groups";

    ensure_allowed_by_acl(&deps, &info.sender, "/table/create-groups")?;
    create_group(deps.storage, params, &info, &env)?;

    Ok(Response::new().add_attribute("action", action))
}

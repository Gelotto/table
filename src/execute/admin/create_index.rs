use cosmwasm_std::Response;

use crate::{
    context::Context,
    error::ContractError,
    msg::IndexCreationParams,
    state::{create_index, ensure_allowed_by_acl},
};

pub fn on_execute(
    ctx: Context,
    params: IndexCreationParams,
) -> Result<Response, ContractError> {
    let action = "create_index";
    let Context { deps, info, .. } = ctx;

    ensure_allowed_by_acl(&deps, &info.sender, "/table/create-index")?;
    create_index(deps.storage, params)?;

    Ok(Response::new().add_attribute("action", action))
}

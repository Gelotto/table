use cosmwasm_std::Response;

use crate::{
    context::Context,
    error::ContractError,
    msg::TableInfo,
    state::{ensure_allowed_by_acl, TABLE_INFO},
};

// Replace the existing config info object.
pub fn on_execute(
    ctx: Context,
    table_info: TableInfo,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    let action = "update_info";

    ensure_allowed_by_acl(&deps, &info.sender, "/table/update-info")?;

    TABLE_INFO.save(deps.storage, &table_info)?;

    Ok(Response::new().add_attribute("action", action))
}

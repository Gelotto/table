use cosmwasm_std::Response;

use crate::{
    context::Context,
    error::ContractError,
    msg::PartitionCreationParams,
    state::{create_partition, ensure_allowed_by_acl},
};

pub fn on_execute(
    ctx: Context,
    params: PartitionCreationParams,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let action = "create_partition";

    ensure_allowed_by_acl(&deps, &info.sender, "/table/create-partition")?;

    create_partition(deps.storage, env.block.time, &params)?;

    Ok(Response::new().add_attribute("action", action))
}

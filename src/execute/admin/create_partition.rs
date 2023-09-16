use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
  error::ContractError,
  msg::PartitionCreationParams,
  state::{create_partition, ensure_is_authorized_owner},
};

pub fn on_execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  params: PartitionCreationParams,
) -> Result<Response, ContractError> {
  let action = "create_partition";

  ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;
  create_partition(deps.storage, env.block.time, &params)?;

  Ok(Response::new().add_attribute("action", action))
}

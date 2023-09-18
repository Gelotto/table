use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Uint64};

use crate::{
  error::ContractError,
  msg::{GroupCreationParams, GroupMetadata},
  state::{ensure_sender_is_owner, GroupID, GROUP_ID_COUNTER, GROUP_METADATA, GROUP_NAME_2_ID},
};

pub fn on_execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  params: GroupCreationParams,
) -> Result<Response, ContractError> {
  let action = "create_group";

  ensure_sender_is_owner(deps.storage, deps.querier, &info.sender, action)?;

  let group_id = increment_next_group_id(deps.storage)?;
  let name = params.name.unwrap_or_else(|| group_id.to_string());

  // Save id into name -> ID lookup table.
  GROUP_NAME_2_ID.save(deps.storage, name.clone(), &group_id)?;

  GROUP_METADATA.update(deps.storage, group_id, |maybe_meta| -> Result<_, ContractError> {
    if maybe_meta.is_some() {
      Err(ContractError::NotAuthorized {
        reason: format!("index {} already exists", name),
      })
    } else {
      Ok(GroupMetadata {
        description: params.description,
        created_at: env.block.time,
        size: Uint64::zero(),
        name,
      })
    }
  })?;

  Ok(Response::new().add_attribute("action", action))
}

fn increment_next_group_id(storage: &mut dyn Storage) -> Result<GroupID, ContractError> {
  GROUP_ID_COUNTER.update(storage, |n| -> Result<_, ContractError> {
    n.checked_add(1).ok_or_else(|| ContractError::UnexpectedError {
      reason: "unexpected overflow incrementing group ID counter".to_owned(),
    })
  })
}

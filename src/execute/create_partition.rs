use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};

use crate::{
  error::ContractError,
  msg::{PartitionCreationParams, PartitionMetadata},
  state::{ensure_is_authorized_owner, PartitionID, PARTITION_ID_COUNTER, PARTITION_METADATA, PARTITION_NAME_2_ID},
};

pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  params: PartitionCreationParams,
) -> Result<Response, ContractError> {
  let action = "create_partition";

  ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;

  let partition_id = increment_next_partition_id(deps.storage)?;
  let name = params.name.unwrap_or_else(|| partition_id.to_string());

  // Save id into name -> ID lookup table.
  PARTITION_NAME_2_ID.save(deps.storage, name.clone(), &partition_id)?;

  // Init partition metadata state.
  PARTITION_METADATA.update(deps.storage, partition_id, |maybe_meta| -> Result<_, ContractError> {
    if maybe_meta.is_some() {
      Err(ContractError::NotAuthorized {
        reason: format!("partition {} already exists", partition_id),
      })
    } else {
      Ok(PartitionMetadata {
        description: params.description,
        name,
      })
    }
  })?;

  Ok(Response::new().add_attribute("action", action))
}

fn increment_next_partition_id(storage: &mut dyn Storage) -> Result<PartitionID, ContractError> {
  PARTITION_ID_COUNTER.update(storage, |n| -> Result<_, ContractError> {
    n.checked_add(1).ok_or_else(|| ContractError::UnexpectedError {
      reason: "unexpected overflow incrementing partition ID counter".to_owned(),
    })
  })
}

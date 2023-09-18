use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint64};

use crate::{
  error::ContractError,
  msg::{IndexCreationParams, IndexMetadata},
  state::{ensure_sender_allowed, INDEX_METADATA},
};

pub fn on_execute(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  params: IndexCreationParams,
) -> Result<Response, ContractError> {
  let action = "create_index";

  ensure_sender_allowed(deps.storage, deps.querier, &info.sender, action)?;

  INDEX_METADATA.update(
    deps.storage,
    params.name.clone(),
    |maybe_meta| -> Result<_, ContractError> {
      if maybe_meta.is_some() {
        Err(ContractError::NotAuthorized {
          reason: format!("index {} already exists", params.name),
        })
      } else {
        Ok(IndexMetadata {
          size: Uint64::zero(),
          index_type: params.index_type,
          name: params.name,
        })
      }
    },
  )?;

  Ok(Response::new().add_attribute("action", action))
}

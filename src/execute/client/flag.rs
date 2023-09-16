use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Response};
use cw_storage_plus::Deque;

use crate::{
  error::ContractError,
  models::ContractFlag,
  msg::FlagParams,
  state::{ensure_is_authorized_owner, load_contract_id, CONTRACT_SUSPENSIONS},
};

pub fn on_execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  params: FlagParams,
) -> Result<Response, ContractError> {
  let action = "flag";

  let contract_addr = deps.api.addr_validate(params.contract.as_str())?;

  // If sender isn't the contract itself, only allow sender if auth'd by owner
  // address or ACL.
  if contract_addr != info.sender {
    ensure_is_authorized_owner(deps.storage, deps.querier, &info.sender, action)?;
  };

  let contract_id = load_contract_id(deps.storage, &contract_addr)?;
  let flags_deque_key = format!("_flags_{}", contract_id);
  let flags: Deque<ContractFlag> = Deque::new(flags_deque_key.as_str());

  if params.suspend.unwrap_or(false) {
    CONTRACT_SUSPENSIONS.save(deps.storage, &contract_addr, &true)?;
  }

  flags.push_back(
    deps.storage,
    &ContractFlag {
      sender: info.sender,
      height: env.block.height.into(),
      time: env.block.time,
      reason: params.reason,
      code: params.code,
    },
  )?;

  Ok(Response::new().add_attributes(vec![attr("action", action)]))
}
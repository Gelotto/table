use crate::error::ContractError;
use crate::execute;
use crate::models::ReplyJob;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::query;
use crate::state::{self, load_reply_job};
use cosmwasm_std::{entry_point, Reply};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:cw-table";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  state::initialize(deps, &env, &info, &msg)?;
  Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    ExecuteMsg::Create(params) => execute::create::on_execute(deps, env, info, params),
    ExecuteMsg::Update(params) => execute::update::on_execute(deps, env, info, params),
    ExecuteMsg::Move(addr, partition) => execute::r#move::on_execute(deps, env, info, addr, partition),
    ExecuteMsg::Sudo(msg) => match msg {
      SudoMsg::Config(config) => execute::config::on_execute(deps, env, info, config),
      SudoMsg::Revert() => execute::revert::on_execute(deps, env, info),
    },
  }
}

#[entry_point]
pub fn reply(
  deps: DepsMut,
  env: Env,
  reply: Reply,
) -> Result<Response, ContractError> {
  let job = load_reply_job(deps.storage, reply.id)?;
  return Ok(match job {
    ReplyJob::Create { params, initiator } => execute::create::on_reply(deps, env, reply, params, initiator),
  }?);
}

#[entry_point]
pub fn query(
  deps: Deps,
  _env: Env,
  msg: QueryMsg,
) -> Result<Binary, ContractError> {
  let result = match msg {
    QueryMsg::Info { fields, account } => to_binary(&query::select(deps, fields, account)?),
    QueryMsg::Read(queries) => to_binary(&query::read(deps, queries)?),
  }?;
  Ok(result)
}

#[entry_point]
pub fn migrate(
  deps: DepsMut,
  _env: Env,
  _msg: MigrateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  Ok(Response::default())
}

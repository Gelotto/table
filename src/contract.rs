use crate::error::ContractError;
use crate::execute;
use crate::models::ReplyJob;
use crate::msg::{AdminMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReadMsg};
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
    ExecuteMsg::Delete(addr) => execute::delete::on_execute(deps, env, info, addr),
    ExecuteMsg::Flag(params) => execute::flag::on_execute(deps, env, info, params),
    ExecuteMsg::Admin(msg) => match msg {
      AdminMsg::UpdateConfig(config) => execute::admin::update_config::on_execute(deps, env, info, config),
      AdminMsg::UpdateInfo(table_info) => execute::admin::update_info::on_execute(deps, env, info, table_info),
      AdminMsg::RevertConfig() => execute::admin::revert_config::on_execute(deps, env, info),
      AdminMsg::Unsuspend(addr) => execute::admin::unsuspend::on_execute(deps, env, info, addr),
      AdminMsg::CreatePartition(params) => execute::admin::create_partition::on_execute(deps, env, info, params),
      AdminMsg::CreateIndex(params) => execute::admin::create_index::on_execute(deps, env, info, params),
      AdminMsg::DeleteIndex(name) => execute::admin::delete_index::on_execute(deps, env, info, name),
      AdminMsg::Partition(addr, partition) => execute::admin::partition::on_execute(deps, env, info, addr, partition),
      AdminMsg::Group(addr, updates) => execute::admin::group::on_execute(deps, env, info, addr, updates),
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
    QueryMsg::Indices() => to_binary(&query::indices(deps)?),
    QueryMsg::Partition(selector) => to_binary(&query::partition(deps, selector)?),
    QueryMsg::Read(msg) => match msg {
      ReadMsg::Index(params) => to_binary(&query::read::index(deps, params)?),
      ReadMsg::Tags(params) => to_binary(&query::read::tags(deps, params)?),
      ReadMsg::Relationships(params) => to_binary(&query::read::relationships(deps, params)?),
    },
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

use crate::context::Context;
use crate::error::ContractError;
use crate::execute;
use crate::models::ReplyJob;
use crate::msg::{
    AdminMsg, ClientMsg, ContractQueryMsg, ContractsQueryMsg, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg, TableQueryMsg,
};
use crate::query;
use crate::state::{self, load_reply_job};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
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
    let ctx = Context { deps, env, info };
    set_contract_version(ctx.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    state::initialize(ctx, msg)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = Context { deps, env, info };
    match msg {
        // Client functions - executable by any smart contract "in" that exists in
        // this "table" OR by accounts with owner auth:
        ExecuteMsg::Client(msg) => match msg {
            ClientMsg::Create(params) => execute::client::create::on_execute(ctx, params),
            ClientMsg::Update(params) => execute::client::update::on_execute(ctx, params),
            ClientMsg::Delete(addr) => execute::client::delete::on_execute(ctx, addr),
            ClientMsg::Flag(params) => execute::client::flag::on_execute(ctx, params),
        },
        // Admin functions - require "owner" auth:
        ExecuteMsg::Admin(msg) => match msg {
            AdminMsg::SetOwner(owner) => execute::admin::set_owner::on_execute(ctx, owner),
            AdminMsg::UpdateInfo(info) => execute::admin::update_info::on_execute(ctx, info),
            AdminMsg::Unsuspend(addr) => execute::admin::unsuspend::on_execute(ctx, addr),

            // Config operations
            AdminMsg::UpdateConfig(config) => {
                execute::admin::update_config::on_execute(ctx, config)
            },
            AdminMsg::RevertConfig() => execute::admin::revert_config::on_execute(ctx),

            // Index operations
            AdminMsg::CreateIndex(params) => execute::admin::create_index::on_execute(ctx, params),
            AdminMsg::DeleteIndex(name) => execute::admin::delete_index::on_execute(ctx, name),

            // Partition operations
            AdminMsg::CreatePartition(params) => {
                execute::admin::create_partition::on_execute(ctx, params)
            },
            AdminMsg::SetPartition(addr, partition) => {
                execute::admin::set_partition::on_execute(ctx, addr, partition)
            },
            // Group operations
            AdminMsg::CreateGroup(params) => execute::admin::create_group::on_execute(ctx, params),
            AdminMsg::AssignGroups(updates) => {
                execute::admin::assign_groups::on_execute(ctx, updates)
            },
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
        ReplyJob::Create { params, initiator } => {
            execute::client::create::on_reply(deps, env, reply, params, initiator)
        },
    }?);
}

#[entry_point]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    let result = match msg {
        // Paginate top-level data structures related to the table.
        QueryMsg::Table(msg) => match msg {
            TableQueryMsg::Indices(params) => to_json_binary(&query::table::indices(deps, params)?),
            TableQueryMsg::Partitions(params) => {
                to_json_binary(&query::table::partitions(deps, params)?)
            },
            TableQueryMsg::Tags(params) => to_json_binary(&query::table::tags(deps, params)?),
            TableQueryMsg::Groups(params) => to_json_binary(&query::table::groups(deps, params)?),
        },
        // Paginate collections of contracts by various means.
        QueryMsg::Contracts(msg) => match msg {
            ContractsQueryMsg::Range(params) => {
                to_json_binary(&query::contracts::range(deps, params)?)
            },
            ContractsQueryMsg::WithTag(params) => {
                to_json_binary(&query::contracts::with_tag(deps, params)?)
            },
            ContractsQueryMsg::InGroup(params) => {
                to_json_binary(&query::contracts::in_group(deps, params)?)
            },
            ContractsQueryMsg::ByAddresses(mut params) => {
                to_json_binary(&query::contracts::by_addresses(deps, &mut params)?)
            },
            ContractsQueryMsg::RelatedTo(params) => {
                to_json_binary(&query::contracts::related_to(deps, params)?)
            },
        },
        // Paginate relationshps, groups, & tags associated with a given contract.
        QueryMsg::Contract(msg) => match msg {
            ContractQueryMsg::Relationships(params) => {
                to_json_binary(&query::contract::relationships(deps, params)?)
            },
            ContractQueryMsg::Groups(params) => {
                to_json_binary(&query::contract::groups(deps, params)?)
            },
            ContractQueryMsg::Tags(params) => to_json_binary(&query::contract::tags(deps, params)?),
            ContractQueryMsg::IsRelatedTo(params) => {
                to_json_binary(&query::contract::is_related_to(deps, params)?)
            },
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

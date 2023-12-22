use crate::context::Context;
use crate::error::ContractError;
use crate::execute;
use crate::models::ReplyJob;
use crate::msg::{
    AdminMsg, ClientMsg, ContractQueryMsg, ContractsQueryMsg, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg, TableQueryMsg,
};
use crate::query;
use crate::state::{
    self, load_reply_job, CustomIndexMap, CONFIG_STR_CASE_SENSITIVE, CONFIG_STR_MAX_LEN,
    CONTRACT_ID_2_ADDR, CONTRACT_TAGS, CONTRACT_USES_LIFECYCLE_HOOKS, IX_TAG, PARTITION_TAG_COUNTS,
    REL_ADDR_2_ID, REL_ID_2_ADDR, X,
};
use crate::util::{build_index_storage_key, pad};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response,
};
use cw2::set_contract_version;
use cw_storage_plus::Map;

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
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg {
        MigrateMsg::V0_0_3 {
            string_indices,
            use_lifecycle_hooks,
        } => {
            // Set all existing contract's lifecycle toggles to true
            if use_lifecycle_hooks {
                for id in CONTRACT_ID_2_ADDR
                    .keys(deps.storage, None, None, Order::Ascending)
                    .map(|k| k.unwrap())
                    .collect::<Vec<u64>>()
                {
                    CONTRACT_USES_LIFECYCLE_HOOKS.save(deps.storage, id, &true)?;
                }
            }

            CONFIG_STR_CASE_SENSITIVE.save(deps.storage, &false)?;

            // Max len of string index values:
            let max_len: usize = 128;

            // Pad all existing string index values
            for index_name in string_indices.iter() {
                let storage_key = build_index_storage_key(index_name);
                let map: CustomIndexMap<&String> = Map::new(&storage_key);
                let old_keys: Vec<(u32, String, u64)> = map
                    .keys(deps.storage, None, None, Order::Ascending)
                    .map(|r| r.unwrap())
                    .collect();
                map.clear(deps.storage);
                for (a, b, c) in old_keys.iter() {
                    let padded_str = pad(b, max_len).to_lowercase();
                    map.save(deps.storage, (*a, &padded_str, *c), &X)?;
                }
            }

            // Set default index string max length
            if !CONFIG_STR_MAX_LEN.exists(deps.storage) {
                CONFIG_STR_MAX_LEN.save(deps.storage, &(max_len as u16))?;
            }

            // TODO: Pad tags
            let ix_tag_entries: Vec<_> = IX_TAG
                .range(deps.storage, None, None, Order::Ascending)
                .map(|k| k.unwrap())
                .collect();

            IX_TAG.clear(deps.storage);
            for ((a, b, c), v) in ix_tag_entries.iter() {
                IX_TAG.save(deps.storage, (*a, &pad(b, max_len), *c), v)?;
            }

            let tag_count_entries: Vec<_> = PARTITION_TAG_COUNTS
                .range(deps.storage, None, None, Order::Ascending)
                .map(|k| k.unwrap())
                .collect();

            PARTITION_TAG_COUNTS.clear(deps.storage);
            for ((p, tag), v) in tag_count_entries.iter() {
                PARTITION_TAG_COUNTS.save(deps.storage, (*p, &pad(tag, max_len)), &v)?;
            }

            let contract_tag_keys: Vec<_> = CONTRACT_TAGS
                .keys(deps.storage, None, None, Order::Ascending)
                .map(|k| k.unwrap())
                .collect();

            CONTRACT_TAGS.clear(deps.storage);
            for (p, tag) in contract_tag_keys.iter() {
                CONTRACT_TAGS.save(deps.storage, (*p, pad(tag, max_len)), &X)?;
            }

            // TODO: Pad relationship names
            {
                let entries: Vec<_> = REL_ADDR_2_ID
                    .range(deps.storage, None, None, Order::Ascending)
                    .map(|k| k.unwrap())
                    .collect();
                REL_ADDR_2_ID.clear(deps.storage);
                for ((addr_str, name, id_str), v) in entries.iter() {
                    REL_ADDR_2_ID.save(
                        deps.storage,
                        (addr_str.clone(), pad(name, max_len), id_str.clone()),
                        v,
                    )?;
                }
            }
            {
                let entries: Vec<_> = REL_ID_2_ADDR
                    .range(deps.storage, None, None, Order::Ascending)
                    .map(|k| k.unwrap())
                    .collect();
                REL_ID_2_ADDR.clear(deps.storage);
                for ((id, name, addr_str), v) in entries.iter() {
                    REL_ID_2_ADDR.save(
                        deps.storage,
                        (*id, pad(name, max_len), addr_str.clone()),
                        v,
                    )?;
                }
            }
        },
    }
    Ok(Response::default())
}

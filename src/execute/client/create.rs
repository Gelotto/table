use cosmwasm_std::{
    attr, Addr, DepsMut, Env, Event, Reply, Response, StdResult, Storage, SubMsg, Uint64, WasmMsg,
};
use cw_lib::utils::state::increment;

use crate::{
    ensure::ensure_authorized_code_id,
    error::ContractError,
    execute::Context,
    models::{ContractMetadata, ReplyJob},
    msg::CreationParams,
    state::{
        append_group, ensure_allowed_by_acl, ensure_contract_not_suspended,
        exists_contract_address, load_contract_id, load_next_contract_id, resolve_partition_id,
        CONTRACT_METADATA, IX_CODE_ID, IX_CONTRACT_ID, IX_CREATED_AT, IX_CREATED_BY, IX_REV,
        IX_UPDATED_AT, IX_UPDATED_BY, PARTITION_SIZES, REPLY_JOBS, REPLY_JOB_ID_COUNTER, X,
    },
};

pub fn on_execute(
    ctx: Context,
    params: CreationParams,
) -> Result<Response, ContractError> {
    let Context { deps, info, env } = ctx;
    let action = "create";

    ensure_authorized_code_id(deps.storage, params.code_id.into())?;

    // If sender isn't the contract itself, only allow sender if auth'd by owner
    // address or ACL.
    if !exists_contract_address(deps.storage, &info.sender) {
        ensure_allowed_by_acl(&deps, &info.sender, "/table/create")?;
    } else {
        let sender_contract_id = load_contract_id(deps.storage, &info.sender)?;
        ensure_contract_not_suspended(deps.storage, sender_contract_id)?;
    }

    let initiator = &info.sender;
    let job_id = create_reply_job(deps.storage, &params, initiator)?;
    let admin: Option<String> = Some(params.admin.unwrap_or(env.contract.address).into());
    let label = params
        .label
        .unwrap_or_else(|| format!("Contract-{}", job_id));

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", action),
            attr("job_id", job_id.to_string()),
        ])
        .add_submessage(SubMsg::reply_always(
            WasmMsg::Instantiate {
                code_id: params.code_id.into(),
                msg: params.instantiate_msg.clone(),
                funds: info.funds,
                admin,
                label,
            },
            job_id,
        )))
}

fn create_reply_job(
    storage: &mut dyn Storage,
    msg: &CreationParams,
    initiator: &Addr,
) -> Result<u64, ContractError> {
    let job_id: u64 = increment(storage, &REPLY_JOB_ID_COUNTER, Uint64::one())?.into();
    let job = ReplyJob::Create {
        params: msg.clone(),
        initiator: initiator.clone(),
    };
    REPLY_JOBS.save(storage, job_id, &job)?;
    Ok(job_id)
}

pub fn on_reply(
    deps: DepsMut,
    env: Env,
    reply: Reply,
    params: CreationParams,
    initiator: Addr,
) -> Result<Response, ContractError> {
    let mut resp = Response::new();

    match &reply.result {
        cosmwasm_std::SubMsgResult::Ok(subcall_resp) => {
            if let Some(e) = subcall_resp.events.iter().find(|e| e.ty == "instantiate") {
                if let Some(attr) = e
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "_contract_address")
                {
                    let contract_addr = Addr::unchecked(attr.value.to_string());
                    let contract_id = load_next_contract_id(deps.storage, &contract_addr)?;
                    let p = resolve_partition_id(deps.storage, params.partition)?;

                    // init creation-time contract metadata
                    let metadata = ContractMetadata {
                        id: contract_id.into(),
                        is_managed: params.admin.is_none(),
                        created_at_height: env.block.height.into(),
                        created_at: env.block.time,
                        created_by: initiator.clone(),
                        code_id: params.code_id.into(),
                        partition: p,
                    };

                    CONTRACT_METADATA.save(deps.storage, contract_id, &metadata)?;

                    PARTITION_SIZES.update(deps.storage, p, |maybe_n| -> StdResult<_> {
                        Ok(maybe_n.unwrap_or_default() + Uint64::one())
                    })?;

                    IX_CONTRACT_ID.save(deps.storage, (p, contract_id, contract_id), &X)?;
                    IX_CODE_ID.save(deps.storage, (p, params.code_id.into(), contract_id), &X)?;
                    IX_REV.save(deps.storage, (p, 1, contract_id), &X)?;
                    IX_CREATED_BY.save(
                        deps.storage,
                        (p, initiator.to_string(), contract_id),
                        &X,
                    )?;
                    IX_UPDATED_BY.save(
                        deps.storage,
                        (p, initiator.to_string(), contract_id),
                        &X,
                    )?;
                    IX_CREATED_AT.save(
                        deps.storage,
                        (p, env.block.time.nanos(), contract_id),
                        &X,
                    )?;
                    IX_UPDATED_AT.save(
                        deps.storage,
                        (p, env.block.time.nanos(), contract_id),
                        &X,
                    )?;

                    if let Some(group_ids) = params.groups {
                        for group_id in group_ids.iter() {
                            append_group(deps.storage, *group_id, contract_id)?;
                        }
                    }

                    resp = resp.add_event(
                        Event::new("post_create")
                            .add_attribute("contract_address", contract_addr.to_string())
                            .add_attribute("contract_id", contract_id.to_string()),
                    )
                }
            }
        },
        cosmwasm_std::SubMsgResult::Err(err_reason) => {
            return Err(ContractError::CreateError {
                reason: err_reason.into(),
            });
        },
    }

    Ok(resp)
}

use cosmwasm_std::{attr, to_json_binary, Response, WasmMsg};
use cw_storage_plus::Deque;

use crate::{
    context::Context,
    error::ContractError,
    lifecycle::{LifecycleArgs, LifecycleExecuteMsg, LifecycleExecuteMsgEnvelope},
    models::ContractFlag,
    msg::FlagParams,
    state::{
        ensure_allowed_by_acl, ensure_contract_not_suspended, load_contract_id,
        CONTRACT_SUSPENSIONS, CONTRACT_USES_LIFECYCLE_HOOKS,
    },
};

pub fn on_execute(
    ctx: Context,
    params: FlagParams,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let action = "flag";

    let contract_addr = deps.api.addr_validate(params.contract.as_str())?;
    let contract_id = load_contract_id(deps.storage, &contract_addr)?;

    // If sender isn't the contract itself, only allow sender if auth'd by owner
    // address or ACL.
    if contract_addr != info.sender {
        ensure_allowed_by_acl(&deps, &info.sender, "/table/flag")?;
    } else {
        ensure_contract_not_suspended(deps.storage, contract_id)?;
    };

    let flags_deque_key = format!("_flags_{}", contract_id);
    let flags: Deque<ContractFlag> = Deque::new(flags_deque_key.as_str());
    let mut resp = Response::new().add_attributes(vec![attr("action", action)]);

    if params.suspend.unwrap_or(false) {
        CONTRACT_SUSPENSIONS.save(deps.storage, contract_id, &true)?;
        if CONTRACT_USES_LIFECYCLE_HOOKS
            .may_load(deps.storage, contract_id.into())?
            .unwrap_or_default()
        {
            resp = resp.add_message(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_json_binary(&LifecycleExecuteMsgEnvelope::Lifecycle(
                    LifecycleExecuteMsg::Suspend(LifecycleArgs {
                        table: env.contract.address.clone(),
                        initiator: info.sender.clone(),
                    }),
                ))?,
                funds: vec![],
            });
        }
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

    Ok(resp)
}

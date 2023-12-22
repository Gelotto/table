use cosmwasm_std::{to_json_binary, Addr, Response, WasmMsg};

use crate::{
    context::Context,
    error::ContractError,
    lifecycle::{LifecycleArgs, LifecycleExecuteMsg, LifecycleExecuteMsgEnvelope},
    state::{
        ensure_allowed_by_acl, CONTRACT_ADDR_2_ID, CONTRACT_SUSPENSIONS,
        CONTRACT_USES_LIFECYCLE_HOOKS,
    },
};

/// Load and restore the previous config, provided there is one.
pub fn on_execute(
    ctx: Context,
    contract_addr: Addr,
) -> Result<Response, ContractError> {
    let action = "unsuspend";
    let Context { deps, info, env } = ctx;
    let mut resp = Response::new().add_attribute("action", action);

    // Only owner authority can un-suspend a contract
    ensure_allowed_by_acl(&deps, &info.sender, "/table/unsuspend")?;
    if let Some(id) = CONTRACT_ADDR_2_ID.may_load(deps.storage, &contract_addr)? {
        CONTRACT_SUSPENSIONS.remove(deps.storage, id.into());
        if CONTRACT_USES_LIFECYCLE_HOOKS
            .may_load(deps.storage, id.into())?
            .unwrap_or_default()
        {
            resp = resp.add_message(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_json_binary(&LifecycleExecuteMsgEnvelope::Lifecycle(
                    LifecycleExecuteMsg::Resume(LifecycleArgs {
                        table: env.contract.address.clone(),
                        initiator: info.sender.clone(),
                    }),
                ))?,
                funds: vec![],
            });
        }
    }

    Ok(resp)
}

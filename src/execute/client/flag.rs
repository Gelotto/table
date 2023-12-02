use cosmwasm_std::{attr, Response};
use cw_storage_plus::Deque;

use crate::{
    error::ContractError,
    execute::Context,
    models::ContractFlag,
    msg::FlagParams,
    state::{
        ensure_allowed_by_acl, ensure_contract_not_suspended, load_contract_id,
        CONTRACT_SUSPENSIONS,
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

    if params.suspend.unwrap_or(false) {
        CONTRACT_SUSPENSIONS.save(deps.storage, contract_id, &true)?;
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

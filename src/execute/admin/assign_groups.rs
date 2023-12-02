use cosmwasm_std::Response;

use crate::{
    context::Context,
    error::ContractError,
    msg::GroupUpdates,
    state::{append_group, ensure_allowed_by_acl, load_contract_id, remove_from_group},
};

pub fn on_execute(
    ctx: Context,
    updates: GroupUpdates,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    ensure_allowed_by_acl(&deps, &info.sender, "/table/assign-groups")?;

    let contract_addr = updates.contract;
    let contract_id = load_contract_id(deps.storage, &contract_addr)?;

    // Remove contract from the given groups.
    if let Some(group_ids) = updates.remove {
        for group_id in group_ids.iter() {
            remove_from_group(deps.storage, *group_id, contract_id)?;
        }
    }

    // Add contract to the given groups.
    if let Some(group_ids) = updates.add {
        for group_id in group_ids.iter() {
            append_group(deps.storage, *group_id, contract_id)?;
        }
    }

    Ok(Response::new().add_attribute("action", "assign_groups"))
}

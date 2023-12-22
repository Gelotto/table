use crate::error::ContractError;
use crate::msg::{ContractIsRelatedToParams, ContractIsRelatedToResponse};
use crate::state::{load_contract_id, CONFIG_STR_MAX_LEN, REL_ADDR_2_ID};
use crate::util::pad;
use cosmwasm_std::Deps;

pub fn is_related_to(
    deps: Deps,
    params: ContractIsRelatedToParams,
) -> Result<ContractIsRelatedToResponse, ContractError> {
    let ContractIsRelatedToParams {
        contract: contract_addr,
        address: target_addr,
        relationships: relationship_names,
    } = params;

    let max_str_len = CONFIG_STR_MAX_LEN.load(deps.storage)? as usize;
    let contract_id = load_contract_id(deps.storage, &contract_addr)?;
    let mut is_related = true;

    for rel_name in relationship_names.iter() {
        if !REL_ADDR_2_ID.has(
            deps.storage,
            (
                target_addr.to_string(),
                pad(rel_name, max_str_len),
                contract_id.to_string(),
            ),
        ) {
            is_related = false;
            break;
        }
    }

    Ok(ContractIsRelatedToResponse { is_related })
}

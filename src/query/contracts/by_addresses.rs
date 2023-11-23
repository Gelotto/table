use crate::error::ContractError;
use crate::msg::{AddressesQueryParams, ContractRecord, ContractsByAddressResponse};
use crate::state::{load_contract_id, load_one_contract_record};
use cosmwasm_std::Deps;

/// Paginate over contracts by address
pub fn by_addresses(
    deps: Deps,
    params: &mut AddressesQueryParams,
) -> Result<ContractsByAddressResponse, ContractError> {
    if params.contracts.is_empty() {
        return Ok(ContractsByAddressResponse {
            contracts: vec![],
            cursor: None,
        });
    }

    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let i_start = params.cursor.unwrap_or(0).clamp(0, limit as u32 - 1) as usize;
    let i_stop = (i_start + limit).min(params.contracts.len() - 1);
    let desc = params.desc.unwrap_or(false);

    if desc {
        params.contracts.reverse();
    };

    let mut contracts: Vec<ContractRecord> = Vec::with_capacity(limit);

    for i in i_start..i_stop {
        let contract_addr = &params.contracts[i];
        let contract_id = load_contract_id(deps.storage, contract_addr)?;
        contracts.push(load_one_contract_record(
            deps.storage,
            contract_id,
            params.details.clone(),
        )?);
    }

    let cursor = if i_stop < contracts.len() {
        Some(i_stop as u32)
    } else {
        None
    };

    Ok(ContractsByAddressResponse { contracts, cursor })
}

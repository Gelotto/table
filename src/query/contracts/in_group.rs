use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ContractRecord, ContractsByGroupResponse, GroupQueryParams};
use crate::state::{load_contract_records, ContractID, IX_GROUP};
use cosmwasm_std::{Deps, Order, Uint64};
use cw_storage_plus::Bound;

/// Paginate the contracts in a given group.
pub fn in_group(
    deps: Deps,
    params: GroupQueryParams,
) -> Result<ContractsByGroupResponse, ContractError> {
    let group_id = params.group;
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };
    let (min, max) = match order {
        Order::Ascending => (
            params
                .cursor
                .and_then(|group_id| Some(Bound::Exclusive((group_id.u64(), PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            params
                .cursor
                .and_then(|group_id| Some(Bound::Exclusive((group_id.u64(), PhantomData)))),
        ),
    };

    let mut contract_ids: Vec<ContractID> = vec![];
    let mut cursor: Option<Uint64> = None;

    // Read one page of the group's contract ID's
    for maybe_contract_id in IX_GROUP
        .prefix(group_id)
        .keys(deps.storage, min, max, order)
    {
        contract_ids.push(maybe_contract_id?);
    }

    // Get cursor needed for next page
    if contract_ids.len() == limit {
        cursor = Some(Uint64::from(contract_ids.last().unwrap().clone()))
    }

    // Load contract records from IDs
    let contracts: Vec<ContractRecord> =
        load_contract_records(deps.storage, &contract_ids, params.details)?;

    Ok(ContractsByGroupResponse { contracts, cursor })
}

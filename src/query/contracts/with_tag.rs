use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ContractRecord, ContractsByTagResponse, TagQueryParams};
use crate::state::{load_contract_records, CONFIG_STR_MAX_LEN, IX_TAG};
use crate::util::pad;
use cosmwasm_std::{Deps, Order, Uint64};
use cw_storage_plus::Bound;

/// Paginate over the tags + tag metadata within a given partition.
pub fn with_tag(
    deps: Deps,
    params: TagQueryParams,
) -> Result<ContractsByTagResponse, ContractError> {
    // let exact = params.exact.unwrap_or(true);
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
                .and_then(|start_id| Some(Bound::Exclusive((start_id.u64(), PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            params
                .cursor
                .and_then(|start_id| Some(Bound::Exclusive((start_id.u64(), PhantomData)))),
        ),
    };

    let max_str_len = CONFIG_STR_MAX_LEN.load(deps.storage)? as usize;
    let cannonical_tag = pad(&params.tag, max_str_len);

    // Collect contract ids, cursor and add them to push them on return vals
    let mut contract_ids: Vec<u64> = Vec::with_capacity(4);
    let mut cursor: Option<Uint64> = None;

    for maybe_contract_id in IX_TAG
        .prefix((params.partition, &cannonical_tag))
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let contract_id = maybe_contract_id?;
        contract_ids.push(contract_id);
    }

    if contract_ids.len() == limit {
        cursor = Some(Uint64::from(contract_ids.last().unwrap().clone()));
    }

    // Load contract records from IDs
    let contracts: Vec<ContractRecord> =
        load_contract_records(deps.storage, &contract_ids, params.details)?;

    Ok(ContractsByTagResponse { contracts, cursor })
}

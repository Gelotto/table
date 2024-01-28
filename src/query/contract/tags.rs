use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ContractTagsQueryParams, ContractTagsResponse};
use crate::state::{load_contract_id, CONTRACT_TAGS};
use crate::util::trim_padding;
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

pub fn query_tags(
    deps: Deps,
    params: ContractTagsQueryParams,
) -> Result<ContractTagsResponse, ContractError> {
    let contract_id = load_contract_id(deps.storage, &params.contract)?;
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
                .and_then(|start_tag| Some(Bound::Exclusive((start_tag, PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            params
                .cursor
                .and_then(|start_tag| Some(Bound::Exclusive((start_tag, PhantomData)))),
        ),
    };

    let mut cursor: Option<String> = None;
    let mut tags: Vec<String> = Vec::with_capacity(4);

    for maybe_tag in CONTRACT_TAGS
        .prefix(contract_id)
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let tag = maybe_tag?;
        tags.push(trim_padding(&tag));
    }

    if tags.len() == limit {
        cursor = Some(tags.last().unwrap().clone());
    }

    Ok(ContractTagsResponse { tags, cursor })
}

use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ContractGroupsQueryParams, ContractGroupsResponse, GroupMetadataView};
use crate::state::{load_contract_id, GroupID, CONTRACT_GROUP_IDS, GROUP_METADATA};
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

pub fn query_groups(
    deps: Deps,
    params: ContractGroupsQueryParams,
) -> Result<ContractGroupsResponse, ContractError> {
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
                .and_then(|group_id| Some(Bound::Exclusive((group_id, PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            params
                .cursor
                .and_then(|group_id| Some(Bound::Exclusive((group_id, PhantomData)))),
        ),
    };

    let mut cursor: Option<GroupID> = None;
    let mut groups: Vec<GroupMetadataView> = Vec::with_capacity(4);

    // Append relationships vec
    for maybe_key in CONTRACT_GROUP_IDS
        .prefix(contract_id)
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let group_id = maybe_key?;
        let meta = GROUP_METADATA.load(deps.storage, group_id)?;
        groups.push(GroupMetadataView {
            id: group_id.into(),
            name: meta.name,
            created_at: meta.created_at,
            description: meta.description,
            size: meta.size,
        });
    }

    if groups.len() == limit {
        cursor = Some(groups.last().unwrap().id);
    }

    Ok(ContractGroupsResponse { groups, cursor })
}

pub fn _query_groups(
    deps: Deps,
    params: ContractGroupsQueryParams,
) -> Result<ContractGroupsResponse, ContractError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };

    let mut cursor: Option<GroupID> = None;
    let mut groups: Vec<GroupMetadataView> = Vec::with_capacity(4);

    // Get min, max range bounds. For paginating in descending order, we swap
    // their positions in the range call.
    let contract_id = load_contract_id(deps.storage, &params.contract)?;

    let mut max = None;
    let mut min = params
        .cursor
        .and_then(|group_id| Some(Bound::Exclusive(((contract_id, group_id), PhantomData))));

    if desc {
        (min, max) = (max, min)
    }

    deps.api.debug(format!(">>>> min: {:?}", min).as_str());
    deps.api.debug(format!(">>>> max: {:?}", max).as_str());

    // Append relationships vec
    for maybe_key in CONTRACT_GROUP_IDS
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let (_, group_id) = maybe_key?;
        let meta = GROUP_METADATA.load(deps.storage, group_id)?;
        groups.push(GroupMetadataView {
            id: group_id.into(),
            name: meta.name,
            created_at: meta.created_at,
            description: meta.description,
            size: meta.size,
        });
    }

    if groups.len() == limit {
        cursor = Some(groups.last().unwrap().id);
    }

    Ok(ContractGroupsResponse { groups, cursor })
}

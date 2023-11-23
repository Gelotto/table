use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{
    ContractRelationshipsQueryParams, ContractRelationshipsResponse, RelationshipAddresses,
};
use crate::state::{load_contract_id, REL_CONTRACT_ID_2_ADDR};
use cosmwasm_std::{Addr, Deps, Order};
use cw_storage_plus::Bound;

pub fn query_relationships(
    deps: Deps,
    params: ContractRelationshipsQueryParams,
) -> Result<ContractRelationshipsResponse, ContractError> {
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
                .and_then(|(r, a)| Some(Bound::Exclusive(((contract_id, r, a), PhantomData)))),
            Some(Bound::Exclusive((
                (contract_id + 1, "".to_owned(), "".to_owned()),
                PhantomData,
            ))),
        ),
        Order::Descending => (
            Some(Bound::Inclusive((
                (contract_id, "".to_owned(), "".to_owned()),
                PhantomData,
            ))),
            params
                .cursor
                .and_then(|(r, a)| Some(Bound::Exclusive(((contract_id, r, a), PhantomData)))),
        ),
    };

    let mut cursor: Option<(String, String)> = None;

    let mut name_2_rel_addrs: HashMap<String, RelationshipAddresses> = HashMap::with_capacity(4);
    let mut ordered_names: Vec<String> = Vec::with_capacity(4);

    // Append relationships vec
    for result in REL_CONTRACT_ID_2_ADDR
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let (_, name, addr_str) = result?;

        cursor = Some((name.clone(), addr_str.clone()));

        if let Some(rel_addrs) = name_2_rel_addrs.get_mut(&name) {
            rel_addrs.addresses.push(Addr::unchecked(addr_str));
        } else {
            ordered_names.push(name.clone());
            name_2_rel_addrs.insert(
                name.clone(),
                RelationshipAddresses {
                    name,
                    addresses: vec![Addr::unchecked(addr_str)],
                },
            );
        };
    }

    Ok(ContractRelationshipsResponse {
        cursor,
        relationships: ordered_names
            .iter()
            .map(|name| name_2_rel_addrs.get(name).unwrap().to_owned())
            .collect(),
    })
}

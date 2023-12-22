use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{
    ContractRelationshipsQueryParams, ContractRelationshipsResponse, RelationshipAddresses,
};
use crate::state::{load_contract_id, CONFIG_STR_MAX_LEN, REL_ID_2_ADDR};
use crate::util::{pad, trim_padding};
use cosmwasm_std::{Addr, Deps, Order};
use cw_storage_plus::Bound;

pub fn query_relationships(
    deps: Deps,
    params: ContractRelationshipsQueryParams,
) -> Result<ContractRelationshipsResponse, ContractError> {
    let contract_id = load_contract_id(deps.storage, &params.contract)?;
    let max_str_len = CONFIG_STR_MAX_LEN.load(deps.storage)? as usize;
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let desc = params.desc.unwrap_or(false);
    let blank = pad("", max_str_len);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };
    let (min, max) = match order {
        Order::Ascending => (
            params.cursor.and_then(|(r, a)| {
                Some(Bound::Exclusive((
                    (contract_id, pad(&r, max_str_len), a),
                    PhantomData,
                )))
            }),
            Some(Bound::Exclusive((
                (contract_id + 1, blank.clone(), blank.clone()),
                PhantomData,
            ))),
        ),
        Order::Descending => (
            Some(Bound::Inclusive((
                (contract_id, blank.clone(), blank.clone()),
                PhantomData,
            ))),
            params.cursor.and_then(|(r, a)| {
                Some(Bound::Exclusive((
                    (contract_id, pad(&r, max_str_len), a),
                    PhantomData,
                )))
            }),
        ),
    };

    let mut cursor: Option<(String, String)> = None;
    let mut name_2_rel_addrs: HashMap<String, RelationshipAddresses> = HashMap::with_capacity(4);
    let mut ordered_names: Vec<String> = Vec::with_capacity(4);

    // Append relationships vec
    for result in REL_ID_2_ADDR
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let (_, name, addr_str) = result?;
        let name = trim_padding(&name);

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

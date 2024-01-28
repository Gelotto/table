use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{
    Range, ReadRelationshipResponse, RelatedContract, RelationshipMetadata, RelationshipQueryParams,
};
use crate::state::{
    load_one_contract_record, ContractID, CONFIG_STR_MAX_LEN, REL_ADDR_2_ID, UNIQUE,
};
use crate::util::{pad, parse, trim_padding};
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

/// Paginate over lists of relationships between contracts in the table and
/// other arbitrary addresses. Use this query to paginate over lists of
/// relationships. Relationships are N-to-M.
///
/// Using RelationshipSide::Contract allows you to query relationships from the
/// PoV of a given smart contract contained in the table. Conversely, using
/// RelationshipSide::Account, you can query all smart contracts with
/// relationships to a given arbitrary address.
pub fn related_to(
    deps: Deps,
    params: RelationshipQueryParams,
) -> Result<ReadRelationshipResponse, ContractError> {
    let max_str_len = CONFIG_STR_MAX_LEN.load(deps.storage)? as usize;
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };

    let (start_name, stop_name) = if params.cursor.is_some() {
        (String::default(), String::default())
    } else {
        match params.name {
            Some(target) => match target {
                crate::msg::Target::Equals(name) => (name, String::new()),
                crate::msg::Target::Between(Range { start, stop }) => {
                    (start.unwrap_or_default(), stop.unwrap_or_default())
                },
            },
            None => (String::default(), String::default()),
        }
    };

    let (min, max) = match order {
        Order::Ascending => (
            if let Some((rel_name, c_id_str)) = params.cursor {
                Some(Bound::Exclusive((
                    (
                        params.address.to_string(),
                        pad(&rel_name, max_str_len),
                        c_id_str,
                    ),
                    PhantomData,
                )))
            } else {
                Some(Bound::Inclusive((
                    (
                        params.address.to_string(),
                        pad(&start_name, max_str_len),
                        ContractID::MIN.to_string(),
                    ),
                    PhantomData,
                )))
            },
            if stop_name.is_empty() {
                None
            } else {
                Some(Bound::Inclusive((
                    (
                        params.address.to_string(),
                        pad(&stop_name, max_str_len),
                        ContractID::MAX.to_string(), // TODO: do not convert contract ID to string since it messes up ordering
                    ),
                    PhantomData,
                )))
            },
        ),
        Order::Descending => (
            if stop_name.is_empty() {
                None
            } else {
                Some(Bound::Inclusive((
                    (
                        params.address.to_string(),
                        pad(&stop_name, max_str_len),
                        ContractID::MAX.to_string(), // TODO: do not convert contract ID to string since it messes up ordering
                    ),
                    PhantomData,
                )))
            },
            if let Some((rel_name, c_id_str)) = params.cursor {
                Some(Bound::Exclusive((
                    (
                        params.address.to_string(),
                        pad(&rel_name, max_str_len),
                        c_id_str,
                    ),
                    PhantomData,
                )))
            } else {
                Some(Bound::Exclusive((
                    (
                        format!("{}1", params.address),
                        pad(&start_name, max_str_len),
                        ContractID::MIN.to_string(),
                    ),
                    PhantomData,
                )))
            },
        ),
    };

    let mut cursor: Option<(String, String)> = None;
    let mut contract_ids: Vec<ContractID> = Vec::with_capacity(4);

    // Return a unique record for each contract related to the given address
    // param. Each record contains a ContractRecord and a list of relationship
    // names that adhere between it and the address param, like:
    // { contract: {...}, relationships: ["player", "winner"] }
    let mut memoized: HashMap<ContractID, RelatedContract> = HashMap::with_capacity(4);
    let target_contract_addr_str = params.address.to_string();

    for result in REL_ADDR_2_ID
        .range(deps.storage, min, max, order)
        .take(limit)
    {
        let ((contract_addr, name, contract_id_str), uniqueness) = result?;
        let name = trim_padding(&name);

        if (stop_name.is_empty() && !start_name.is_empty() && name != start_name)
            || (!stop_name.is_empty() && name > stop_name)
            || (contract_addr != target_contract_addr_str)
        {
            break;
        }

        let related_contract_id = parse::<u64>(contract_id_str.clone())?;

        if let Some(contract_rel) = memoized.get_mut(&related_contract_id) {
            contract_rel.relationships.push(RelationshipMetadata {
                name: name.clone(),
                unique: uniqueness == UNIQUE,
            });
        } else {
            contract_ids.push(related_contract_id);
            memoized.insert(
                related_contract_id,
                RelatedContract {
                    contract: load_one_contract_record(
                        deps.storage,
                        related_contract_id,
                        params.details.clone(),
                    )?,
                    relationships: vec![RelationshipMetadata {
                        name: name.clone(),
                        unique: uniqueness == UNIQUE,
                    }],
                },
            );
        }

        cursor = Some((name, contract_id_str));
    }

    Ok(ReadRelationshipResponse {
        cursor,
        contracts: contract_ids
            .iter()
            .map(|id| memoized.get(id).unwrap().clone())
            .collect(),
    })
}

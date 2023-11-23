use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ReadRelationshipResponse, RelatedContract, RelationshipQueryParams};
use crate::state::{load_one_contract_record, ContractID, REL_ADDR_2_CONTRACT_ID};
use crate::util::parse;
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
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };
    // let (min, max) = match order {
    //   Order::Ascending => (
    //     params.cursor.and_then(|(rel_name, c_id)| {
    //       Some(Bound::Exclusive((
    //         (params.address.to_string(), rel_name, c_id),
    //         PhantomData,
    //       )))
    //     }),
    //     None,
    //   ),
    //   Order::Descending => (
    //     None,
    //     params.cursor.and_then(|(rel_name, c_id)| {
    //       Some(Bound::Exclusive((
    //         (params.address.to_string(), rel_name, c_id),
    //         PhantomData,
    //       )))
    //     }),
    //   ),
    // };

    let (min, max) = match order {
        Order::Ascending => (
            if let Some((rel_name, c_id_str)) = params.cursor {
                Some(Bound::Exclusive((
                    (params.address.to_string(), rel_name.clone(), c_id_str),
                    PhantomData,
                )))
            } else {
                Some(Bound::Inclusive((
                    (
                        params.address.to_string(),
                        "".to_string(),
                        ContractID::MIN.to_string(),
                    ),
                    PhantomData,
                )))
            },
            None,
        ),
        Order::Descending => (
            None,
            if let Some((rel_name, c_id_str)) = params.cursor {
                Some(Bound::Exclusive((
                    (params.address.to_string(), rel_name.clone(), c_id_str),
                    PhantomData,
                )))
            } else {
                Some(Bound::Exclusive((
                    (
                        format!("{}1", params.address),
                        "".to_string(),
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

    for result in REL_ADDR_2_CONTRACT_ID
        .keys(deps.storage, min, max, order)
        .take(limit)
    {
        let (contract_addr, name, contract_id_str) = result?;

        if contract_addr != target_contract_addr_str {
            break;
        }

        let related_contract_id = parse::<u64>(contract_id_str.clone())?;

        if let Some(contract_rel) = memoized.get_mut(&related_contract_id) {
            contract_rel.relationships.push(name.clone());
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
                    relationships: vec![name.clone()],
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

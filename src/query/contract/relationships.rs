use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ContractRelationshipsQueryParams, ContractRelationshipsResponse, Relationship};
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
  let order = if desc { Order::Descending } else { Order::Ascending };
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
  let mut relationships: Vec<Relationship> = Vec::with_capacity(4);

  // Append relationships vec
  for result in REL_CONTRACT_ID_2_ADDR.keys(deps.storage, min, max, order).take(limit) {
    let (_, name, account_addr_str) = result?;
    cursor = Some((name.clone(), account_addr_str.clone()));
    relationships.push(Relationship {
      address: Addr::unchecked(account_addr_str),
      name,
    });
  }

  Ok(ContractRelationshipsResponse { relationships, cursor })
}

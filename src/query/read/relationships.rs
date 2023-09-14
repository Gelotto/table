use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ReadRelationshipsParams, ReadRelationshipsResponse, Relationship, RelationshipSide};
use crate::state::{load_contract_addr, load_contract_id, REL_ADDR_2_CONTRACT_ID, REL_CONTRACT_ID_2_ADDR};
use crate::util::parse;
use cosmwasm_std::{Addr, Deps, Order};
use cw_storage_plus::Bound;

pub fn read_relationships(
  deps: Deps,
  params: ReadRelationshipsParams,
) -> Result<ReadRelationshipsResponse, ContractError> {
  let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
  let desc = params.desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };

  let mut cursor: Option<(String, String)> = None;
  let mut relationships: Vec<Relationship> = Vec::with_capacity(4);

  match params.side {
    // Return the relationships associated with the given contract address,
    // returning the related addrs and relationship names.
    RelationshipSide::Contract(contract_addr) => {
      // Get min, max range bounds. For paginating in descending order, we swap
      // their positions in the range call.
      let owner_contract_id = load_contract_id(deps.storage, &contract_addr)?;
      let mut min = params.cursor.and_then(|(name, accout_addr)| {
        Some(Bound::Exclusive((
          (owner_contract_id, name.clone(), accout_addr),
          PhantomData,
        )))
      });
      let mut max = None;
      if desc {
        (min, max) = (max, min)
      }

      // Append relationships vec
      for result in REL_CONTRACT_ID_2_ADDR.keys(deps.storage, min, max, order).take(limit) {
        let (_, name, account_addr_str) = result?;
        cursor = Some((name.clone(), account_addr_str.clone()));
        relationships.push(Relationship {
          address: Addr::unchecked(account_addr_str),
          rel: name,
        });
      }
    },

    // Return the relationships associated with the given account address,
    // returning the related contract addrs and relationship names.
    RelationshipSide::Account(account_addr) => {
      // Get min, max range bounds. For paginating in descending order, we swap
      // their positions in the range call.
      let mut min = params
        .cursor
        .and_then(|(name, id)| Some(Bound::Exclusive(((account_addr.to_string(), name, id), PhantomData))));
      let mut max = None;
      if desc {
        (min, max) = (max, min)
      }

      let mut memoized_addrs: HashMap<u64, String> = HashMap::with_capacity(4);
      let storage = deps.storage;

      // Append relationships vec
      for result in REL_ADDR_2_CONTRACT_ID.keys(storage, min, max, order).take(limit) {
        let (_, name, contract_id_str) = result?;
        let related_contract_id = parse::<u64>(contract_id_str.clone())?;
        if let Some(contract_addr) = memoized_addrs.get(&related_contract_id) {
          relationships.push(Relationship {
            rel: name.clone(),
            address: Addr::unchecked(contract_addr),
          });
        } else {
          let contract_addr = load_contract_addr(storage, related_contract_id)?;
          memoized_addrs.insert(related_contract_id, contract_addr.clone().into());
          relationships.push(Relationship {
            address: contract_addr,
            rel: name.clone(),
          });
        }

        cursor = Some((name, contract_id_str));
      }
    },
  }

  Ok(ReadRelationshipsResponse { relationships, cursor })
}

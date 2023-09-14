use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{ReadTagsParams, ReadTagsResponse};
use crate::state::{load_contract_addr, IX_TAG};
use cosmwasm_std::{Addr, Deps, Order, Uint64};
use cw_storage_plus::Bound;

pub fn read_tags(
  deps: Deps,
  params: ReadTagsParams,
) -> Result<ReadTagsResponse, ContractError> {
  let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
  let desc = params.desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };

  let mut contract_id_vecs: Vec<Vec<u64>> = vec![];
  let mut cursors: Vec<Option<Uint64>> = vec![];

  // Build vecs of contract IDs and cursors
  for (i, tag) in params.tags.iter().enumerate() {
    // Starting point to resume pagination:
    let cursor = params.cursors.as_ref().and_then(|v| v.get(i));
    // Get min, max bounds for Map.range
    let (min, max) = if desc {
      (
        None,
        cursor.and_then(|id| Some(Bound::Exclusive(((params.partition, tag, id.u64()), PhantomData)))),
      )
    } else {
      (
        cursor.and_then(|id| Some(Bound::Exclusive(((params.partition, tag, id.u64()), PhantomData)))),
        None,
      )
    };
    // Collect contract ids, cursor and add them to push them on return vals
    let mut contract_ids: Vec<u64> = Vec::with_capacity(4);
    let mut cursor: Option<Uint64> = None;

    for result in IX_TAG.keys(deps.storage, min, max, order).take(limit) {
      let (_, _, contract_id) = result?;
      cursor = Some(contract_id.into());
      contract_ids.push(contract_id);
    }
    cursors.push(cursor);
    contract_id_vecs.push(contract_ids);
  }

  // Convert contract ID's to addresses
  let mut contract_addr_vecs: Vec<Vec<Addr>> = Vec::with_capacity(contract_id_vecs.len());
  let mut memoized_addrs: HashMap<u64, Addr> = HashMap::with_capacity(4);

  for contract_ids in contract_id_vecs.iter() {
    let mut addrs: Vec<Addr> = Vec::with_capacity(contract_ids.len());
    for id in contract_ids.iter() {
      if let Some(addr) = memoized_addrs.get(id) {
        addrs.push(addr.clone());
      } else {
        let addr = load_contract_addr(deps.storage, *id)?;
        memoized_addrs.insert(*id, addr.clone());
        addrs.push(addr);
      }
    }
    contract_addr_vecs.push(addrs);
  }

  Ok(ReadTagsResponse {
    contracts: contract_addr_vecs,
    cursors,
  })
}

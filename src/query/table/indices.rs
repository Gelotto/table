use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{IndexMetadata, IndicesResponse};
use crate::state::INDEX_METADATA;
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

pub const PAGE_SIZE: usize = 50;

/// Return custom index metadata records, created via create_index.
pub fn query_indices(
  deps: Deps,
  maybe_cursor: Option<String>,
  maybe_desc: Option<bool>,
) -> Result<IndicesResponse, ContractError> {
  let mut indices: Vec<IndexMetadata> = Vec::with_capacity(4);

  let desc = maybe_desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };
  let (min, max) = match order {
    Order::Ascending => (
      maybe_cursor.and_then(|start_name| Some(Bound::Exclusive((start_name, PhantomData)))),
      None,
    ),
    Order::Descending => (
      None,
      maybe_cursor.and_then(|start_name| Some(Bound::Exclusive((start_name, PhantomData)))),
    ),
  };

  for result in INDEX_METADATA.range(deps.storage, min, max, order).take(PAGE_SIZE) {
    let (_, meta) = result?;
    indices.push(meta);
  }

  // Get Cursor for next page
  let cursor: Option<String> = if let Some(last) = indices.last() {
    Some(last.name.clone())
  } else {
    None
  };

  Ok(IndicesResponse { indices, cursor })
}

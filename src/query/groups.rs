use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{GroupMetadataView, GroupsResponse};
use crate::state::{GroupID, GROUP_METADATA};
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

pub const PAGE_SIZE: usize = 50;

/// Return custom index metadata records, created via create_index.
pub fn query_groups(
  deps: Deps,
  maybe_cursor: Option<GroupID>,
  maybe_desc: Option<bool>,
) -> Result<GroupsResponse, ContractError> {
  let mut groups: Vec<GroupMetadataView> = Vec::with_capacity(4);

  let desc = maybe_desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };

  let mut min = Some(Bound::Exclusive((maybe_cursor.unwrap_or_default(), PhantomData)));
  let mut max: Option<Bound<GroupID>> = None;

  if desc {
    (min, max) = (max, min)
  }

  for result in GROUP_METADATA.range(deps.storage, min, max, order).take(PAGE_SIZE) {
    let (id, meta) = result?;
    groups.push(GroupMetadataView {
      id,
      description: meta.description,
      created_at: meta.created_at,
      size: meta.size,
      name: meta.name,
    });
  }

  // Get Cursor for next page
  let cursor: Option<GroupID> = if let Some(last) = groups.last() {
    Some(last.id)
  } else {
    None
  };

  Ok(GroupsResponse { groups, cursor })
}

use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{TableTagsQueryParams, TagCount, TagsResponse};
use crate::state::PARTITION_TAG_COUNTS;
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

pub const PAGE_SIZE: usize = 50;

/// Return custom index metadata records, created via create_index.
pub fn query_tags(
  deps: Deps,
  params: TableTagsQueryParams,
) -> Result<TagsResponse, ContractError> {
  let mut tags: Vec<TagCount> = Vec::with_capacity(4);

  let desc = params.desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };
  let start = params.cursor.unwrap_or("".into());
  let (min, max) = match order {
    Order::Ascending => (
      if start.is_empty() {
        None
      } else {
        Some(Bound::Exclusive((&start, PhantomData)))
      },
      None,
    ),
    Order::Descending => (
      None,
      if start.is_empty() {
        None
      } else {
        Some(Bound::Exclusive((&start, PhantomData)))
      },
    ),
  };

  for maybe_entry in PARTITION_TAG_COUNTS
    .prefix(params.partition)
    .range(deps.storage, min, max, order)
    .take(PAGE_SIZE)
  {
    let (tag, count) = maybe_entry?;
    tags.push(TagCount { tag, count });
  }

  // Get Cursor for next page
  let cursor: Option<String> = if let Some(last) = tags.last() {
    Some(last.tag.clone())
  } else {
    None
  };

  Ok(TagsResponse { tags, cursor })
}

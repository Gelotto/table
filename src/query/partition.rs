use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{PartitionResponse, PartitionSelector, TagCount};
use crate::state::{load_partition_id_from_selector, PARTITION_METADATA, PARTITION_SIZES, PARTITION_TAG_COUNTS};
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

// TODO: Keep tag counts in a map with keys that are ordered by count:
// IndexMap<(u32, String)> and upgrade the update API to adjust this map

pub const TAG_COUNT_PAGE_SIZE: usize = 25;

/// Return metadata for the given partition
pub fn query_partition(
  deps: Deps,
  selector: PartitionSelector,
) -> Result<PartitionResponse, ContractError> {
  let partition_id = load_partition_id_from_selector(deps.storage, selector)?;
  let size = PARTITION_SIZES.load(deps.storage, partition_id).unwrap_or_default();
  let meta = PARTITION_METADATA.load(deps.storage, partition_id)?;

  // Get first page of tag counts
  let mut tags: Vec<TagCount> = Vec::with_capacity(TAG_COUNT_PAGE_SIZE);
  let cursor_in: Option<String> = None; // TODO: Implement in a "partition tags" query

  if !size.is_zero() {
    let start_tag = cursor_in.unwrap_or_default();
    let min: Option<Bound<_>> = if !start_tag.is_empty() {
      Some(Bound::Exclusive((&start_tag, PhantomData)))
    } else {
      None
    };
    for result in PARTITION_TAG_COUNTS
      .prefix(partition_id)
      .range(deps.storage, min, None, Order::Ascending)
      .take(TAG_COUNT_PAGE_SIZE)
    {
      let (tag, count) = result?;
      tags.push(TagCount {
        tag: tag.clone(),
        count,
      });
    }
  }

  // // Get Cursor for next page of tag counts
  // let cursor: Option<String> = if let Some(last) = tags.last() {
  //   Some(last.tag.clone())
  // } else {
  //   None
  // };

  Ok(PartitionResponse {
    name: meta.name,
    description: meta.description,
    size,
    tags,
  })
}

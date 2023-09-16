use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{PartitionView, PartitionsResponse};
use crate::state::{PartitionID, PARTITION_METADATA, PARTITION_SIZES};
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

// TODO: Keep tag counts in a map with keys that are ordered by count:
// IndexMap<(u32, String)> and upgrade the update API to adjust this map

pub const PAGE_SIZE: usize = 50;

/// Return metadata for the given partition
pub fn query_partitions(
  deps: Deps,
  maybe_cursor: Option<PartitionID>,
  maybe_desc: Option<bool>,
) -> Result<PartitionsResponse, ContractError> {
  let mut partitions: Vec<PartitionView> = Vec::with_capacity(2);

  let desc = maybe_desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };

  let mut min = Some(Bound::Exclusive((maybe_cursor.unwrap_or_default(), PhantomData)));
  let mut max: Option<Bound<PartitionID>> = None;

  if desc {
    (min, max) = (max, min)
  }

  for result in PARTITION_METADATA.range(deps.storage, min, max, order).take(PAGE_SIZE) {
    let (partition_id, meta) = result?;
    let size = PARTITION_SIZES.load(deps.storage, partition_id).unwrap_or_default();
    partitions.push(PartitionView {
      id: partition_id,
      size,
      description: meta.description,
      name: meta.name,
    })
  }

  // Get Cursor for next page
  let cursor: Option<PartitionID> = if let Some(last) = partitions.last() {
    Some(last.id)
  } else {
    None
  };

  // // Get first page of tag counts
  // let mut tags: Vec<TagCount> = Vec::with_capacity(TAG_COUNT_PAGE_SIZE);
  // let cursor_in: Option<String> = None; // TODO: Implement in a "partition tags" query

  // if !size.is_zero() {
  //   let start_tag = cursor_in.unwrap_or_default();
  //   let min: Option<Bound<_>> = if !start_tag.is_empty() {
  //     Some(Bound::Exclusive((&start_tag, PhantomData)))
  //   } else {
  //     None
  //   };
  //   for result in PARTITION_TAG_COUNTS
  //     .prefix(partition_id)
  //     .range(deps.storage, min, None, Order::Ascending)
  //     .take(TAG_COUNT_PAGE_SIZE)
  //   {
  //     let (tag, count) = result?;
  //     tags.push(TagCount {
  //       tag: tag.clone(),
  //       count,
  //     });
  //   }
  // }

  // // Get Cursor for next page of tag counts
  // let cursor: Option<String> = if let Some(last) = tags.last() {
  //   Some(last.tag.clone())
  // } else {
  //   None
  // };

  Ok(PartitionsResponse { partitions, cursor })
}

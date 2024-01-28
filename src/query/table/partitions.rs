use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{PartitionView, PartitionsResponse, TablePartitionsQueryParams};
use crate::state::{PartitionID, PARTITION_METADATA, PARTITION_SIZES};
use crate::util::parse;
use cosmwasm_std::{Deps, Order};
use cw_storage_plus::Bound;

// TODO: Keep tag counts in a map with keys that are ordered by count:
// IndexMap<(u32, String)> and upgrade the update API to adjust this map

pub const PAGE_SIZE: usize = 50;

/// Return metadata for the given partition
pub fn query_partitions(
    deps: Deps,
    params: TablePartitionsQueryParams,
) -> Result<PartitionsResponse, ContractError> {
    let mut partitions: Vec<PartitionView> = Vec::with_capacity(2);

    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };
    let (min, max) = match order {
        Order::Ascending => (
            params.cursor.and_then(|start| {
                Some(Bound::Exclusive((
                    parse::<PartitionID>(start).ok()?,
                    PhantomData,
                )))
            }),
            None,
        ),
        Order::Descending => (
            None,
            params.cursor.and_then(|start| {
                Some(Bound::Exclusive((
                    parse::<PartitionID>(start).ok()?,
                    PhantomData,
                )))
            }),
        ),
    };

    for result in PARTITION_METADATA
        .range(deps.storage, min, max, order)
        .take(PAGE_SIZE)
    {
        let (partition_id, meta) = result?;
        let size = PARTITION_SIZES
            .load(deps.storage, partition_id)
            .unwrap_or_default();
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

    Ok(PartitionsResponse { partitions, cursor })
}

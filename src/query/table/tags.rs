use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{TableTagsQueryParams, TagCount, TagsResponse};
use crate::state::{CONFIG_STR_MAX_LEN, PARTITION_TAG_COUNTS};
use crate::util::{pad, trim_padding};
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
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };

    let start = pad(
        &params.cursor.unwrap_or("".into()),
        CONFIG_STR_MAX_LEN.load(deps.storage)? as usize,
    );

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
        tags.push(TagCount {
            tag: trim_padding(&tag),
            count,
        });
    }

    // Get Cursor for next page
    let cursor: Option<String> = if let Some(last) = tags.last() {
        Some(last.tag.clone())
    } else {
        None
    };

    Ok(TagsResponse { tags, cursor })
}

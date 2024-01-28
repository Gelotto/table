use std::marker::PhantomData;

use crate::error::ContractError;
use crate::msg::{GroupMetadataView, GroupSelector, GroupsResponse, TableGroupsQueryParams};
use crate::state::{GroupID, GROUP_IX_CREATED_AT, GROUP_IX_NAME, GROUP_METADATA};
use crate::util::parse;
use cosmwasm_std::{Deps, Order, Timestamp};
use cw_storage_plus::Bound;

pub const PAGE_SIZE: usize = 50;

/// Return custom index metadata records, created via create_index.
pub fn query_groups(
    deps: Deps,
    params: TableGroupsQueryParams,
) -> Result<GroupsResponse, ContractError> {
    let desc = params.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };

    let groups: Vec<GroupMetadataView> = if let Some(selector) = params.select {
        match selector {
            GroupSelector::WithName(name) => load_groups_by_name(deps, name, params.cursor, order),
            GroupSelector::CreatedBetween(start, stop) => {
                load_groups_created_between(deps, start, stop, params.cursor, order)
            },
        }
    } else {
        load_groups_from_metadata_map(deps, params.cursor, order)
    }?;

    // Get Cursor for next page
    let cursor: Option<GroupID> = if let Some(last) = groups.last() {
        Some(last.id)
    } else {
        None
    };

    Ok(GroupsResponse { groups, cursor })
}

fn ensure_valid_cursor(
    cursor_vec: &Vec<String>,
    exp_size: usize,
) -> Result<(), ContractError> {
    if cursor_vec.len() != exp_size {
        return Err(ContractError::InvalidCursor {
            reason: format!("Expected cursor with {} component/s", exp_size),
        });
    }
    Ok(())
}

fn load_groups_by_name<'a>(
    deps: Deps,
    group_name: String,
    maybe_cursor: Option<Vec<String>>,
    order: Order,
) -> Result<Vec<GroupMetadataView>, ContractError> {
    let maybe_cursor = if let Some(cursor) = maybe_cursor {
        ensure_valid_cursor(&cursor, 1)?;
        Some(cursor[0].clone())
    } else {
        None
    };

    let (min, max) = match order {
        Order::Ascending => (
            maybe_cursor
                .and_then(|group_id| {
                    Some(Bound::Exclusive((
                        parse::<GroupID>(group_id).ok()?,
                        PhantomData,
                    )))
                })
                .or_else(|| Some(Bound::Inclusive((GroupID::MIN, PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            maybe_cursor
                .and_then(|group_id| {
                    Some(Bound::Exclusive((
                        parse::<GroupID>(group_id).ok()?,
                        PhantomData,
                    )))
                })
                .or_else(|| Some(Bound::Inclusive((GroupID::MAX, PhantomData)))),
        ),
    };

    let mut group_ids: Vec<GroupID> = Vec::with_capacity(8);

    for maybe_group_id in GROUP_IX_NAME
        .prefix(group_name)
        .keys(deps.storage, min, max, order)
        .take(PAGE_SIZE)
    {
        group_ids.push(maybe_group_id?);
    }

    load_groups_by_ids(deps, &group_ids)
}

fn load_groups_created_between<'a>(
    deps: Deps,
    start: Timestamp,
    stop: Timestamp,
    maybe_cursor: Option<Vec<String>>,
    order: Order,
) -> Result<Vec<GroupMetadataView>, ContractError> {
    let maybe_cursor = if let Some(cursor) = maybe_cursor {
        ensure_valid_cursor(&cursor, 2)?;
        Some((cursor[0].clone(), cursor[1].clone()))
    } else {
        None
    };

    let (min, max) = match order {
        Order::Ascending => (
            maybe_cursor
                .and_then(|(t, id)| {
                    Some(Bound::Exclusive((
                        (parse::<u64>(t).ok()?, parse::<GroupID>(id).ok()?),
                        PhantomData,
                    )))
                })
                .or_else(|| {
                    Some(Bound::Inclusive((
                        (start.nanos(), GroupID::MIN),
                        PhantomData,
                    )))
                }),
            Some(Bound::Inclusive((
                (stop.nanos(), GroupID::MAX),
                PhantomData,
            ))),
        ),
        Order::Descending => (
            Some(Bound::Inclusive((
                (start.nanos(), GroupID::MIN),
                PhantomData,
            ))),
            maybe_cursor
                .and_then(|(t, id)| {
                    Some(Bound::Exclusive((
                        (parse::<u64>(t).ok()?, parse::<GroupID>(id).ok()?),
                        PhantomData,
                    )))
                })
                .or_else(|| Some(Bound::Inclusive(((u64::MAX, GroupID::MAX), PhantomData)))),
        ),
    };

    let mut group_ids: Vec<GroupID> = Vec::with_capacity(8);

    for maybe_key in GROUP_IX_CREATED_AT
        .keys(deps.storage, min, max, order)
        .take(PAGE_SIZE)
    {
        let (_, group_id) = maybe_key?;
        group_ids.push(group_id);
    }

    load_groups_by_ids(deps, &group_ids)
}

fn load_groups_by_ids<'a>(
    deps: Deps,
    group_ids: &Vec<GroupID>,
) -> Result<Vec<GroupMetadataView>, ContractError> {
    let mut groups: Vec<GroupMetadataView> = Vec::with_capacity(group_ids.len());
    for group_id in group_ids.iter() {
        if let Some(meta) = GROUP_METADATA.may_load(deps.storage, *group_id)? {
            groups.push(GroupMetadataView {
                id: *group_id,
                description: meta.description,
                created_at: meta.created_at,
                size: meta.size,
                name: meta.name,
            });
        } else {
            return Err(ContractError::GroupNotFound {
                reason: format!("Group {} not found", group_id),
            });
        }
    }

    Ok(groups)
}

fn load_groups_from_metadata_map<'a>(
    deps: Deps,
    maybe_cursor: Option<Vec<String>>,
    order: Order,
) -> Result<Vec<GroupMetadataView>, ContractError> {
    let maybe_cursor = if let Some(cursor) = maybe_cursor {
        ensure_valid_cursor(&cursor, 1)?;
        Some(cursor[0].clone())
    } else {
        None
    };

    let (min, max) = match order {
        Order::Ascending => (
            maybe_cursor
                .and_then(|group_id| {
                    Some(Bound::Exclusive((
                        parse::<GroupID>(group_id).ok()?,
                        PhantomData,
                    )))
                })
                .or_else(|| Some(Bound::Inclusive((GroupID::MIN, PhantomData)))),
            None,
        ),
        Order::Descending => (
            None,
            maybe_cursor
                .and_then(|group_id| {
                    Some(Bound::Exclusive((
                        parse::<GroupID>(group_id).ok()?,
                        PhantomData,
                    )))
                })
                .or_else(|| Some(Bound::Inclusive((GroupID::MAX, PhantomData)))),
        ),
    };

    let mut groups: Vec<GroupMetadataView> = Vec::with_capacity(8);

    for result in GROUP_METADATA
        .range(deps.storage, min, max, order)
        .take(PAGE_SIZE)
    {
        let (id, meta) = result?;
        groups.push(GroupMetadataView {
            id,
            description: meta.description,
            created_at: meta.created_at,
            size: meta.size,
            name: meta.name,
        });
    }
    Ok(groups)
}

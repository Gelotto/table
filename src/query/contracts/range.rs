use std::marker::PhantomData;
use std::str::FromStr;

use crate::msg::{ContractsRangeResponse, Cursor, RangeSelector, Target};
use crate::state::{
    load_contract_records, ContractID, CustomIndexMap, PartitionID, IX_CODE_ID, IX_CONTRACT_ID,
    IX_CREATED_AT, IX_CREATED_BY, IX_REV, IX_UPDATED_AT, IX_UPDATED_BY,
};
use crate::util::{build_index_storage_key, pad, parse, parse_bool};
use crate::{error::ContractError, msg::RangeQueryParams};
use cosmwasm_std::{Api, Binary, Deps, Order, StdResult, Storage, Uint64};
use cw_storage_plus::{Bound, KeyDeserialize, Map, Prefixer, PrimaryKey};

pub fn range(
    deps: Deps,
    query: RangeQueryParams,
) -> Result<ContractsRangeResponse, ContractError> {
    // let limit = query.limit.unwrap_or(20).clamp(1, 200) as usize;
    // let desc = query.desc.unwrap_or(false);
    let details = query.details.clone();

    // Find matching contract ID's
    let (ids, cursor) = match query.params.clone() {
        Target::Equals(value) => {
            get_contract_ids(deps.api, deps.storage, query, Some(value), None, true)
        },
        Target::Between(range) => get_contract_ids(
            deps.api,
            deps.storage,
            query,
            range.start,
            range.stop,
            false,
        ),
    }?;

    // Convert contract ID's to Addrs
    let contracts = load_contract_records(deps.storage, &ids, details)?;

    Ok(ContractsRangeResponse { contracts, cursor })
}

fn build_bounds<'a, T>(
    order: Order,
    partition: PartitionID,
    range_start_value: T,
    range_stop_value: T,
    maybe_cursor: Option<Cursor>,
) -> Result<
    (
        Option<Bound<'a, (PartitionID, T, u64)>>,
        Option<Bound<'a, (PartitionID, T, u64)>>,
    ),
    ContractError,
>
where
    T: PrimaryKey<'a> + Prefixer<'a> + KeyDeserialize + FromStr,
{
    Ok(match order {
        Order::Ascending => {
            (
                // min
                Some(if let Some((p, v_str, id)) = maybe_cursor {
                    let v = parse(v_str)?;
                    Bound::Exclusive(((p, v, id.u64()), PhantomData))
                } else {
                    Bound::Inclusive(((partition, range_start_value, u64::MIN), PhantomData))
                }),
                // max
                Some(Bound::Exclusive((
                    (partition, range_stop_value, u64::MAX),
                    PhantomData,
                ))),
            )
        },
        Order::Descending => {
            (
                // min
                Some(Bound::Inclusive((
                    (partition, range_start_value, u64::MIN),
                    PhantomData,
                ))),
                // max
                Some(if let Some((p, v_str, id)) = maybe_cursor {
                    let v = parse(v_str)?;
                    Bound::Exclusive(((p, v, id.u64()), PhantomData))
                } else {
                    Bound::Inclusive(((partition, range_stop_value, u64::MAX), PhantomData))
                }),
            )
        },
    })
}

fn build_range_bounds_str<'a>(
    order: Order,
    partition: PartitionID,
    range_start_value: Option<String>,
    range_stop_value: Option<String>,
    maybe_cursor: Option<Cursor>,
) -> Result<
    (
        Option<Bound<'a, (PartitionID, String, u64)>>,
        Option<Bound<'a, (PartitionID, String, u64)>>,
    ),
    ContractError,
> {
    Ok(match order {
        Order::Ascending => {
            (
                // min
                if let Some((p, v_str, id)) = maybe_cursor {
                    Some(Bound::Exclusive(((p, v_str, id.u64()), PhantomData)))
                } else if let Some(v) = range_start_value {
                    Some(Bound::Inclusive(((partition, v, u64::MIN), PhantomData)))
                } else {
                    None
                },
                // max
                if let Some(v) = range_stop_value {
                    Some(Bound::Inclusive(((partition, v, u64::MAX), PhantomData)))
                } else {
                    None
                },
            )
        },
        Order::Descending => {
            (
                // min
                if let Some(v) = range_start_value {
                    Some(Bound::Exclusive(((partition, v, u64::MIN), PhantomData)))
                } else {
                    None
                },
                // max
                if let Some((p, v_str, id)) = maybe_cursor {
                    Some(Bound::Exclusive(((p, v_str, id.u64()), PhantomData)))
                } else if let Some(v) = range_stop_value {
                    Some(Bound::Inclusive(((partition, v, u64::MAX), PhantomData)))
                } else {
                    None
                },
            )
        },
    })
}

fn build_start_stop<T: FromStr + Clone>(
    start_value_raw: Option<String>,
    start_value_default: T,
    stop_value_raw: Option<String>,
    stop_value_default: T,
    exact: bool,
    fn_parse: &dyn Fn(String) -> Result<T, ContractError>,
) -> Result<(T, T), ContractError> {
    let start: T = if let Some(raw_value) = start_value_raw {
        fn_parse(raw_value)?
    } else {
        start_value_default
    };

    if exact {
        return Ok((start.clone(), start));
    }

    let stop = if let Some(raw_value) = stop_value_raw {
        parse::<T>(raw_value)?
    } else {
        stop_value_default
    };

    Ok((start, stop))
}

fn build_start_stop_values_str(
    start_value_raw: Option<String>,
    stop_value_raw: Option<String>,
    exact: bool,
    max_str_len: Option<usize>,
) -> Result<(Option<String>, Option<String>), ContractError> {
    let (start_value_raw, stop_value_raw) = if let Some(max_len) = max_str_len {
        (
            start_value_raw.and_then(|s| Some(pad(&s, max_len))),
            stop_value_raw.and_then(|s| Some(pad(&s, max_len))),
        )
    } else {
        (start_value_raw, stop_value_raw)
    };

    if exact {
        return Ok((start_value_raw.clone(), start_value_raw));
    }

    Ok((start_value_raw, stop_value_raw))
}

fn build_start_stop_values_binary<'a>(
    start_value_raw_str: Option<String>,
    stop_value_raw_str: Option<String>,
    exact: bool,
) -> Result<(Option<Vec<u8>>, Option<Vec<u8>>), ContractError> {
    let start_value = if let Some(s) = start_value_raw_str {
        let value = Binary::from_base64(&s)?;
        Some(value.to_vec())
    } else {
        None
    };

    let stop_value = if let Some(s) = stop_value_raw_str {
        let value = Binary::from_base64(&s)?;
        Some(value.to_vec())
    } else {
        None
    };

    if exact {
        return Ok((start_value.clone(), start_value));
    }

    Ok((start_value, stop_value))
}

fn page<'a, D>(
    iter: Box<dyn Iterator<Item = StdResult<(PartitionID, D, ContractID)>> + 'a>,
    limit: usize,
    to_string: &dyn Fn(&D) -> String,
) -> Result<(Vec<ContractID>, Option<Cursor>), ContractError> {
    let limit = limit as usize;
    let mut contract_ids = Vec::with_capacity(limit);
    let mut cursor: Option<Cursor> = None;

    for item in iter.take(limit) {
        let (partition, value, contract_id) = item?;
        cursor = Some((partition, to_string(&value), Uint64::from(contract_id)));
        contract_ids.push(contract_id);
    }

    Ok((contract_ids, cursor))
}

fn get_contract_ids(
    _api: &dyn Api,
    store: &dyn Storage,
    query: RangeQueryParams,
    raw_start: Option<String>,
    raw_stop: Option<String>,
    exact: bool,
) -> Result<(Vec<u64>, Option<Cursor>), ContractError> {
    let partition = query.partition;
    let limit = query.limit.unwrap_or(20).clamp(1, 200) as usize;
    let desc = query.desc.unwrap_or(false);
    let order = if desc {
        Order::Descending
    } else {
        Order::Ascending
    };

    Ok(match &query.index {
        RangeSelector::Id => {
            let index = IX_CONTRACT_ID;
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::CodeId => {
            let index = IX_CODE_ID;
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Rev => {
            let index = IX_REV;
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::CreatedAt => {
            let index = IX_CREATED_AT;
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::UpdatedAt => {
            let index = IX_UPDATED_AT;
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::CreatedBy => {
            let index = IX_CREATED_BY;
            let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact, None)?;
            let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::UpdatedBy => {
            let index = IX_UPDATED_BY;
            let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact, None)?;
            let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::String(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<String> = Map::new(&storage_key);
            let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact, None)?;
            let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Bool(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u8> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u8::MIN, raw_stop, u8::MAX, exact, &parse_bool)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Timestamp(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u64> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Int32(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<i32> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, i32::MIN, raw_stop, i32::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Uint8(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u8> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u8::MIN, raw_stop, u8::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Uint16(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u16> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u16::MIN, raw_stop, u16::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Uint32(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u32> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u32::MIN, raw_stop, u32::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Uint64(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u64> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Uint128(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<u128> = Map::new(&storage_key);
            let (start, stop) =
                build_start_stop(raw_start, u128::MIN, raw_stop, u128::MAX, exact, &parse)?;
            let (min, max) = build_bounds(order, partition, start, stop, query.cursor)?;
            page(index.keys(store, min, max, order), limit, &|x| {
                x.to_string()
            })?
        },
        RangeSelector::Binary(index_name) => {
            let storage_key = build_index_storage_key(index_name);
            let index: CustomIndexMap<&[u8]> = Map::new(&storage_key);
            let (start, stop) = build_start_stop_values_binary(raw_start, raw_stop, exact)?;

            #[allow(unused_assignments)]
            let mut start_vec: Vec<u8> = vec![];

            #[allow(unused_assignments)]
            let mut stop_vec: Vec<u8> = vec![];

            let (min, max) = match order {
                Order::Ascending => {
                    (
                        // min
                        if let Some((p, v_str, id)) = query.cursor {
                            start_vec = Binary::from_base64(&v_str)?.to_vec();
                            Some(Bound::Exclusive((
                                (p, start_vec.as_slice(), id.u64()),
                                PhantomData,
                            )))
                        } else if let Some(v) = start {
                            start_vec = v;
                            Some(Bound::Inclusive((
                                (partition, start_vec.as_slice(), u64::MIN),
                                PhantomData,
                            )))
                        } else {
                            None
                        },
                        // max
                        if let Some(v) = stop {
                            stop_vec = v;
                            Some(Bound::Exclusive((
                                (partition, stop_vec.as_slice(), u64::MAX),
                                PhantomData,
                            )))
                        } else {
                            None
                        },
                    )
                },
                Order::Descending => {
                    (
                        // min
                        if let Some(v) = start {
                            start_vec = v;
                            Some(Bound::Exclusive((
                                (partition, start_vec.as_slice(), u64::MIN),
                                PhantomData,
                            )))
                        } else {
                            None
                        },
                        // max
                        if let Some((p, v_str, id)) = query.cursor {
                            stop_vec = Binary::from_base64(&v_str)?.to_vec();
                            Some(Bound::Exclusive((
                                (p, stop_vec.as_slice(), id.u64()),
                                PhantomData,
                            )))
                        } else if let Some(v) = stop {
                            stop_vec = v;
                            Some(Bound::Inclusive((
                                (partition, stop_vec.as_slice(), u64::MAX),
                                PhantomData,
                            )))
                        } else {
                            None
                        },
                    )
                },
            };

            // index.prefix_range(
            //     storage,
            //     Some(PrefixBound::Exclusive((
            //         (partition, "".as_bytes()),
            //         PhantomData,
            //     ))),
            //     None,
            //     order,
            // );

            page(index.keys(store, min, max, order), limit, &|x| {
                Binary::from(x.as_slice()).to_base64()
            })?
        },
    })
}

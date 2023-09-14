use std::marker::PhantomData;
use std::str::FromStr;

use crate::msg::{Cursor, IndexName, IndexQueryParams, ReadIndexResponse};
use crate::state::{
  CONTRACT_ID_2_ADDR, IX_CODE_ID, IX_CONTRACT_ID, IX_CREATED_AT, IX_CREATED_BY, IX_REV, IX_UPDATED_AT, IX_UPDATED_BY,
};
use crate::util::{parse, parse_bool};
use crate::{error::ContractError, msg::ReadIndexParams};
use cosmwasm_std::{Addr, Api, Deps, Order, StdResult, Storage, Uint64};
use cw_storage_plus::{Bound, KeyDeserialize, Map, Prefixer, PrimaryKey};

pub fn read_index(
  deps: Deps,
  query: ReadIndexParams,
) -> Result<ReadIndexResponse, ContractError> {
  // let limit = query.limit.unwrap_or(20).clamp(1, 200) as usize;
  // let desc = query.desc.unwrap_or(false);

  // Find matching contract ID's
  let (ids, cursor) = match query.params.clone() {
    IndexQueryParams::Equals(value) => get_contract_ids(deps.api, deps.storage, query, Some(value), None, true),
    IndexQueryParams::Between(range) => get_contract_ids(deps.api, deps.storage, query, range.start, range.stop, false),
  }?;

  // Convert contract ID's to Addrs
  let addresses = load_contract_addresses(deps.storage, &ids)?;

  Ok(ReadIndexResponse {
    contracts: addresses,
    cursor,
  })
}

fn build_range_bounds<'a, T>(
  order: Order,
  partition: u16,
  range_start_value: T,
  range_stop_value: T,
  maybe_cursor: Option<Cursor>,
) -> Result<(Option<Bound<'a, (u16, T, u64)>>, Option<Bound<'a, (u16, T, u64)>>), ContractError>
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
        Some(Bound::Exclusive(((partition, range_stop_value, u64::MIN), PhantomData))),
      )
    },
    Order::Descending => {
      (
        // min
        Some(Bound::Exclusive((
          (partition, range_start_value, u64::MAX),
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
  partition: u16,
  range_start_value: Option<String>,
  range_stop_value: Option<String>,
  maybe_cursor: Option<Cursor>,
) -> Result<
  (
    Option<Bound<'a, (u16, String, u64)>>,
    Option<Bound<'a, (u16, String, u64)>>,
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
          Some(Bound::Exclusive(((partition, v, u64::MIN), PhantomData)))
        } else {
          None
        },
      )
    },
    Order::Descending => {
      (
        // min
        if let Some(v) = range_start_value {
          Some(Bound::Exclusive(((partition, v, u64::MAX), PhantomData)))
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

fn build_start_stop_values<T: FromStr + Clone>(
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
) -> Result<(Option<String>, Option<String>), ContractError> {
  if exact {
    return Ok((start_value_raw.clone(), start_value_raw));
  }

  Ok((start_value_raw, stop_value_raw))
}

fn next_page<'a, D>(
  iter: Box<dyn Iterator<Item = StdResult<(u16, D, u64)>> + 'a>,
  limit: usize,
) -> Result<(Vec<u64>, Option<Cursor>), ContractError>
where
  D: ToString,
{
  let limit = limit as usize;
  let mut contract_ids = Vec::with_capacity(limit);
  let mut cursor: Option<Cursor> = None;

  for item in iter.take(limit) {
    let (group, value, contract_id) = item?;
    cursor = Some((group, value.to_string(), Uint64::from(contract_id)));
    contract_ids.push(contract_id);
  }

  Ok((contract_ids, cursor))
}

fn get_contract_ids(
  _api: &dyn Api,
  storage: &dyn Storage,
  query: ReadIndexParams,
  raw_start: Option<String>,
  raw_stop: Option<String>,
  exact: bool,
) -> Result<(Vec<u64>, Option<Cursor>), ContractError> {
  let partition = query.partition;
  let limit = query.limit.unwrap_or(20).clamp(1, 200) as usize;
  let desc = query.desc.unwrap_or(false);
  let order = if desc { Order::Descending } else { Order::Ascending };

  Ok(match &query.index {
    IndexName::Id => {
      let index = IX_CONTRACT_ID;
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::CodeId => {
      let index = IX_CODE_ID;
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Rev => {
      let index = IX_REV;
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::CreatedAt => {
      let index = IX_CREATED_AT;
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::UpdatedAt => {
      let index = IX_UPDATED_AT;
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::CreatedBy => {
      let index = IX_CREATED_BY;
      let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact)?;
      let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::UpdatedBy => {
      let index = IX_UPDATED_BY;
      let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact)?;
      let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::String(index_name) => {
      let index: Map<(u16, String, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values_str(raw_start, raw_stop, exact)?;
      let (min, max) = build_range_bounds_str(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Bool(index_name) => {
      let index: Map<(u16, u8, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u8::MIN, raw_stop, u8::MAX, exact, &parse_bool)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Timestamp(index_name) => {
      let index: Map<(u16, u64, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Uint8(index_name) => {
      let index: Map<(u16, u8, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u8::MIN, raw_stop, u8::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Uint16(index_name) => {
      let index: Map<(u16, u16, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u16::MIN, raw_stop, u16::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Uint32(index_name) => {
      let index: Map<(u16, u32, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u32::MIN, raw_stop, u32::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Uint64(index_name) => {
      let index: Map<(u16, u64, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u64::MIN, raw_stop, u64::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
    IndexName::Uint128(index_name) => {
      let index: Map<(u16, u128, u64), u8> = Map::new(index_name.as_str());
      let (start, stop) = build_start_stop_values(raw_start, u128::MIN, raw_stop, u128::MAX, exact, &parse)?;
      let (min, max) = build_range_bounds(order, partition, start, stop, query.cursor)?;
      next_page(index.keys(storage, min, max, order), limit)?
    },
  })
}

fn load_contract_addresses(
  storage: &dyn Storage,
  contract_ids: &Vec<u64>,
) -> Result<Vec<Addr>, ContractError> {
  let mut addrs: Vec<Addr> = Vec::with_capacity(contract_ids.len());
  for id in contract_ids.iter() {
    addrs.push(CONTRACT_ID_2_ADDR.load(storage, *id)?)
  }
  Ok(addrs)
}

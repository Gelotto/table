use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, Timestamp, Uint128, Uint64};
use cw_lib::models::Owner;

use crate::error::ContractError;

pub type Cursor = (u16, String, Uint64);

#[cw_serde]
pub struct InstantiateMsg {
  pub config: Config,
}

#[cw_serde]
pub enum SudoMsg {
  Config(Config),
  Revert(),
}

#[cw_serde]
pub enum ExecuteMsg {
  Sudo(SudoMsg),
  Create(CreateParams),
  Update(UpdateParams),
  Move(Addr, u16),
}

#[cw_serde]
pub enum QueryMsg {
  Info {
    fields: Option<Vec<String>>,
    account: Option<Addr>,
  },
  Read(Query),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct SelectResponse {}

#[cw_serde]
pub struct SearchResponse {
  pub contracts: Vec<Addr>,
  pub cursor: Option<Cursor>,
}

#[cw_serde]
pub struct CreateParams {
  pub code_id: Uint64,
  pub instantiate_msg: Binary,
  pub label: Option<String>,
  pub admin: Option<Addr>,
  pub partition: u16,
  pub tags: Option<Vec<String>>,
}

#[cw_serde]
pub struct UpdateParams {
  pub initiator: Addr,
  pub values: Option<Vec<KeyValue>>,
  pub tags: Option<TagUpdates>,
}

#[cw_serde]
pub enum KeyValue {
  String(String, Option<String>),
  Bool(String, Option<bool>),
  Timestamp(String, Option<Timestamp>),
  Uint8(String, Option<u8>),
  Uint16(String, Option<u16>),
  Uint32(String, Option<u32>),
  Uint64(String, Option<Uint64>),
  Uint128(String, Option<Uint128>),
}

#[cw_serde]
pub enum IndexValue {
  String(String),
  Bool(bool),
  Timestamp(Timestamp),
  Uint8(u8),
  Uint16(u16),
  Uint32(u32),
  Uint64(Uint64),
  Uint128(Uint128),
}

#[cw_serde]
pub enum IndexValueRange {
  String(String, String),
  Bool(bool, bool),
  Timestamp(Timestamp, Timestamp),
  Uint8(u8, u8),
  Uint16(u16, u16),
  Uint32(u32, u32),
  Uint64(Uint64, Uint64),
  Uint128(Uint128, Uint128),
}

#[cw_serde]
pub enum IndexName {
  CreatedAt,
  UpdatedAt,
  CreatedBy,
  UpdatedBy,
  CodeId,
  Id,
  Rev,
  String(String),
  Bool(String),
  Timestamp(String),
  Uint8(String),
  Uint16(String),
  Uint32(String),
  Uint64(String),
  Uint128(String),
}

#[cw_serde]
pub enum IndexType {
  String,
  Bool,
  Timestamp,
  Uint8,
  Uint16,
  Uint32,
  Uint64,
  Uint128,
}

#[cw_serde]
pub struct TagUpdates {
  pub remove: Vec<String>,
  pub add: Vec<String>,
}

#[cw_serde]
pub struct Config {
  pub owner: Owner,
  pub code_id_allowlist_enabled: bool,
}

impl Config {
  pub fn validate(
    &self,
    api: &dyn Api,
  ) -> Result<(), ContractError> {
    api.addr_validate(self.owner.to_addr().as_str())?;
    Ok(())
  }
}

#[cw_serde]
pub struct Range {
  pub start: Option<String>,
  pub stop: Option<String>,
}

#[cw_serde]
pub enum QueryParams {
  Equals(String),
  Between(Range),
  // Tags(Vec<String>),
}

#[cw_serde]
pub struct Query {
  pub index: IndexName,
  pub partition: u16,
  pub params: QueryParams,
  pub desc: Option<bool>,
  pub limit: Option<u32>,
  pub cursor: Option<Cursor>,
}

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
pub enum ConfigMsg {
  Update(Config),
  Revert(),
}

#[cw_serde]
pub struct FlagParams {
  pub contract: Addr,
  pub suspend: Option<bool>,
  pub reason: Option<String>,
  pub code: Option<u32>,
}

#[cw_serde]
pub enum ExecuteMsg {
  Config(ConfigMsg),
  Create(CreationParams),
  Update(UpdateParams),
  Delete(Addr),
  Move(Addr, u16),
  Flag(FlagParams),
  Unsuspend(Addr),
  CreateIndex(IndexCreationParams),
  DeleteIndex(String),
}

#[cw_serde]
pub enum ReadMsg {
  Index(ReadIndexParams),
  Tags(ReadTagsParams),
  Relationships(ReadRelationshipsParams),
}

#[cw_serde]
pub enum QueryMsg {
  Metadata {},
  Read(ReadMsg),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct MetadataResponse {}

#[cw_serde]
pub struct ReadIndexResponse {
  pub contracts: Vec<Addr>,
  pub cursor: Option<Cursor>,
}

#[cw_serde]
pub struct ReadTagsResponse {
  pub contracts: Vec<Vec<Addr>>,
  pub cursors: Vec<Option<Uint64>>,
}

#[cw_serde]
pub struct Relationship {
  pub rel: String,
  pub address: Addr,
}

#[cw_serde]
pub struct ReadRelationshipsResponse {
  pub relationships: Vec<Relationship>,
  pub cursor: Option<(String, String)>,
}

#[cw_serde]
pub struct CreationParams {
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
  pub contract: Option<Addr>,
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
pub struct IndexMetadata {
  pub index_type: IndexType,
  pub name: String,
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
pub struct ReadTagsParams {
  pub tags: Vec<String>,
  pub cursors: Option<Vec<Uint64>>,
  pub desc: Option<bool>,
  pub limit: Option<u32>,
  pub partition: u16,
}

#[cw_serde]
pub enum RelationshipSide {
  Contract(Addr),
  Account(Addr),
}

#[cw_serde]
pub struct ReadRelationshipsParams {
  pub side: RelationshipSide,
  pub names: Option<Vec<String>>,
  pub cursor: Option<(String, String)>,
  pub desc: Option<bool>,
  pub limit: Option<u32>,
  pub partition: u16,
}

#[cw_serde]
pub struct IndexCreationParams {
  pub index_type: IndexType,
  pub name: String,
}

#[cw_serde]
pub enum IndexQueryParams {
  Equals(String),
  Between(Range),
  // Tags(TagQueryParams),
}

#[cw_serde]
pub struct ReadIndexParams {
  pub index: IndexName,
  pub partition: u16,
  pub params: IndexQueryParams,
  pub desc: Option<bool>,
  pub limit: Option<u32>,
  pub cursor: Option<Cursor>,
}

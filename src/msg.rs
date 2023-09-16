use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, Timestamp, Uint128, Uint64};
use cw_lib::models::Owner;

use crate::{
  error::ContractError,
  models::{ContractMetadataView, Details},
  state::{GroupID, PartitionID},
};

pub type Cursor = (PartitionID, String, Uint64);

#[cw_serde]
pub struct InstantiateMsg {
  pub config: Config,
}

#[cw_serde]
pub enum AdminMsg {
  UpdateInfo(TableInfo),
  SetPartition(Addr, PartitionSelector),
  SetGroups(Addr, GroupUpdates),
  UpdateConfig(Config),
  RevertConfig(),
  Unsuspend(Addr),
  CreatePartition(PartitionCreationParams),
  CreateIndex(IndexCreationParams),
  DeleteIndex(String),
}

#[cw_serde]
pub struct GroupUpdates {
  pub remove: Option<Vec<GroupSelector>>,
  pub add: Option<Vec<GroupSelector>>,
}

#[cw_serde]
pub struct FlagParams {
  pub contract: Addr,
  pub suspend: Option<bool>,
  pub reason: Option<String>,
  pub code: Option<u32>,
}

#[cw_serde]
pub enum ClientMsg {
  Create(CreationParams),
  Update(UpdateParams),
  Delete(Addr),
  Flag(FlagParams),
}

#[cw_serde]
pub enum ExecuteMsg {
  Admin(AdminMsg),
  Client(ClientMsg),
}

#[cw_serde]
pub enum ReadMsg {
  Index(ReadIndexParams),
  Tags(ReadTagsParams),
  Relationships(ReadRelationshipsParams),
}

#[cw_serde]
pub enum QueryMsg {
  Read(ReadMsg),
  Indices {
    cursor: Option<String>,
    desc: Option<bool>,
  },
  Partitions {
    cursor: Option<PartitionID>,
    desc: Option<bool>,
  },
  Groups {
    cursor: Option<GroupID>,
    desc: Option<bool>,
  },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct IndicesResponse {
  pub indices: Vec<IndexMetadata>,
  pub cursor: Option<String>,
}

#[cw_serde]
pub struct GroupsResponse {
  pub groups: Vec<GroupMetadataView>,
  pub cursor: Option<GroupID>,
}

#[cw_serde]
pub struct TagCount {
  pub tag: String,
  pub count: u32,
}

#[cw_serde]
pub struct PartitionView {
  pub id: PartitionID,
  pub size: Uint64,
  pub description: Option<String>,
  pub name: String,
}

#[cw_serde]
pub struct PartitionsResponse {
  pub partitions: Vec<PartitionView>,
  pub cursor: Option<PartitionID>,
}

#[cw_serde]
pub struct ContractRecord {
  pub address: Addr,
  pub meta: Option<ContractMetadataView>,
}

#[cw_serde]
pub struct ReadIndexResponse {
  pub contracts: Vec<ContractRecord>,
  pub cursor: Option<Cursor>,
}

#[cw_serde]
pub struct ReadTagsResponse {
  pub contracts: Vec<Vec<Addr>>,
  pub cursors: Vec<Option<Uint64>>,
}

#[cw_serde]
pub struct Relationship {
  pub name: String,
  pub address: Addr,
}

#[cw_serde]
pub struct ReadRelationshipsResponse {
  pub relationships: Vec<Relationship>,
  pub cursor: Option<(String, String)>,
}

#[cw_serde]
pub struct CreationParams {
  // Downstream instantiation params
  pub code_id: Uint64,
  pub instantiate_msg: Binary,
  pub admin: Option<Addr>,
  // Internal contract params
  pub partition: PartitionSelector,
  pub label: Option<String>,
  pub groups: Option<Vec<GroupSelector>>,
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
pub enum PartitionSelector {
  Id(PartitionID),
  Name(String),
}

#[cw_serde]
pub struct PartitionMetadata {
  pub name: String,
  pub description: Option<String>,
}

#[cw_serde]
pub struct TableInfo {
  pub name: Option<String>,
  pub description: Option<String>,
}

#[cw_serde]
pub struct IndexMetadata {
  pub index_type: IndexType,
  pub name: String,
  pub size: Uint64,
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

#[cw_serde]
pub enum GroupSelector {
  Id(GroupID),
  Name(String),
}

#[cw_serde]
pub struct GroupMetadata {
  pub name: String,
  pub created_at: Timestamp,
  pub description: Option<String>,
  pub size: Uint64,
}

#[cw_serde]
pub struct GroupMetadataView {
  pub id: u32,
  pub name: String,
  pub created_at: Timestamp,
  pub description: Option<String>,
  pub size: Uint64,
}

#[cw_serde]
pub struct GroupCreationParams {
  pub name: Option<String>,
  pub description: Option<String>,
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
  pub partition: PartitionID,
  pub details: Option<Details>,
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
pub struct PartitionCreationParams {
  pub name: Option<String>,
  pub description: Option<String>,
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
}

#[cw_serde]
pub struct ReadIndexParams {
  pub index: IndexName,
  pub partition: PartitionID,
  pub params: IndexQueryParams,
  pub desc: Option<bool>,
  pub limit: Option<u32>,
  pub cursor: Option<Cursor>,
  pub details: Option<Details>,
}

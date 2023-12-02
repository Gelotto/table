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
    pub info: TableInfo,
    pub config: Config,
    pub partitions: Option<Vec<PartitionCreationParams>>,
    pub groups: Option<Vec<GroupCreationParams>>,
    pub indices: Option<Vec<IndexCreationParams>>,
}

#[cw_serde]
pub enum AdminMsg {
    CreateGroup(GroupCreationParams),
    CreatePartition(PartitionCreationParams),
    CreateIndex(IndexCreationParams),
    UpdateInfo(TableInfo),
    SetPartition(Addr, PartitionSelector),
    AssignGroups(GroupUpdates),
    UpdateConfig(Config),
    RevertConfig(),
    Unsuspend(Addr),
    DeleteIndex(String),
}

#[cw_serde]
pub enum ClientMsg {
    Create(CreationParams),
    Update(UpdateParams),
    Delete(Addr),
    Flag(FlagParams),
}

#[cw_serde]
pub struct GroupUpdates {
    pub contract: Addr,
    pub remove: Option<Vec<GroupID>>,
    pub add: Option<Vec<GroupID>>,
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
    Admin(AdminMsg),
    Client(ClientMsg),
}

#[cw_serde]
pub enum ContractsQueryMsg {
    Range(RangeQueryParams),
    WithTag(TagQueryParams),
    InGroup(GroupQueryParams),
    ByAddresses(AddressesQueryParams),
    RelatedTo(RelationshipQueryParams),
}

#[cw_serde]
pub enum TableQueryMsg {
    Indices(TableIndicesQueryParams),
    Partitions(TablePartitionsQueryParams),
    Groups(TableGroupsQueryParams),
    Tags(TableTagsQueryParams),
}

#[cw_serde]
pub enum ContractQueryMsg {
    Relationships(ContractRelationshipsQueryParams),
    Groups(ContractGroupsQueryParams),
    Tags(ContractTagsQueryParams),
}

#[cw_serde]
pub enum QueryMsg {
    Table(TableQueryMsg),
    Contracts(ContractsQueryMsg),
    Contract(ContractQueryMsg),
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
pub struct TagsResponse {
    pub tags: Vec<TagCount>,
    pub cursor: Option<String>,
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
pub struct ContractsByAddressResponse {
    pub contracts: Vec<ContractRecord>,
    pub cursor: Option<u32>,
}

#[cw_serde]
pub struct ContractsRangeResponse {
    pub contracts: Vec<ContractRecord>,
    pub cursor: Option<Cursor>,
}

#[cw_serde]
pub struct ContractsByTagResponse {
    pub contracts: Vec<ContractRecord>,
    pub cursor: Option<Uint64>,
}

#[cw_serde]
pub struct ContractsByGroupResponse {
    pub contracts: Vec<ContractRecord>,
    pub cursor: Option<Uint64>,
}

#[cw_serde]
pub struct FullRelationship {
    pub contract: Addr,
    pub name: String,
    pub address: Addr,
}

#[cw_serde]
pub struct RelationshipAddresses {
    pub name: String,
    pub addresses: Vec<Addr>,
}

#[cw_serde]
pub struct Relationship {
    pub name: String,
    pub address: Addr,
}

#[cw_serde]
pub struct RelatedContract {
    pub relationships: Vec<String>,
    pub contract: ContractRecord,
}

#[cw_serde]
pub struct ReadRelationshipResponse {
    pub contracts: Vec<RelatedContract>,
    pub cursor: Option<(String, String)>,
}

#[cw_serde]
pub struct ContractGroupsResponse {
    pub groups: Vec<GroupMetadataView>,
    pub cursor: Option<GroupID>,
}

#[cw_serde]
pub struct ContractTagsResponse {
    pub tags: Vec<String>,
    pub cursor: Option<String>,
}

#[cw_serde]
pub struct ContractRelationshipsResponse {
    pub relationships: Vec<RelationshipAddresses>,
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
    pub groups: Option<Vec<GroupID>>,
    pub tags: Option<Vec<String>>,
}

#[cw_serde]
pub struct UpdateParams {
    pub contract: Addr,
    pub initiator: Addr,
    pub values: Option<Vec<KeyValue>>,
    pub tags: Option<TagUpdates>,
    pub relationships: Option<RelationshipUpdates>,
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
    Binary(String, Option<Binary>),
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
    Binary(Binary),
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
    Binary(Binary, Binary),
}

#[cw_serde]
pub enum RangeSelector {
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
    Binary(String),
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
    Binary,
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
    pub created_at: Timestamp,
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
#[derive(Default)]
pub struct TagUpdates {
    pub remove: Option<Vec<String>>,
    pub add: Option<Vec<String>>,
}

#[cw_serde]
pub struct RelationshipUpdates {
    pub remove: Option<Vec<Relationship>>,
    pub add: Option<Vec<Relationship>>,
}

#[cw_serde]
pub struct Config {
    pub owner: Owner,
    pub code_id_allowlist_enabled: bool,
}

#[cw_serde]
pub struct GroupMetadata {
    pub name: String,
    pub created_by: Addr,
    pub created_at: Timestamp,
    pub description: Option<String>,
    pub size: Uint64,
}

#[cw_serde]
pub struct GroupMetadataView {
    pub id: GroupID,
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
pub struct ContractGroupsQueryParams {
    pub contract: Addr,
    pub cursor: Option<GroupID>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
}

#[cw_serde]
pub struct ContractTagsQueryParams {
    pub contract: Addr,
    pub cursor: Option<String>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
}

#[cw_serde]
pub struct ContractRelationshipsQueryParams {
    pub contract: Addr,
    pub name: Option<String>,
    pub cursor: Option<(String, String)>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
}

#[cw_serde]
pub struct GroupQueryParams {
    pub group: GroupID,
    pub cursor: Option<Uint64>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
    pub details: Option<Details>,
}

#[cw_serde]
pub struct AddressesQueryParams {
    pub contracts: Vec<Addr>,
    pub cursor: Option<u32>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
    pub details: Option<Details>,
}

#[cw_serde]
pub struct TagQueryParams {
    pub tag: String,
    pub cursor: Option<Uint64>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
    pub details: Option<Details>,
}

#[cw_serde]
pub struct TableTagsQueryParams {
    pub cursor: Option<String>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
}

#[cw_serde]
pub enum GroupSelector {
    WithName(String),
    CreatedBetween(Timestamp, Timestamp),
}

#[cw_serde]
pub struct TableGroupsQueryParams {
    pub select: Option<GroupSelector>,
    pub cursor: Option<Vec<String>>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
}

#[cw_serde]
pub struct TablePartitionsQueryParams {
    pub cursor: Option<String>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub struct TableIndicesQueryParams {
    pub cursor: Option<String>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub enum RelationshipSide {
    Contract(Addr),
    Account(Addr),
}

#[cw_serde]
pub struct RelationshipQueryParams {
    pub address: Addr,
    pub cursor: Option<(String, String)>,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub partition: PartitionID,
    pub details: Option<Details>,
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
pub struct RangeQueryParams {
    pub index: RangeSelector,
    pub partition: PartitionID,
    pub params: IndexQueryParams,
    pub desc: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<Cursor>,
    pub details: Option<Details>,
}

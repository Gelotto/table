use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint64};

use crate::msg::CreationParams;

#[cw_serde]
pub struct ContractFlag {
  pub sender: Addr,
  pub reason: Option<String>,
  pub code: Option<u32>,
  pub height: Uint64,
  pub time: Timestamp,
}

#[cw_serde]
pub struct ContractMetadata {
  pub id: Uint64,
  pub code_id: Uint64,
  pub created_by: Addr,
  pub created_at: Timestamp,
  pub created_at_height: Uint64,
  pub is_managed: bool,
  pub partition: u16,
}

#[cw_serde]
pub struct ContractMetadataViewDetails {
  pub id: Uint64,
  pub is_managed: bool,
  pub code_id: Uint64,
  pub created_by: Addr,
  pub created_at_height: Uint64,
  pub updated_at_height: Uint64,
  pub updated_by: Addr,
}

#[cw_serde]
pub struct ContractMetadataView {
  pub partition: u16,
  pub created_at: Timestamp,
  pub updated_at: Timestamp,
  pub rev: Uint64,
  pub details: Option<ContractMetadataViewDetails>,
}

#[cw_serde]
pub struct DynamicContractMetadata {
  pub rev: Uint64,
  pub updated_at: Timestamp,
  pub updated_at_height: Uint64,
  pub updated_by: Addr,
}

#[cw_serde]
pub enum ReplyJob {
  Create { params: CreationParams, initiator: Addr },
}

#[cw_serde]
pub enum Verbosity {
  Basic,
  Full,
}

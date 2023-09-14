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
  pub initiator: Addr,
  pub time: Timestamp,
  pub height: Uint64,
  pub is_managed: bool,
  pub partition: u16,
}

#[cw_serde]
pub struct DynamicContractMetadata {
  pub rev: Uint64,
  pub time: Timestamp,
  pub height: Uint64,
  pub initiator: Addr,
}

#[cw_serde]
pub enum ReplyJob {
  Create { params: CreationParams, initiator: Addr },
}

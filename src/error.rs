use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("GenericError: {reason:?}")]
  GenericError { reason: String },

  #[error("ContractNotFound: {reason:?}")]
  ContractNotFound { reason: String },

  #[error("UnexpectedError: {reason:?}")]
  UnexpectedError { reason: String },

  #[error("JobNotFound: {reason:?}")]
  JobNotFound { reason: String },

  #[error("PartitionNotFound: {reason:?}")]
  PartitionNotFound { reason: String },

  #[error("NotAuthorized: {reason:?}")]
  NotAuthorized { reason: String },

  #[error("ValidationError: {reason:?}")]
  ValidationError { reason: String },

  #[error("CreateError: {reason:?}")]
  CreateError { reason: String },

  #[error("ContractSuspended: contract {contract_addr:?} has been flagged and suspended")]
  ContractSuspended { contract_addr: Addr },
}

impl From<ContractError> for StdError {
  fn from(err: ContractError) -> Self {
    StdError::generic_err(err.to_string())
  }
}

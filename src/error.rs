use cosmwasm_std::StdError;
use thiserror::Error;

use crate::state::ContractID;

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

    #[error("GroupNotFound: {reason:?}")]
    GroupNotFound { reason: String },

    #[error("NotAuthorized: {reason:?}")]
    NotAuthorized { reason: String },

    #[error("ValidationError: {reason:?}")]
    ValidationError { reason: String },

    #[error("CreateError: {reason:?}")]
    CreateError { reason: String },

    #[error("ContractSuspended: contract {contract_id:?} has been flagged and suspended")]
    ContractSuspended { contract_id: ContractID },

    #[error("InvalidCursor: {reason:?}")]
    InvalidCursor { reason: String },

    #[error("UnexpectedReplyJobType")]
    UnexpectedReplyJobType,
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> Self {
        StdError::generic_err(err.to_string())
    }
}

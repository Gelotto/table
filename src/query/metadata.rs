use crate::error::ContractError;
use crate::msg::MetadataResponse;
use cosmwasm_std::Deps;

pub fn metadata(_deps: Deps) -> Result<MetadataResponse, ContractError> {
  Ok(MetadataResponse {})
}

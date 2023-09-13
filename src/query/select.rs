use crate::error::ContractError;
use crate::msg::SelectResponse;
use cosmwasm_std::{Addr, Deps};

pub fn select(
  _deps: Deps,
  _fields: Option<Vec<String>>,
  _account: Option<Addr>,
) -> Result<SelectResponse, ContractError> {
  Ok(SelectResponse {})
}

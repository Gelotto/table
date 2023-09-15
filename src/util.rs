use std::str::FromStr;

use crate::error::ContractError;

pub fn parse<T: FromStr>(v_str: String) -> Result<T, ContractError> {
  match v_str.parse::<T>() {
    Ok(v) => Ok(v),
    Err(_) => Err(ContractError::ValidationError {
      reason: format!("cannot parse value: {}", v_str),
    }),
  }
}

pub fn parse_bool(s: String) -> Result<u8, ContractError> {
  Ok(if s == "true" {
    1
  } else if s == "false" {
    0
  } else {
    parse(s)?
  })
}

pub fn build_index_name(name: &String) -> String {
  format!("_ix_{}", name)
}

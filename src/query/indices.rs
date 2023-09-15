use crate::error::ContractError;
use crate::msg::{IndexMetadata, IndicesResponse};
use crate::state::INDEX_METADATA;
use cosmwasm_std::{Deps, Order};

/// Return custom index metadata records, created via create_index.
pub fn query_indices(deps: Deps) -> Result<IndicesResponse, ContractError> {
  let mut indices: Vec<IndexMetadata> = Vec::with_capacity(4);

  for result in INDEX_METADATA.range(deps.storage, None, None, Order::Ascending) {
    let (_, meta) = result?;
    indices.push(meta);
  }

  Ok(IndicesResponse(indices))
}

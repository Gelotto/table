use cosmwasm_std::Response;
use cw_storage_plus::Map;

use crate::{
    context::Context,
    error::ContractError,
    msg::IndexType,
    state::{ensure_allowed_by_acl, ContractID, PartitionID, INDEX_METADATA},
    util::build_index_storage_key,
};

pub fn on_execute(
    ctx: Context,
    index_name: String,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;

    ensure_allowed_by_acl(&deps, &info.sender, "/table/delete-index")?;

    if let Some(meta) = INDEX_METADATA.may_load(deps.storage, index_name.clone())? {
        INDEX_METADATA.remove(deps.storage, index_name.clone());

        let map_name = &build_index_storage_key(&index_name);

        match meta.index_type {
            IndexType::String => {
                let map: Map<(PartitionID, &String, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Bool => {
                let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Timestamp => {
                let map: Map<(PartitionID, u64, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Int32 => {
                let map: Map<(PartitionID, i32, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Uint8 => {
                let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Uint16 => {
                let map: Map<(PartitionID, u8, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Uint32 => {
                let map: Map<(PartitionID, u32, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Uint64 => {
                let map: Map<(PartitionID, u64, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Uint128 => {
                let map: Map<(PartitionID, u128, ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
            IndexType::Binary => {
                let map: Map<(PartitionID, &[u8], ContractID), u8> = Map::new(map_name);
                map.clear(deps.storage);
            },
        };
    } else {
        return Err(ContractError::NotAuthorized {
            reason: format!("index metadata does not exist for '{}'", index_name),
        });
    }
    Ok(Response::new().add_attribute("action", "delete_index"))
}

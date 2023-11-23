use cosmwasm_std::Storage;

use crate::{
    error::ContractError,
    state::{CODE_ID_ALLOWLIST, CONFIG_CODE_ID_ALLOWLIST_ENABLED},
};

pub fn ensure_authorized_code_id(
    storage: &dyn Storage,
    code_id: u64,
) -> Result<(), ContractError> {
    if CONFIG_CODE_ID_ALLOWLIST_ENABLED.load(storage)? {
        if !CODE_ID_ALLOWLIST.has(storage, code_id.into()) {
            return Err(ContractError::NotAuthorized {
                reason: format!("code ID {} not allowed", code_id),
            });
        }
    }
    Ok(())
}

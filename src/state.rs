use crate::models::{DynamicContractMetadata, ReplyJob};
use crate::msg::{Config, IndexType, InstantiateMsg};
use crate::{error::ContractError, models::ContractMetadata};
use cosmwasm_std::{to_binary, Addr, Binary, DepsMut, Env, MessageInfo, Storage, Timestamp, Uint128, Uint64};
use cw_lib::models::Owner;
use cw_storage_plus::{Item, Map};

pub type IndexMap<K> = Map<'static, K, u8>;

// Marker/dummy value for IndexMap values
pub const X: u8 = 1;

// Table contract config settings:
pub const CONFIG_OWNER: Item<Owner> = Item::new("owner");
pub const CONFIG_CODE_ID_ALLOWLIST_ENABLED: Item<bool> = Item::new("code_id_allowlist_enabled");
pub const CONFIG_BACKUP: Item<Binary> = Item::new("config_backup");

// Contract ID-related data. Each new contract ID increments the counter, and
// the two maps map Addr <-> u64 ID.
pub const CONTRACT_ID_COUNTER: Item<Uint64> = Item::new("contract_id_counter");
pub const CONTRACT_ID_2_ADDR: Map<u64, Addr> = Map::new("contract_id_2_addr");
pub const CONTRACT_ADDR_2_ID: Map<&Addr, Uint64> = Map::new("contract_addr_2_id");

// Created contract metadata generated through the create API, like its ID,
// block info at the time, and the initiator (i.e. the account that called
// create()).
pub const METADATA: Map<u64, ContractMetadata> = Map::new("metadata");

// Contract metadata that changes on each update to any of its indices.
pub const DYNAMIC_METADATA: Map<u64, DynamicContractMetadata> = Map::new("dynamic_metadata");

// Jobs for processing in the cw reply entrypoint
pub const REPLY_JOBS: Map<u64, ReplyJob> = Map::new("reply_jobs");
pub const REPLY_JOB_ID_COUNTER: Item<Uint64> = Item::new("reply_job_id_counter");

// Allow list, where the keys are the Code ID's that can be instantiated through
// the create() API. Only used if the allowlist is enabled through config.
pub const CODE_ID_ALLOWLIST: IndexMap<u64> = Map::new("code_id_allowlist");

// Each contract can be associated with many tags. TAG_COUNTS records the total
// number of contracts with which each tag is associated.
pub const TAG_COUNTS: Map<&String, u32> = Map::new("tag_counts");

// Lookup table for finding names/keys of indexed values for a given contract ID
pub const INDEXED_KEYS: Map<(u64, &String), IndexType> = Map::new("indexed_keys");

// INDEX_* are built-in index maps owned and managed by this contract.
pub const INDEX_CONTRACT_ID: IndexMap<(u16, u64, u64)> = Map::new("ix_contract_id");
pub const INDEX_CODE_ID: IndexMap<(u16, u64, u64)> = Map::new("ix_code_id");
pub const INDEX_CREATED_BY: IndexMap<(u16, String, u64)> = Map::new("ix_created_by");
pub const INDEX_CREATED_AT: IndexMap<(u16, u64, u64)> = Map::new("ix_created_at");
pub const INDEX_UPDATED_AT: IndexMap<(u16, u64, u64)> = Map::new("ix_updated");
pub const INDEX_UPDATED_BY: IndexMap<(u16, String, u64)> = Map::new("ix_updated_by");
pub const INDEX_REV: IndexMap<(u16, u64, u64)> = Map::new("ix_created_by");
pub const INDEX_TAG: IndexMap<(u16, &String, u64)> = Map::new("ix_tag");

// Lookup tables for current value of a given key, indexed for a given contract
// ID. For example, if a contract's "color" string is indexed, supposing that
// the contract ID is 1, We'd expect that the VALUES_STRING map contains the
// entry: (1, "color") => "red".
pub const VALUES_STRING: Map<(u64, &String), String> = Map::new("values_string");
pub const VALUES_BOOL: Map<(u64, &String), bool> = Map::new("values_bool");
pub const VALUES_TIME: Map<(u64, &String), Timestamp> = Map::new("values_time");
pub const VALUES_U8: Map<(u64, &String), u8> = Map::new("values_u8");
pub const VALUES_U16: Map<(u64, &String), u16> = Map::new("values_u16");
pub const VALUES_U32: Map<(u64, &String), u32> = Map::new("values_u32");
pub const VALUES_U64: Map<(u64, &String), Uint64> = Map::new("values_u64");
pub const VALUES_U128: Map<(u64, &String), Uint128> = Map::new("values_u128");

pub fn initialize(
  deps: DepsMut,
  _env: &Env,
  _info: &MessageInfo,
  msg: &InstantiateMsg,
) -> Result<(), ContractError> {
  deps.api.addr_validate(msg.config.owner.to_addr().as_str())?;
  CONFIG_OWNER.save(deps.storage, &msg.config.owner)?;
  CONFIG_CODE_ID_ALLOWLIST_ENABLED.save(deps.storage, &msg.config.code_id_allowlist_enabled)?;
  CONTRACT_ID_COUNTER.save(deps.storage, &Uint64::zero())?;
  REPLY_JOB_ID_COUNTER.save(deps.storage, &Uint64::zero())?;
  Ok(())
}

pub fn save_config(
  storage: &mut dyn Storage,
  config: &Config,
) -> Result<(), ContractError> {
  // Load and save existing config as backup. This can be restored by the
  // updated owner by executing the Restore msg.
  let prev_config = load_config(storage)?;

  CONFIG_BACKUP.save(storage, &to_binary(&prev_config)?)?;

  // Overwrite existing config settings with new ones
  CONFIG_OWNER.save(storage, &config.owner)?;
  CONFIG_CODE_ID_ALLOWLIST_ENABLED.save(storage, &config.code_id_allowlist_enabled)?;
  Ok(())
}

pub fn load_config(storage: &dyn Storage) -> Result<Config, ContractError> {
  Ok(Config {
    owner: CONFIG_OWNER.load(storage)?,
    code_id_allowlist_enabled: CONFIG_CODE_ID_ALLOWLIST_ENABLED.load(storage)?,
  })
}

pub fn load_reply_job(
  storage: &dyn Storage,
  job_id: u64,
) -> Result<ReplyJob, ContractError> {
  if let Some(job) = REPLY_JOBS.may_load(storage, job_id)? {
    Ok(job)
  } else {
    Err(ContractError::JobNotFound {
      reason: format!("Create msg job {} not found", job_id),
    })
  }
}

pub fn load_contract_id(
  storage: &dyn Storage,
  contract_addr: &Addr,
) -> Result<u64, ContractError> {
  if let Some(id) = CONTRACT_ADDR_2_ID.may_load(storage, contract_addr)? {
    Ok(id.into())
  } else {
    Err(ContractError::NotAuthorized {
      reason: "Unrecognized contract address".to_owned(),
    })
  }
}

pub fn load_next_contract_id(
  storage: &mut dyn Storage,
  contract_addr: &Addr,
) -> Result<u64, ContractError> {
  // Make sure that the contract doesn't already exist.
  if CONTRACT_ADDR_2_ID.has(storage, contract_addr) {
    return Err(ContractError::NotAuthorized {
      reason: "address already exists".to_owned(),
    });
  }
  // Increment and return the ID counter. This is the new Id.
  let contract_id: u64 = CONTRACT_ID_COUNTER
    .update(storage, |counter| -> Result<_, ContractError> {
      Ok(counter + Uint64::one())
    })?
    .into();

  CONTRACT_ADDR_2_ID.save(storage, contract_addr, &contract_id.into())?;
  CONTRACT_ID_2_ADDR.save(storage, contract_id.into(), contract_addr)?;

  Ok(contract_id)
}

use cosmwasm_std::{Response, Storage, Uint64};

use crate::{
    context::Context,
    error::ContractError,
    msg::{GroupCreationParams, GroupMetadata},
    state::{
        ensure_allowed_by_acl, GroupID, GROUP_ID_COUNTER, GROUP_IX_CREATED_AT, GROUP_IX_NAME,
        GROUP_METADATA, X,
    },
};

pub fn on_execute(
    ctx: Context,
    params: GroupCreationParams,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let action = "create_groups";

    ensure_allowed_by_acl(&deps, &info.sender, "/table/create-groups")?;

    let group_id = increment_next_group_id(deps.storage)?;
    let name = params.name.unwrap_or_else(|| group_id.to_string());

    GROUP_METADATA.save(
        deps.storage,
        group_id,
        &GroupMetadata {
            name: name.clone(),
            description: params.description,
            created_by: info.sender.clone(),
            created_at: env.block.time,
            size: Uint64::zero(),
        },
    )?;

    GROUP_IX_NAME.save(deps.storage, (name.clone(), group_id), &X)?;
    GROUP_IX_CREATED_AT.save(deps.storage, (env.block.time.nanos(), group_id), &X)?;

    Ok(Response::new().add_attribute("action", action))
}

fn increment_next_group_id(storage: &mut dyn Storage) -> Result<GroupID, ContractError> {
    GROUP_ID_COUNTER.update(storage, |n| -> Result<_, ContractError> {
        n.checked_add(1)
            .ok_or_else(|| ContractError::UnexpectedError {
                reason: "unexpected overflow incrementing group ID counter".to_owned(),
            })
    })
}

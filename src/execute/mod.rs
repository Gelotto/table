use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub mod admin;
pub mod client;

pub struct Context<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

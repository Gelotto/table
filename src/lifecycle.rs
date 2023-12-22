use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct LifecycleSetupArgs {
    pub table: Addr,
    pub initiator: Addr,
    pub id: String,
}

#[cw_serde]
pub struct LifecycleArgs {
    pub table: Addr,
    pub initiator: Addr,
}

#[cw_serde]
pub enum LifecycleExecuteMsg {
    Setup(LifecycleSetupArgs),
    Teardown(LifecycleArgs),
    Suspend(LifecycleArgs),
    Resume(LifecycleArgs),
}

#[cw_serde]
pub enum LifecycleExecuteMsgEnvelope {
    Lifecycle(LifecycleExecuteMsg),
}

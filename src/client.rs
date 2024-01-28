use cosmwasm_std::{to_json_binary, Addr, Binary, StdResult, Uint64, WasmMsg};

use crate::{
    msg::{
        ClientMsg, CreationParams, ExecuteMsg, FlagParams, KeyValue, PartitionSelector,
        Relationship, RelationshipUpdates, TagUpdate, TagUpdates, UpdateParams,
    },
    state::GroupID,
};

pub struct Table {
    client_addr: Addr,
    table_addr: Addr,
}

impl Table {
    pub fn new(
        table: &Addr,  // Remote Table contract address
        client: &Addr, // Usually this contract's env.contract.address
    ) -> Self {
        Self {
            client_addr: client.clone(),
            table_addr: table.clone(),
        }
    }

    pub fn create(
        &self,
        code_id: Uint64,
        instantiate_msg: Binary,
        label: String,
        use_lifecycle_hooks: bool,
        partition: PartitionSelector,
        admin: Option<Addr>,
        groups: Option<Vec<GroupID>>,
        tags: Option<Vec<String>>,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.table_addr.clone().into(),
            msg: to_json_binary(&ExecuteMsg::Client(ClientMsg::Create(CreationParams {
                code_id,
                instantiate_msg,
                label: Some(label),
                use_lifecycle_hooks: Some(use_lifecycle_hooks),
                admin,
                partition,
                groups,
                tags,
            })))?,
            funds: vec![],
        })
    }

    pub fn update(
        &self,
        initiator: &Addr,
        values: Option<Vec<KeyValue>>,
        tags: Option<TagUpdates>,
        relationships: Option<RelationshipUpdates>,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.table_addr.clone().into(),
            msg: to_json_binary(&ExecuteMsg::Client(ClientMsg::Update(UpdateParams {
                contract: self.client_addr.clone(),
                initiator: initiator.clone(),
                values,
                tags,
                relationships,
            })))?,
            funds: vec![],
        })
    }

    pub fn delete(&self) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.table_addr.clone().into(),
            msg: to_json_binary(&ExecuteMsg::Client(ClientMsg::Delete(
                self.client_addr.clone(),
            )))?,
            funds: vec![],
        })
    }

    pub fn tag(
        &self,
        initiator: &Addr,
        tags: Vec<TagUpdate>,
    ) -> StdResult<WasmMsg> {
        self.update(
            initiator,
            None,
            Some(TagUpdates {
                add: Some(tags),
                remove: None,
            }),
            None,
        )
    }

    pub fn untag(
        &self,
        initiator: &Addr,
        tags: Vec<String>,
    ) -> StdResult<WasmMsg> {
        self.update(
            initiator,
            None,
            Some(TagUpdates {
                remove: Some(tags),
                add: None,
            }),
            None,
        )
    }

    pub fn index(
        &self,
        initiator: &Addr,
        values: Vec<KeyValue>,
    ) -> StdResult<WasmMsg> {
        self.update(initiator, Some(values), None, None)
    }

    pub fn associate(
        &self,
        initiator: &Addr,
        relationships: Vec<Relationship>,
    ) -> StdResult<WasmMsg> {
        self.update(
            initiator,
            None,
            None,
            Some(RelationshipUpdates {
                remove: None,
                add: Some(relationships),
            }),
        )
    }

    pub fn disocciate(
        &self,
        initiator: &Addr,
        relationships: Vec<Relationship>,
    ) -> StdResult<WasmMsg> {
        self.update(
            initiator,
            None,
            None,
            Some(RelationshipUpdates {
                add: None,
                remove: Some(relationships),
            }),
        )
    }

    pub fn flag(
        &self,
        contract: Option<Addr>,
        reason: Option<String>,
        code: Option<u32>,
        suspend: bool,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.table_addr.clone().into(),
            msg: to_json_binary(&ExecuteMsg::Client(ClientMsg::Flag(FlagParams {
                contract: contract.unwrap_or(self.client_addr.clone()),
                suspend: Some(suspend),
                reason,
                code,
            })))?,
            funds: vec![],
        })
    }
}

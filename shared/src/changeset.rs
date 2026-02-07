use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changeset {
    pub version: u32,
    pub description: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    AddTable {
        family: String,
        name: String,
    },
    AddChain {
        family: String,
        table: String,
        chain: ChainSpec,
    },
    AddRule {
        family: String,
        table: String,
        chain: String,
        position: Position,
        rule: RuleSpec,
    },
    ReplaceRule {
        family: String,
        table: String,
        chain: String,
        handle: u64,
        rule: RuleSpec,
    },
    DeleteRule {
        family: String,
        table: String,
        chain: String,
        handle: u64,
    },
    MoveRule {
        family: String,
        table: String,
        chain: String,
        handle: u64,
        position: Position,
    },
    AddSet {
        family: String,
        table: String,
        set: SetSpec,
    },
    AddElement {
        family: String,
        table: String,
        set: String,
        elements: Vec<ElementSpec>,
    },
    DeleteElement {
        family: String,
        table: String,
        set: String,
        elements: Vec<ElementSpec>,
    },
    FlushChain {
        family: String,
        table: String,
        chain: String,
    },
    SetChainPolicy {
        family: String,
        table: String,
        chain: String,
        policy: String,
    },
    DeleteChain {
        family: String,
        table: String,
        chain: String,
    },
    DeleteSet {
        family: String,
        table: String,
        set: String,
    },
    DeleteTable {
        family: String,
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Position {
    First,
    Last,
    BeforeHandle { handle: u64 },
    AfterHandle { handle: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSpec {
    pub name: String,
    pub chain_type: Option<String>,
    pub hook: Option<String>,
    pub priority: Option<i32>,
    pub policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSpec {
    pub expr: Vec<serde_json::Value>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSpec {
    pub name: String,
    pub set_type: String,
    pub flags: Vec<String>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSpec {
    pub value: serde_json::Value,
}

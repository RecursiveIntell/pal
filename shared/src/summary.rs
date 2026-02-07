use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    pub matches: Vec<RuleMatch>,
    pub action: RuleAction,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    pub field: String,
    pub operator: String,
    pub value: String,
    pub negated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleAction {
    Accept,
    Drop,
    Reject {
        reject_type: Option<String>,
    },
    Jump {
        target: String,
    },
    Goto {
        target: String,
    },
    Return,
    Masquerade,
    Snat {
        nat_address: Option<String>,
        nat_port: Option<String>,
    },
    Dnat {
        nat_address: Option<String>,
        nat_port: Option<String>,
    },
    Redirect {
        nat_port: Option<String>,
    },
    Log {
        prefix: Option<String>,
    },
    Counter,
    Limit {
        rate: Option<String>,
    },
    Unknown {
        raw: String,
    },
}

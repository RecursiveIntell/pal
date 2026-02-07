use crate::nftables::ownership::detect_table_owner;
use palisade_shared::TableOwner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesetSnapshot {
    pub tables: Vec<TableState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub family: String,
    pub name: String,
    pub owner: TableOwner,
    pub chains: Vec<ChainState>,
    pub sets: Vec<SetState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub name: String,
    pub chain_type: Option<String>,
    pub hook: Option<String>,
    pub priority: Option<i32>,
    pub policy: Option<String>,
    pub rules: Vec<RuleState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleState {
    pub handle: u64,
    pub expr: Vec<serde_json::Value>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetState {
    pub name: String,
    pub set_type: Option<String>,
    pub flags: Vec<String>,
}

impl RulesetSnapshot {
    pub fn from_nft_json(raw: &str) -> anyhow::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(raw)?;
        let nftables = value
            .get("nftables")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut tables: Vec<TableState> = Vec::new();

        for entry in &nftables {
            if let Some(table) = entry.get("table") {
                let family = table
                    .get("family")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inet")
                    .to_string();
                let name = table
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                tables.push(TableState {
                    family,
                    name,
                    owner: TableOwner::Unmanaged,
                    chains: Vec::new(),
                    sets: Vec::new(),
                });
            }
        }

        for entry in &nftables {
            if let Some(chain) = entry.get("chain") {
                let family = chain
                    .get("family")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inet");
                let table = chain
                    .get("table")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if let Some(t) = tables
                    .iter_mut()
                    .find(|t| t.family == family && t.name == table)
                {
                    t.chains.push(ChainState {
                        name: chain
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("chain")
                            .to_string(),
                        chain_type: chain
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string),
                        hook: chain
                            .get("hook")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string),
                        priority: chain.get("prio").and_then(|v| v.as_i64()).map(|v| v as i32),
                        policy: chain
                            .get("policy")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string),
                        rules: Vec::new(),
                    });
                }
            }
            if let Some(set) = entry.get("set") {
                let family = set.get("family").and_then(|v| v.as_str()).unwrap_or("inet");
                let table = set
                    .get("table")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if let Some(t) = tables
                    .iter_mut()
                    .find(|t| t.family == family && t.name == table)
                {
                    let flags = set
                        .get("flags")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(ToString::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    t.sets.push(SetState {
                        name: set
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("set")
                            .to_string(),
                        set_type: set
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string),
                        flags,
                    });
                }
            }
            if let Some(rule) = entry.get("rule") {
                let family = rule
                    .get("family")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inet");
                let table = rule
                    .get("table")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let chain = rule
                    .get("chain")
                    .and_then(|v| v.as_str())
                    .unwrap_or("input");

                if let Some(t) = tables
                    .iter_mut()
                    .find(|t| t.family == family && t.name == table)
                {
                    if let Some(c) = t.chains.iter_mut().find(|c| c.name == chain) {
                        c.rules.push(RuleState {
                            handle: rule
                                .get("handle")
                                .and_then(|v| v.as_u64())
                                .unwrap_or_default(),
                            expr: rule
                                .get("expr")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default(),
                            comment: rule
                                .get("comment")
                                .and_then(|v| v.as_str())
                                .map(ToString::to_string),
                        });
                    }
                }
            }
        }

        for table in &mut tables {
            table.owner = detect_table_owner(&table.family, &table.name);
        }

        Ok(Self { tables })
    }
}

#[cfg(test)]
mod tests {
    use super::RulesetSnapshot;

    #[test]
    fn parses_basic_fixture() {
        let raw = include_str!("../../../tests/fixtures/rulesets/basic_server.json");
        let snapshot = RulesetSnapshot::from_nft_json(raw).expect("fixture should parse");
        assert_eq!(snapshot.tables.len(), 1);
        assert_eq!(snapshot.tables[0].name, "palisade");
        assert_eq!(snapshot.tables[0].chains.len(), 1);
        assert_eq!(snapshot.tables[0].chains[0].rules.len(), 3);
    }
}

use crate::nftables::ownership::detect_table_owner;
use palisade_shared::{Changeset, Operation, Position, TableOwner};
use serde_json::{Map, Value};

pub fn changeset_to_nft_batch(changeset: &Changeset) -> anyhow::Result<String> {
    let mut commands = Vec::new();

    for op in &changeset.operations {
        enforce_ownership(op)?;
        let cmd = match op {
            Operation::AddTable { family, name } => {
                serde_json::json!({"add": {"table": {"family": family, "name": name}}})
            }
            Operation::AddChain {
                family,
                table,
                chain,
            } => {
                let mut chain_obj = Map::new();
                chain_obj.insert("family".to_string(), Value::String(family.clone()));
                chain_obj.insert("table".to_string(), Value::String(table.clone()));
                chain_obj.insert("name".to_string(), Value::String(chain.name.clone()));
                if let Some(chain_type) = &chain.chain_type {
                    chain_obj.insert("type".to_string(), Value::String(chain_type.clone()));
                }
                if let Some(hook) = &chain.hook {
                    chain_obj.insert("hook".to_string(), Value::String(hook.clone()));
                }
                if let Some(priority) = chain.priority {
                    chain_obj.insert("prio".to_string(), Value::Number(priority.into()));
                }
                if let Some(policy) = &chain.policy {
                    chain_obj.insert("policy".to_string(), Value::String(policy.clone()));
                }
                let mut add_obj = Map::new();
                add_obj.insert("chain".to_string(), Value::Object(chain_obj));
                let mut root_obj = Map::new();
                root_obj.insert("add".to_string(), Value::Object(add_obj));
                Value::Object(root_obj)
            }
            Operation::AddRule {
                family,
                table,
                chain,
                position,
                rule,
            } => serde_json::json!({
                "add": {
                    "rule": {
                        "family": family,
                        "table": table,
                        "chain": chain,
                        "expr": rule.expr,
                        "comment": rule.comment,
                        "position": position_to_json(position)
                    }
                }
            }),
            Operation::ReplaceRule {
                family,
                table,
                chain,
                handle,
                rule,
            } => serde_json::json!({
                "replace": {
                    "rule": {
                        "family": family,
                        "table": table,
                        "chain": chain,
                        "handle": handle,
                        "expr": rule.expr,
                        "comment": rule.comment
                    }
                }
            }),
            Operation::DeleteRule {
                family,
                table,
                chain,
                handle,
            } => serde_json::json!({
                "delete": {"rule": {"family": family, "table": table, "chain": chain, "handle": handle}}
            }),
            Operation::MoveRule {
                family,
                table,
                chain,
                handle,
                position,
            } => serde_json::json!({
                "insert": {
                    "rule": {
                        "family": family,
                        "table": table,
                        "chain": chain,
                        "handle": handle,
                        "position": position_to_json(position)
                    }
                }
            }),
            Operation::AddSet { family, table, set } => serde_json::json!({
                "add": {
                    "set": {
                        "family": family,
                        "table": table,
                        "name": set.name,
                        "type": set.set_type,
                        "flags": set.flags,
                        "timeout": set.timeout
                    }
                }
            }),
            Operation::AddElement {
                family,
                table,
                set,
                elements,
            } => serde_json::json!({
                "add": {
                    "element": {
                        "family": family,
                        "table": table,
                        "name": set,
                        "elem": elements.iter().map(|e| e.value.clone()).collect::<Vec<_>>()
                    }
                }
            }),
            Operation::DeleteElement {
                family,
                table,
                set,
                elements,
            } => serde_json::json!({
                "delete": {
                    "element": {
                        "family": family,
                        "table": table,
                        "name": set,
                        "elem": elements.iter().map(|e| e.value.clone()).collect::<Vec<_>>()
                    }
                }
            }),
            Operation::DeleteTable { family, name } => {
                serde_json::json!({"delete": {"table": {"family": family, "name": name}}})
            }
            Operation::FlushChain {
                family,
                table,
                chain,
            } => {
                serde_json::json!({"flush": {"chain": {"family": family, "table": table, "name": chain}}})
            }
            Operation::SetChainPolicy {
                family,
                table,
                chain,
                policy,
            } => serde_json::json!({
                "add": {"chain": {"family": family, "table": table, "name": chain, "policy": policy}}
            }),
            Operation::DeleteChain {
                family,
                table,
                chain,
            } => serde_json::json!({
                "delete": {"chain": {"family": family, "table": table, "name": chain}}
            }),
            Operation::DeleteSet { family, table, set } => serde_json::json!({
                "delete": {"set": {"family": family, "table": table, "name": set}}
            }),
        };
        commands.push(cmd);
    }

    Ok(serde_json::to_string_pretty(
        &serde_json::json!({"nftables": commands}),
    )?)
}

fn position_to_json(position: &Position) -> serde_json::Value {
    match position {
        Position::First => serde_json::json!({"index": 0}),
        Position::Last => serde_json::json!({"index": "last"}),
        Position::BeforeHandle { handle } => serde_json::json!({"before": handle}),
        Position::AfterHandle { handle } => serde_json::json!({"after": handle}),
    }
}

fn enforce_ownership(op: &Operation) -> anyhow::Result<()> {
    let allowed = match op {
        Operation::AddTable { family, name } | Operation::DeleteTable { family, name } => {
            detect_table_owner(family, name) == TableOwner::Palisade
        }
        Operation::AddChain { family, table, .. }
        | Operation::AddRule { family, table, .. }
        | Operation::ReplaceRule { family, table, .. }
        | Operation::DeleteRule { family, table, .. }
        | Operation::MoveRule { family, table, .. }
        | Operation::AddSet { family, table, .. }
        | Operation::AddElement { family, table, .. }
        | Operation::DeleteElement { family, table, .. }
        | Operation::FlushChain { family, table, .. }
        | Operation::SetChainPolicy { family, table, .. }
        | Operation::DeleteChain { family, table, .. }
        | Operation::DeleteSet { family, table, .. } => {
            detect_table_owner(family, table) == TableOwner::Palisade
        }
    };

    if allowed {
        Ok(())
    } else {
        anyhow::bail!("changeset rejected: operation targets non-palisade table")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palisade_shared::changeset::ChainSpec;

    #[test]
    fn add_chain_omits_unset_optional_fields() {
        let changeset = Changeset {
            version: 1,
            description: "test".to_string(),
            operations: vec![Operation::AddChain {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: ChainSpec {
                    name: "zone_public".to_string(),
                    chain_type: None,
                    hook: None,
                    priority: None,
                    policy: None,
                },
            }],
        };

        let batch = changeset_to_nft_batch(&changeset).expect("batch should serialize");
        let value: serde_json::Value =
            serde_json::from_str(&batch).expect("batch should be valid json");
        let chain = &value["nftables"][0]["add"]["chain"];

        assert_eq!(chain["name"], "zone_public");
        assert!(chain.get("type").is_none());
        assert!(chain.get("hook").is_none());
        assert!(chain.get("prio").is_none());
        assert!(chain.get("policy").is_none());
    }
}

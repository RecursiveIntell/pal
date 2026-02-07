use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRuleRecord {
    pub rule_id: String,
    pub service_name: String,
    pub family: String,
    pub table: String,
    pub chain: String,
    pub protocol: Option<String>,
    pub port_start: Option<u32>,
    pub port_end: Option<u32>,
    pub temporary: bool,
    pub owner_bus_name: Option<String>,
    pub handle: Option<u64>,
    pub comment: String,
    pub raw_rule_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ServiceRuleRegistry {
    db_path: PathBuf,
}

impl ServiceRuleRegistry {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS service_rules (
                rule_id TEXT PRIMARY KEY,
                service_name TEXT NOT NULL,
                family TEXT NOT NULL,
                table_name TEXT NOT NULL,
                chain_name TEXT NOT NULL,
                protocol TEXT,
                port_start INTEGER,
                port_end INTEGER,
                temporary INTEGER NOT NULL,
                owner_bus_name TEXT,
                handle INTEGER,
                comment TEXT NOT NULL,
                raw_rule_json TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_service_rules_service_name
                ON service_rules(service_name);
            CREATE INDEX IF NOT EXISTS idx_service_rules_owner_bus_name
                ON service_rules(owner_bus_name);
            "#,
        )?;

        Ok(Self { db_path })
    }

    pub fn upsert(&self, record: &ServiceRuleRecord) -> anyhow::Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            r#"
            INSERT INTO service_rules (
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(rule_id) DO UPDATE SET
                service_name=excluded.service_name,
                family=excluded.family,
                table_name=excluded.table_name,
                chain_name=excluded.chain_name,
                protocol=excluded.protocol,
                port_start=excluded.port_start,
                port_end=excluded.port_end,
                temporary=excluded.temporary,
                owner_bus_name=excluded.owner_bus_name,
                handle=excluded.handle,
                comment=excluded.comment,
                raw_rule_json=excluded.raw_rule_json
            "#,
            params![
                record.rule_id,
                record.service_name,
                record.family,
                record.table,
                record.chain,
                record.protocol,
                record.port_start,
                record.port_end,
                if record.temporary { 1 } else { 0 },
                record.owner_bus_name,
                record.handle,
                record.comment,
                record.raw_rule_json,
                record.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, rule_id: &str) -> anyhow::Result<Option<ServiceRuleRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            FROM service_rules
            WHERE rule_id = ?
            "#,
        )?;

        let mut rows = stmt.query(params![rule_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row_to_record(row)?));
        }
        Ok(None)
    }

    pub fn delete(&self, rule_id: &str) -> anyhow::Result<bool> {
        let conn = Connection::open(&self.db_path)?;
        let changed = conn.execute("DELETE FROM service_rules WHERE rule_id = ?", params![rule_id])?;
        Ok(changed > 0)
    }

    pub fn list_by_service(&self, service_name: &str) -> anyhow::Result<Vec<ServiceRuleRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            FROM service_rules
            WHERE service_name = ?
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![service_name], row_to_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all(&self) -> anyhow::Result<Vec<ServiceRuleRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            FROM service_rules
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], row_to_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_temporary_for_owner(&self, owner_bus_name: &str) -> anyhow::Result<Vec<ServiceRuleRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            FROM service_rules
            WHERE temporary = 1 AND owner_bus_name = ?
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![owner_bus_name], row_to_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all_temporary(&self) -> anyhow::Result<Vec<ServiceRuleRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                rule_id, service_name, family, table_name, chain_name,
                protocol, port_start, port_end, temporary, owner_bus_name,
                handle, comment, raw_rule_json, created_at
            FROM service_rules
            WHERE temporary = 1
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], row_to_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn new_record(
        rule_id: String,
        service_name: String,
        protocol: Option<String>,
        port_start: Option<u32>,
        port_end: Option<u32>,
        temporary: bool,
        owner_bus_name: Option<String>,
        comment: String,
        raw_rule_json: Option<String>,
    ) -> ServiceRuleRecord {
        ServiceRuleRecord {
            rule_id,
            service_name,
            family: "inet".to_string(),
            table: "palisade".to_string(),
            chain: "service-rules".to_string(),
            protocol,
            port_start,
            port_end,
            temporary,
            owner_bus_name,
            handle: None,
            comment,
            raw_rule_json,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServiceRuleRecord> {
    let temporary: i64 = row.get(8)?;
    let handle_i64: Option<i64> = row.get(10)?;
    Ok(ServiceRuleRecord {
        rule_id: row.get(0)?,
        service_name: row.get(1)?,
        family: row.get(2)?,
        table: row.get(3)?,
        chain: row.get(4)?,
        protocol: row.get(5)?,
        port_start: row.get(6)?,
        port_end: row.get(7)?,
        temporary: temporary == 1,
        owner_bus_name: row.get(9)?,
        handle: handle_i64.map(|h| h as u64),
        comment: row.get(11)?,
        raw_rule_json: row.get(12)?,
        created_at: row.get(13)?,
    })
}

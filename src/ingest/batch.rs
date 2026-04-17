//! Shared multi-statement transaction helper for ingestion paths.
//!
//! SurrealKV is an embedded single-writer LSM: throughput is bound by commit
//! count, not statement count. Wrapping many CREATE/RELATE/UPDATE statements
//! in a single `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` lets the engine
//! perform one fsync per batch instead of one per statement, which is the
//! single biggest lever for ingest speed.

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{SurrealValue, Variables};

use crate::db::Db;

/// Accumulates SurrealQL statements + bindings for a single transaction-wrapped
/// multi-statement query. Each call to [`Batch::param`] allocates a unique
/// parameter name (`p_<n>`) so we can mix many CREATE/RELATE statements safely.
pub(crate) struct Batch {
    sql: String,
    vars: Variables,
    counter: usize,
}

impl Batch {
    pub(crate) fn new() -> Self {
        let mut sql = String::new();
        sql.push_str("BEGIN TRANSACTION;\n");
        Self {
            sql,
            vars: Variables::new(),
            counter: 0,
        }
    }

    /// Bind a value and return its `$param_<n>` placeholder for inlining into SQL.
    pub(crate) fn param(&mut self, value: impl SurrealValue) -> String {
        let key = format!("p_{}", self.counter);
        self.counter += 1;
        self.vars.insert(key.clone(), value);
        format!("${key}")
    }

    /// Append a statement (terminating semicolon + newline added).
    pub(crate) fn push(&mut self, stmt: impl AsRef<str>) {
        self.sql.push_str(stmt.as_ref());
        self.sql.push_str(";\n");
    }

    /// Whether any statements have been pushed (no params bound yet).
    pub(crate) fn is_empty(&self) -> bool {
        self.counter == 0
    }

    /// Commit the transaction in a single round-trip. No-op if empty.
    pub(crate) async fn commit(mut self, db: &Surreal<Db>) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        self.sql.push_str("COMMIT TRANSACTION;");
        db.query(&self.sql).bind(self.vars).await?.check()?;
        Ok(())
    }
}

/// Read the shared `INGEST_BATCH` env var, falling back to `default`.
///
/// Use a single env var across all ingest paths so operators can tune one knob.
pub(crate) fn batch_size_from_env(default: usize) -> usize {
    std::env::var("INGEST_BATCH")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or(default)
}

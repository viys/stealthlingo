use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Ordered schema migrations; `PRAGMA user_version` records how many have run.
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_initial.sql"),
    include_str!("../../migrations/002_dictionary_sources.sql"),
    include_str!("../../migrations/003_word_aliases.sql"),
    include_str!("../../migrations/004_archived_words.sql"),
    include_str!("../../migrations/005_word_origin.sql"),
];

pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("could not create {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("could not open database {}", path.display()))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    pub fn latest_schema_version() -> u32 {
        MIGRATIONS.len() as u32
    }

    fn migrate(&mut self) -> Result<()> {
        let current = self.schema_version()? as usize;
        if current > MIGRATIONS.len() {
            anyhow::bail!(
                "database schema version {current} is newer than this StealthLingo supports ({}); please upgrade",
                MIGRATIONS.len()
            );
        }
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)
                .with_context(|| format!("database migration {} failed", index + 1))?;
            tx.pragma_update(None, "user_version", (index + 1) as u32)?;
            tx.commit()?;
        }
        Ok(())
    }
}

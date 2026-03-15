use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Table Metadata Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub catalog_name: String,
    pub schema_name: String,
    pub table_name: String,
    pub file_path: String,
    pub source_type: String, // "csv", "excel", "sqlite", "oracle"
    pub sheet_name: Option<String>,
    pub schema_json: Option<String>,  // Serialized Arrow Schema
    pub stats_json: Option<String>,   // Serialized Statistics (row count, etc.)
    pub indexes_json: Option<String>, // Serialized Index Info

    // Extended fields
    pub header_rows: Option<usize>,
    pub header_mode: Option<String>,
    pub column_default_formulas_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub table_name: String,
    pub friendly_name: Option<String>, // Changed from name
    pub lance_path: String,
    pub created_at: i64, // Changed from u64
    pub is_default: bool,
    pub parent_session_id: Option<String>, // Changed from from_session_id
    pub last_accessed_at: i64,             // Changed from u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetAttribute {
    pub session_id: String,
    pub cell_key: String,
    pub attr_type: String,
    pub attr_value: String,
}

pub struct MetadataStore {
    conn: Arc<Mutex<Connection>>,
}

impl MetadataStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // 1. Create table with new columns if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tables_metadata (
                id INTEGER PRIMARY KEY,
                catalog_name TEXT NOT NULL DEFAULT 'datafusion',
                schema_name TEXT NOT NULL DEFAULT 'public',
                table_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                sheet_name TEXT,
                schema_json TEXT,
                stats_json TEXT,
                indexes_json TEXT,
                header_rows INTEGER,
                header_mode TEXT,
                column_default_formulas_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(catalog_name, schema_name, table_name)
            )",
            [],
        )?;

        // 2. Migration: Try to add columns if they don't exist
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN catalog_name TEXT NOT NULL DEFAULT 'datafusion'", []);
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN schema_name TEXT NOT NULL DEFAULT 'public'",
            [],
        );
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN sheet_name TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN schema_json TEXT",
            [],
        );
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN stats_json TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN indexes_json TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN header_rows INTEGER",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN header_mode TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN column_default_formulas_json TEXT",
            [],
        );

        // 3. Create Sessions Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                table_name TEXT NOT NULL,
                friendly_name TEXT,
                lance_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                is_default BOOLEAN NOT NULL DEFAULT 0,
                parent_session_id TEXT,
                last_accessed_at INTEGER NOT NULL
            )",
            [],
        )?;

        // 4. Create Sheet Attributes Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sheet_attributes (
                session_id TEXT NOT NULL,
                cell_key TEXT NOT NULL,
                attr_type TEXT NOT NULL,
                attr_value TEXT,
                PRIMARY KEY (session_id, cell_key, attr_type)
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_table(&self, meta: &TableMetadata) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tables_metadata (catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json, header_rows, header_mode, column_default_formulas_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            (
                &meta.catalog_name,
                &meta.schema_name,
                &meta.table_name,
                &meta.file_path,
                &meta.source_type,
                &meta.sheet_name,
                &meta.schema_json,
                &meta.stats_json,
                &meta.indexes_json,
                &meta.header_rows,
                &meta.header_mode,
                &meta.column_default_formulas_json
            ),
        )?;
        Ok(())
    }

    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json, header_rows, header_mode, column_default_formulas_json FROM tables_metadata")?;

        let rows = stmt.query_map([], |row| {
            Ok(TableMetadata {
                catalog_name: row.get(0)?,
                schema_name: row.get(1)?,
                table_name: row.get(2)?,
                file_path: row.get(3)?,
                source_type: row.get(4)?,
                sheet_name: row.get(5)?,
                schema_json: row.get(6)?,
                stats_json: row.get(7)?,
                indexes_json: row.get(8)?,
                header_rows: row.get(9)?,
                header_mode: row.get(10)?,
                column_default_formulas_json: row.get(11)?,
            })
        })?;

        let mut tables = Vec::new();
        for row in rows {
            tables.push(row?);
        }
        Ok(tables)
    }

    pub fn add_table(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        file_path: &str,
        source_type: &str,
        sheet_name: Option<String>,
    ) -> Result<()> {
        self.save_table(&TableMetadata {
            catalog_name: catalog.to_string(),
            schema_name: schema.to_string(),
            table_name: table.to_string(),
            file_path: file_path.to_string(),
            source_type: source_type.to_string(),
            sheet_name,
            schema_json: None,
            stats_json: None,
            indexes_json: None,
            header_rows: None,
            header_mode: None,
            column_default_formulas_json: None,
        })
    }

    pub fn delete_table(&self, catalog: &str, schema: &str, table: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM tables_metadata WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3",
            (catalog, schema, table),
        )?;
        Ok(count)
    }

    pub fn get_table(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
    ) -> Result<Option<TableMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json, header_rows, header_mode, column_default_formulas_json
             FROM tables_metadata 
             WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3"
        )?;

        let mut rows = stmt.query_map((catalog, schema, table), |row| {
            Ok(TableMetadata {
                catalog_name: row.get(0)?,
                schema_name: row.get(1)?,
                table_name: row.get(2)?,
                file_path: row.get(3)?,
                source_type: row.get(4)?,
                sheet_name: row.get(5)?,
                schema_json: row.get(6)?,
                stats_json: row.get(7)?,
                indexes_json: row.get(8)?,
                header_rows: row.get(9)?,
                header_mode: row.get(10)?,
                column_default_formulas_json: row.get(11)?,
            })
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    // --- Session Management ---

    pub fn create_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sessions (session_id, table_name, friendly_name, lance_path, created_at, is_default, parent_session_id, last_accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &session.session_id,
                &session.table_name,
                &session.friendly_name,
                &session.lance_path,
                &session.created_at,
                &session.is_default,
                &session.parent_session_id,
                &session.last_accessed_at
            ),
        )?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])?;
        Ok(count)
    }

    pub fn list_all_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT session_id, table_name, friendly_name, lance_path, created_at, is_default, parent_session_id, last_accessed_at FROM sessions")?;
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                session_id: row.get(0)?,
                table_name: row.get(1)?,
                friendly_name: row.get(2)?,
                lance_path: row.get(3)?,
                created_at: row.get(4)?,
                is_default: row.get(5)?,
                parent_session_id: row.get(6)?,
                last_accessed_at: row.get(7)?,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    // --- Sheet Attributes ---

    pub fn get_sheet_attributes(&self, session_id: &str) -> Result<Vec<SheetAttribute>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT session_id, cell_key, attr_type, attr_value FROM sheet_attributes WHERE session_id = ?1")?;
        let rows = stmt.query_map([session_id], |row| {
            Ok(SheetAttribute {
                session_id: row.get(0)?,
                cell_key: row.get(1)?,
                attr_type: row.get(2)?,
                attr_value: row.get(3)?,
            })
        })?;

        let mut attrs = Vec::new();
        for row in rows {
            attrs.push(row?);
        }
        Ok(attrs)
    }

    pub fn set_sheet_attribute(&self, attr: &SheetAttribute) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sheet_attributes (session_id, cell_key, attr_type, attr_value) VALUES (?1, ?2, ?3, ?4)",
            (&attr.session_id, &attr.cell_key, &attr.attr_type, &attr.attr_value),
        )?;
        Ok(())
    }

    pub fn delete_sheet_attributes_by_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM sheet_attributes WHERE session_id = ?1",
            [session_id],
        )?;
        Ok(0)
    }

    // Transactional Helper
    pub fn create_table_transaction(&self, table: &TableMetadata, session: &Session) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Save Table
        tx.execute(
            "INSERT OR REPLACE INTO tables_metadata (catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json, header_rows, header_mode, column_default_formulas_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            (
                &table.catalog_name,
                &table.schema_name,
                &table.table_name,
                &table.file_path,
                &table.source_type,
                &table.sheet_name,
                &table.schema_json,
                &table.stats_json,
                &table.indexes_json,
                &table.header_rows,
                &table.header_mode,
                &table.column_default_formulas_json
            ),
        )?;

        // Save Session
        tx.execute(
            "INSERT OR REPLACE INTO sessions (session_id, table_name, friendly_name, lance_path, created_at, is_default, parent_session_id, last_accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &session.session_id,
                &session.table_name,
                &session.friendly_name,
                &session.lance_path,
                &session.created_at,
                &session.is_default,
                &session.parent_session_id,
                &session.last_accessed_at
            ),
        )?;

        tx.commit()?;
        Ok(())
    }
}

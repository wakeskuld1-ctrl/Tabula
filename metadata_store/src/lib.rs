use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

// Table Metadata Structure
#[derive(Debug, Clone)]
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(catalog_name, schema_name, table_name)
            )",
            [],
        )?;

        // 2. Migration: Try to add columns if they don't exist for backward compatibility
        // Ignoring errors if columns already exist
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN catalog_name TEXT NOT NULL DEFAULT 'datafusion'", []);
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN schema_name TEXT NOT NULL DEFAULT 'public'", []);
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN sheet_name TEXT", []);
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN schema_json TEXT", []);
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN stats_json TEXT", []);
        let _ = conn.execute("ALTER TABLE tables_metadata ADD COLUMN indexes_json TEXT", []);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_table(&self, meta: &TableMetadata) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tables_metadata (catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                &meta.catalog_name,
                &meta.schema_name,
                &meta.table_name, 
                &meta.file_path, 
                &meta.source_type, 
                &meta.sheet_name,
                &meta.schema_json,
                &meta.stats_json,
                &meta.indexes_json
            ),
        )?;
        Ok(())
    }

    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json FROM tables_metadata")?;
        
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
            })
        })?;

        let mut tables = Vec::new();
        for row in rows {
            tables.push(row?);
        }
        Ok(tables)
    }

    pub fn add_table(&self, catalog: &str, schema: &str, table: &str, file_path: &str, source_type: &str, sheet_name: Option<String>) -> Result<()> {
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

    pub fn get_table(&self, catalog: &str, schema: &str, table: &str) -> Result<Option<TableMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name, schema_json, stats_json, indexes_json 
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
            })
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }
}

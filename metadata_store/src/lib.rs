use rusqlite::{Connection, Result};
// **[2026-03-15]** 变更原因：新增会话与样式属性需要序列化结构。
// **[2026-03-15]** 变更目的：为 Session / SheetAttribute / TableMetadata 提供统一序列化能力。
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Table Metadata Structure
// **[2026-03-15]** 变更原因：会话管理需要持久化表头配置与默认公式。
// **[2026-03-15]** 变更目的：补齐 header_rows/header_mode/column_default_formulas_json。
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

    // **[2026-03-15]** 变更原因：表注册入口已带 header_rows/header_mode 参数。
    // **[2026-03-15]** 变更目的：落盘表头配置，供后续加载与比对。
    pub header_rows: Option<usize>,
    // **[2026-03-15]** 变更原因：注册流程需要记录 header_mode。
    // **[2026-03-15]** 变更目的：保持 UI/后端解析规则一致。
    pub header_mode: Option<String>,
    // **[2026-03-15]** 变更原因：默认公式列需持久化。
    // **[2026-03-15]** 变更目的：支持公式列回放与会话复制。
    pub column_default_formulas_json: Option<String>,
}

// **[2026-03-15]** 变更原因：SessionManager 依赖 Session 结构落盘。
// **[2026-03-15]** 变更目的：补齐会话持久化与恢复流程。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub table_name: String,
    pub friendly_name: Option<String>,
    pub lance_path: String,
    pub created_at: i64,
    pub is_default: bool,
    pub parent_session_id: Option<String>,
    pub last_accessed_at: i64,
}

// **[2026-03-15]** 变更原因：样式/合并需要按单元格持久化。
// **[2026-03-15]** 变更目的：提供 SheetAttribute 结构支持样式/合并存储。
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
    // **[2026-03-15]** 变更原因：新增 sessions / sheet_attributes 表。
    // **[2026-03-15]** 变更目的：补齐会话与样式属性持久化能力。
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

        // 2. Migration: Try to add columns if they don't exist for backward compatibility
        // Ignoring errors if columns already exist
        let _ = conn.execute(
            "ALTER TABLE tables_metadata ADD COLUMN catalog_name TEXT NOT NULL DEFAULT 'datafusion'",
            [],
        );
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
        // **[2026-03-15]** 变更原因：补齐表头/公式字段迁移。
        // **[2026-03-15]** 变更目的：兼容旧库，避免已有表报错。
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

    // **[2026-03-15]** 变更原因：新增表字段需要持久化。
    // **[2026-03-15]** 变更目的：保存 header_rows/header_mode/column_default_formulas_json。
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
                &meta.column_default_formulas_json,
            ),
        )?;
        Ok(())
    }

    // **[2026-03-15]** 变更原因：新增字段需要读回。
    // **[2026-03-15]** 变更目的：保证列表查询包含表头/公式配置。
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

    // **[2026-03-15]** 变更原因：保持 add_table 接口对新字段兼容。
    // **[2026-03-15]** 变更目的：新增字段默认 None 不影响旧调用。
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

    // **[2026-03-15]** 变更原因：新增字段需要读回。
    // **[2026-03-15]** 变更目的：保证 get_table 返回完整表信息。
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
             WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3",
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

    // **[2026-03-15]** 变更原因：会话需要落盘供恢复与切换。
    // **[2026-03-15]** 变更目的：提供 create_session 持久化入口。
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
                &session.last_accessed_at,
            ),
        )?;
        Ok(())
    }

    // **[2026-03-15]** 变更原因：删除表时需清理会话。
    // **[2026-03-15]** 变更目的：提供 delete_session 以匹配 SessionManager 清理流程。
    pub fn delete_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])?;
        Ok(count)
    }

    // **[2026-03-15]** 变更原因：SessionManager 需要加载全部会话。
    // **[2026-03-15]** 变更目的：提供 list_all_sessions 批量读取。
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

    // **[2026-03-15]** 变更原因：样式/合并需要从 DB 读取。
    // **[2026-03-15]** 变更目的：提供 get_sheet_attributes 读取入口。
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

    // **[2026-03-15]** 变更原因：样式/合并需要写入 DB。
    // **[2026-03-15]** 变更目的：提供 set_sheet_attribute 写入入口。
    pub fn set_sheet_attribute(&self, attr: &SheetAttribute) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sheet_attributes (session_id, cell_key, attr_type, attr_value) VALUES (?1, ?2, ?3, ?4)",
            (
                &attr.session_id,
                &attr.cell_key,
                &attr.attr_type,
                &attr.attr_value,
            ),
        )?;
        Ok(())
    }

    // **[2026-03-15]** 变更原因：删除会话需清理属性。
    // **[2026-03-15]** 变更目的：提供 delete_sheet_attributes_by_session 清理入口。
    pub fn delete_sheet_attributes_by_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM sheet_attributes WHERE session_id = ?1",
            [session_id],
        )?;
        Ok(count)
    }

    // Transactional Helper
    // **[2026-03-15]** 变更原因：注册流程需要表与会话同事务写入。
    // **[2026-03-15]** 变更目的：提供 create_table_transaction 以保持一致性。
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
                &table.column_default_formulas_json,
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
                &session.last_accessed_at,
            ),
        )?;

        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // **[2026-03-15]** 变更原因：先复现会话与样式属性缺失导致的基线编译问题。
    // **[2026-03-15]** 变更目的：用最小可复现测试驱动补齐 Session / SheetAttribute 能力。
    // **[2026-03-15]** 变更说明：测试先红后绿，满足 TDD 要求。
    #[test]
    fn test_session_and_sheet_attributes_roundtrip() {
        let db_path = "test_metadata_sessions.db";
        let _ = std::fs::remove_file(db_path);

        let store = MetadataStore::new(db_path).expect("init store");

        let session = Session {
            session_id: "s1".to_string(),
            table_name: "t1".to_string(),
            friendly_name: Some("Draft".to_string()),
            lance_path: "path".to_string(),
            created_at: 1,
            is_default: true,
            parent_session_id: None,
            last_accessed_at: 1,
        };
        store.create_session(&session).expect("create session");

        let sessions = store.list_all_sessions().expect("list sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "s1");

        let attr = SheetAttribute {
            session_id: "s1".to_string(),
            cell_key: "0,0".to_string(),
            attr_type: "style".to_string(),
            attr_value: "{}".to_string(),
        };
        store.set_sheet_attribute(&attr).expect("set attr");

        let attrs = store
            .get_sheet_attributes("s1")
            .expect("get attrs");
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].cell_key, "0,0");

        let _ = store
            .delete_sheet_attributes_by_session("s1")
            .expect("delete attrs");
        let attrs = store
            .get_sheet_attributes("s1")
            .expect("get attrs after delete");
        assert!(attrs.is_empty());

        let _ = store.delete_session("s1").expect("delete session");
        let sessions = store
            .list_all_sessions()
            .expect("list sessions after delete");
        assert!(sessions.is_empty());

        let _ = std::fs::remove_file(db_path);
    }
}

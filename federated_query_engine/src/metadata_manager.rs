use datafusion::error::{DataFusionError, Result};
use datafusion::prelude::SessionContext;
use metadata_store::{ConnectionMetadata, MetadataStore, TableMetadata};
use rusqlite::Connection;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct MetadataManager {
    store: Arc<MetadataStore>,
}

impl MetadataManager {
    /// 初始化元数据管理器
    ///
    /// **实现方案**:
    /// 创建 `MetadataStore` 实例并用 `Arc` 封装，提供线程安全的访问。
    ///
    /// **调用链路**:
    /// - 应用程序启动时调用。
    pub fn new(store_path: &str) -> Result<Self> {
        let store = MetadataStore::new(store_path).map_err(|e| {
            DataFusionError::Execution(format!("Failed to init metadata store: {}", e))
        })?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    // --- Connections ---

    /// 保存数据库连接配置
    ///
    /// **实现方案**:
    /// 将连接参数封装为 `ConnectionMetadata`，并调用底层 `MetadataStore` 进行持久化。
    ///
    /// **调用链路**:
    /// - API 层 (如 `/api/connections`) 调用。
    pub fn save_connection(
        &self,
        id: &str,
        name: &str,
        source_type: &str,
        config: &str,
    ) -> Result<()> {
        let meta = ConnectionMetadata {
            id: id.to_string(),
            name: name.to_string(),
            source_type: source_type.to_string(),
            config: config.to_string(),
        };
        self.store
            .save_connection(&meta)
            .map_err(|e| DataFusionError::Execution(format!("Failed to save connection: {}", e)))
    }

    pub fn list_connections(&self) -> Result<Vec<ConnectionMetadata>> {
        self.store
            .list_connections()
            .map_err(|e| DataFusionError::Execution(format!("Failed to list connections: {}", e)))
    }

    pub fn get_connection(&self, id: &str) -> Result<Option<ConnectionMetadata>> {
        self.store
            .get_connection(id)
            .map_err(|e| DataFusionError::Execution(format!("Failed to get connection: {}", e)))
    }

    pub fn delete_connection(&self, id: &str) -> Result<usize> {
        self.store
            .delete_connection(id)
            .map_err(|e| DataFusionError::Execution(format!("Failed to delete connection: {}", e)))
    }

    // --- Tables ---

    /// 注册表到元数据存储
    ///
    /// **实现方案**:
    /// 1. 从 DataFusion Context 中获取 Schema 信息并序列化。
    /// 2. 如果是 SQLite 源，通过 PRAGMA 语句获取索引信息并序列化。
    /// 3. 构建 `TableMetadata` 对象，设置 `stats_updated_at` 为当前时间。
    /// 4. 调用底层 `MetadataStore` 持久化。
    ///
    /// **调用链路**:
    /// - 用户手动注册表时调用。
    /// - 系统启动时自动发现表时调用。
    ///
    /// **关键问题点**:
    /// - Schema 捕获：依赖 DataFusion 的 `table_provider` 接口。
    /// - 索引捕获：仅 SQLite 源实现了索引元数据提取。
    #[allow(clippy::too_many_arguments)]
    pub async fn register_table(
        &self,
        ctx: &SessionContext,
        catalog: &str,
        schema: &str,
        table: &str,
        file_path: &str,
        source_type: &str,
        sheet_name: Option<String>,
        stats_json: Option<String>,
    ) -> Result<()> {
        // 1. Capture Schema from DataFusion
        let schema_json = if let Ok(provider) = ctx.table_provider(table).await {
            let schema = provider.schema();
            let fields: Vec<serde_json::Value> = schema
                .fields()
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name(),
                        "type": f.data_type().to_string(),
                        "nullable": f.is_nullable()
                    })
                })
                .collect();
            Some(serde_json::to_string(&fields).unwrap_or_default())
        } else {
            None
        };

        // 2. Capture Indexes (if SQLite)
        let indexes_json = if source_type == "sqlite" {
            if let Ok(conn) = Connection::open(file_path) {
                let mut indexes = Vec::new();
                let sql = format!("PRAGMA index_list({})", table);
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    let index_iter = stmt.query_map([], |row| {
                        let name: String = row.get(1)?;
                        let unique: bool = row.get(2)?;
                        Ok((name, unique))
                    });

                    if let Ok(iter) = index_iter {
                        for (name, unique) in iter.flatten() {
                            let mut cols = Vec::new();
                            let info_sql = format!("PRAGMA index_info({})", name);
                            if let Ok(mut info_stmt) = conn.prepare(&info_sql) {
                                let col_iter = info_stmt.query_map([], |r| {
                                    let col_name: String = r.get(2)?;
                                    Ok(col_name)
                                });
                                if let Ok(c_iter) = col_iter {
                                    for cn in c_iter.flatten() {
                                        cols.push(cn);
                                    }
                                }
                            }
                            indexes.push(serde_json::json!({
                                "name": name,
                                "unique": unique,
                                "columns": cols
                            }));
                        }
                    }
                }
                if !indexes.is_empty() {
                    Some(serde_json::to_string(&indexes).unwrap_or_default())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let stats_stale = stats_json.is_none();
        let meta = TableMetadata {
            catalog_name: catalog.to_string(),
            schema_name: schema.to_string(),
            table_name: table.to_string(),
            file_path: file_path.to_string(),
            source_type: source_type.to_string(),
            sheet_name,
            schema_json,
            stats_json,
            indexes_json,
            stats_updated_at: Some(now_timestamp()),
            stats_stale: Some(stats_stale),
            stats_source: Some("register".to_string()),
        };

        self.store
            .save_table(&meta)
            .map_err(|e| DataFusionError::Execution(format!("Failed to register table: {}", e)))
    }

    pub fn unregister_table(&self, catalog: &str, schema: &str, table: &str) -> Result<usize> {
        self.store
            .delete_table(catalog, schema, table)
            .map_err(|e| DataFusionError::Execution(format!("Failed to unregister table: {}", e)))
    }

    /// 列出所有已注册的表
    ///
    /// **实现方案**:
    /// 直接调用底层 `MetadataStore` 的 `list_tables` 方法。
    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        self.store
            .list_tables()
            .map_err(|e| DataFusionError::Execution(format!("Failed to list tables: {}", e)))
    }

    #[allow(dead_code)]
    pub fn find_tables(&self, catalog: &str, schema: &str, table: &str) -> Vec<TableMetadata> {
        // Since we don't have a direct find method in MetadataStore that supports duplicates efficiently
        // (or maybe we do, let's check MetadataStore API)
        // Actually MetadataStore has `get_table` which returns Option<TableMetadata> (single).
        // But we changed save_table to allow duplicates.
        // `get_table` implementation in MetadataStore needs to be checked.
        // It uses `find`, which returns the FIRST match.
        // So we should use `list_tables` and filter here.

        if let Ok(tables) = self.store.list_tables() {
            tables
                .into_iter()
                .filter(|t| {
                    t.catalog_name == catalog && t.schema_name == schema && t.table_name == table
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    #[allow(dead_code)]
    pub async fn refresh_metadata(&self) -> Result<()> {
        // Placeholder for metadata refresh logic
        // This will iterate over tables and update schema/stats by contacting the source
        // In a real implementation, we would call source.introspect() here
        // println!("Refreshing metadata...");
        // Commented out to avoid noise in tests/logs
        Ok(())
    }
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_metadata_manager_lifecycle() {
        let db_path = "test_metadata_mgr.db";
        if std::path::Path::new(db_path).exists() {
            fs::remove_file(db_path).unwrap();
        }

        let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
        let ctx = SessionContext::new();

        // 1. Clear existing tables first to ensure clean state
        let existing = mgr.list_tables().unwrap();
        for t in existing {
            mgr.unregister_table(&t.catalog_name, &t.schema_name, &t.table_name)
                .unwrap();
        }

        // 1. Register a table with 3-layer namespace
        mgr.register_table(
            &ctx,
            "my_cat",
            "my_schema",
            "orders",
            "/tmp/orders.csv",
            "csv",
            None,
            None,
        )
        .await
        .expect("Failed to register table");

        // 2. List tables
        let tables = mgr.list_tables().expect("Failed to list tables");
        assert_eq!(tables.len(), 1);

        let t = &tables[0];
        assert_eq!(t.catalog_name, "my_cat");
        assert_eq!(t.schema_name, "my_schema");
        assert_eq!(t.table_name, "orders");
        assert_eq!(t.file_path, "/tmp/orders.csv");

        // 3. Register another table (default namespace simulation)
        mgr.register_table(
            &ctx,
            "datafusion",
            "public",
            "users",
            "/tmp/users.xlsx",
            "excel",
            Some("Sheet1".to_string()),
            None,
        )
        .await
        .expect("Failed to register users");

        let tables = mgr.list_tables().expect("Failed to list tables");
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[1].sheet_name, Some("Sheet1".to_string()));

        // Cleanup
        drop(mgr); // Explicitly drop manager to close DB connection
                   // Give OS a moment to release the file lock if necessary, though drop should be synchronous for connection closing
                   // We ignore the error here because the test logic already passed, and file deletion issues on Windows are common in tests
        let _ = fs::remove_file(db_path);
    }
}

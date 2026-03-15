use datafusion::error::{DataFusionError, Result};
use datafusion::prelude::SessionContext;
use metadata_store::{MetadataStore, TableMetadata};
use rusqlite::Connection;
use std::sync::Arc;

#[derive(Clone)]
pub struct MetadataManager {
    // **[2026-03-15]** 变更原因：SessionManager/handlers 需要访问 store CRUD。
    // **[2026-03-15]** 变更目的：开放 store 以保持现有调用语义。
    pub store: Arc<MetadataStore>,
}

// **[2026-03-15]** 变更原因：register_table 调用已统一为参数结构体。
// **[2026-03-15]** 变更目的：对齐 register_service/upload_service 的调用方式。
pub struct RegisterTableParams<'a> {
    pub catalog: &'a str,
    pub schema: &'a str,
    pub table: &'a str,
    pub file_path: &'a str,
    pub source_type: &'a str,
    pub sheet_name: Option<String>,
    pub header_rows: Option<usize>,
    pub header_mode: Option<String>,
}

impl MetadataManager {
    pub fn new(store_path: &str) -> Result<Self> {
        let store = MetadataStore::new(store_path).map_err(|e| {
            DataFusionError::Execution(format!("Failed to init metadata store: {}", e))
        })?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    // **[2026-03-15]** 变更原因：RegisterTableParams 结构体取代多参数签名。
    // **[2026-03-15]** 变更目的：统一注册入口并保留 header_rows/header_mode 落盘。
    pub async fn register_table(
        &self,
        ctx: &SessionContext,
        params: RegisterTableParams<'_>,
    ) -> Result<()> {
        let RegisterTableParams {
            catalog,
            schema,
            table,
            file_path,
            source_type,
            sheet_name,
            header_rows,
            header_mode,
        } = params;

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

        // 3. Save to Metadata Store
        let meta = TableMetadata {
            catalog_name: catalog.to_string(),
            schema_name: schema.to_string(),
            table_name: table.to_string(),
            file_path: file_path.to_string(),
            source_type: source_type.to_string(),
            sheet_name,
            schema_json,
            stats_json: None,
            indexes_json,
            header_rows,
            header_mode,
            column_default_formulas_json: None,
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

    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        self.store
            .list_tables()
            .map_err(|e| DataFusionError::Execution(format!("Failed to list tables: {}", e)))
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

        // 1. Register a table with 3-layer namespace
        mgr.register_table(
            &ctx,
            RegisterTableParams {
                catalog: "my_cat",
                schema: "my_schema",
                table: "orders",
                file_path: "/tmp/orders.csv",
                source_type: "csv",
                sheet_name: None,
                header_rows: None,
                header_mode: None,
            },
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
            RegisterTableParams {
                catalog: "datafusion",
                schema: "public",
                table: "users",
                file_path: "/tmp/users.xlsx",
                source_type: "excel",
                sheet_name: Some("Sheet1".to_string()),
                header_rows: None,
                header_mode: None,
            },
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

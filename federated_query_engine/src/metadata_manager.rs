use std::sync::Arc;
use metadata_store::{MetadataStore, TableMetadata};
use datafusion::error::{DataFusionError, Result};
use datafusion::prelude::SessionContext;
use rusqlite::Connection;

#[derive(Clone)]
pub struct MetadataManager {
    store: Arc<MetadataStore>,
}

impl MetadataManager {
    pub fn new(store_path: &str) -> Result<Self> {
        let store = MetadataStore::new(store_path)
            .map_err(|e| DataFusionError::Execution(format!("Failed to init metadata store: {}", e)))?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    pub async fn register_table(&self, ctx: &SessionContext, catalog: &str, schema: &str, table: &str, file_path: &str, source_type: &str, sheet_name: Option<String>) -> Result<()> {
        // 1. Capture Schema from DataFusion
        let schema_json = if let Ok(provider) = ctx.table_provider(table).await {
            let schema = provider.schema();
            let fields: Vec<serde_json::Value> = schema.fields().iter().map(|f| {
                serde_json::json!({
                    "name": f.name(),
                    "type": f.data_type().to_string(),
                    "nullable": f.is_nullable()
                })
            }).collect();
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
                         for i in iter {
                             if let Ok((name, unique)) = i {
                                 let mut cols = Vec::new();
                                 let info_sql = format!("PRAGMA index_info({})", name);
                                 if let Ok(mut info_stmt) = conn.prepare(&info_sql) {
                                     let col_iter = info_stmt.query_map([], |r| {
                                         let col_name: String = r.get(2)?;
                                         Ok(col_name)
                                     });
                                     if let Ok(c_iter) = col_iter {
                                         for c in c_iter {
                                             if let Ok(cn) = c { cols.push(cn); }
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
        };

        self.store.save_table(&meta)
            .map_err(|e| DataFusionError::Execution(format!("Failed to register table: {}", e)))
    }

    pub fn unregister_table(&self, catalog: &str, schema: &str, table: &str) -> Result<usize> {
        self.store.delete_table(catalog, schema, table)
            .map_err(|e| DataFusionError::Execution(format!("Failed to unregister table: {}", e)))
    }

    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        self.store.list_tables()
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
        mgr.register_table(&ctx, "my_cat", "my_schema", "orders", "/tmp/orders.csv", "csv", None)
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
        mgr.register_table(&ctx, "datafusion", "public", "users", "/tmp/users.xlsx", "excel", Some("Sheet1".to_string()))
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

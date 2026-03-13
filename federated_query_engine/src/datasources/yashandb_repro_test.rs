#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use datafusion::prelude::SessionContext;
    use datafusion::datasource::TableProvider;
    use datafusion::logical_expr::{col, lit, Expr};
    use crate::datasources::yashandb::YashanDataSource;

    #[tokio::test]
    async fn test_create_pushdown_provider_simple_unwrapping() {
        let sql = "SELECT * FROM TPCC.BMSQL_ITEM";
        let config = r#"{"host": "localhost", "port": 1688, "user": "sys", "pass": "oracle"}"#;
        
        let provider = YashanDataSource::create_pushdown_provider(config, sql.to_string())
            .await
            .expect("Failed to create provider");

        // The name of the provider should be the table name, NOT a subquery
        // If wrapped, it would be "(SELECT * FROM TPCC.BMSQL_ITEM) PUSHDOWN_ALIAS"
        // If unwrapped, it should be "TPCC.BMSQL_ITEM"
        // Note: The logic in yashandb.rs uses the extracted name as the "name" of the table.
        
        let name = provider.as_any().downcast_ref::<crate::datasources::yashandb::YashanTable>().unwrap().name();
        println!("Provider Name: {}", name);
        
        assert_eq!(name, "TPCC.BMSQL_ITEM", "Should have unwrapped the simple SQL");
    }

    #[tokio::test]
    async fn test_create_pushdown_provider_complex_wrapping() {
        let sql = "SELECT * FROM TPCC.BMSQL_ITEM WHERE I_ID > 100";
        let config = r#"{"host": "localhost", "port": 1688, "user": "sys", "pass": "oracle"}"#;
        
        let provider = YashanDataSource::create_pushdown_provider(config, sql.to_string())
            .await
            .expect("Failed to create provider");

        let name = provider.as_any().downcast_ref::<crate::datasources::yashandb::YashanTable>().unwrap().name();
        println!("Provider Name: {}", name);
        
        assert!(name.contains("PUSHDOWN_ALIAS"), "Should wrap complex SQL");
    }
}

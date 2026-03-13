#[cfg(test)]
mod tests {
    use crate::build_session_context;
    use crate::datasources::oracle::OracleDataSource;
    use crate::datasources::parquet::ParquetDataSource;
    use crate::datasources::DataSource;
    use arrow::array::{Array, Float64Array, Int64Array, UInt64Array};
    use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::datasource::MemTable;
    use datafusion::error::DataFusionError;
    use datafusion::error::Result;
    use datafusion::execution::runtime_env::RuntimeEnvBuilder;
    use datafusion::prelude::{SessionConfig, SessionContext};
    use std::collections::HashMap;
    use std::env;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::SystemTime;

    struct OracleConfig {
        user: String,
        pass: String,
        host: String,
        port: u16,
        service: String,
        schema: String,
        prefix: String,
    }

    fn build_tpcc_context() -> SessionContext {
        let mut config = SessionConfig::new();
        let _ = config
            .options_mut()
            .set("datafusion.sql_parser.enable_ident_normalization", "true");
        let runtime_env = RuntimeEnvBuilder::new().build().unwrap();
        let ctx = build_session_context(config, Arc::new(runtime_env));
        register_tpcc_tables(&ctx).unwrap();
        ctx
    }

    fn register_table(ctx: &SessionContext, name: &str, schema: Arc<Schema>) -> Result<()> {
        let batch = RecordBatch::new_empty(schema.clone());
        let table = MemTable::try_new(schema, vec![vec![batch]])?;
        ctx.register_table(name, Arc::new(table))?;
        Ok(())
    }

    fn register_tpcc_tables(ctx: &SessionContext) -> Result<()> {
        register_table(
            ctx,
            "warehouse",
            Arc::new(Schema::new(vec![
                Field::new("w_id", DataType::Int32, false),
                Field::new("w_name", DataType::Utf8, true),
                Field::new("w_ytd", DataType::Float64, true),
                Field::new("w_tax", DataType::Float64, true),
            ])),
        )?;
        register_table(
            ctx,
            "district",
            Arc::new(Schema::new(vec![
                Field::new("d_id", DataType::Int32, false),
                Field::new("d_w_id", DataType::Int32, false),
                Field::new("d_name", DataType::Utf8, true),
                Field::new("d_ytd", DataType::Float64, true),
                Field::new("d_tax", DataType::Float64, true),
                Field::new("d_next_o_id", DataType::Int32, true),
            ])),
        )?;
        register_table(
            ctx,
            "customer",
            Arc::new(Schema::new(vec![
                Field::new("c_id", DataType::Int32, false),
                Field::new("c_d_id", DataType::Int32, false),
                Field::new("c_w_id", DataType::Int32, false),
                Field::new("c_first", DataType::Utf8, true),
                Field::new("c_last", DataType::Utf8, true),
                Field::new("c_credit", DataType::Utf8, true),
                Field::new("c_balance", DataType::Float64, true),
                Field::new("c_ytd_payment", DataType::Float64, true),
                Field::new("c_payment_cnt", DataType::Int32, true),
                Field::new("c_delivery_cnt", DataType::Int32, true),
            ])),
        )?;
        register_table(
            ctx,
            "history",
            Arc::new(Schema::new(vec![
                Field::new("h_c_id", DataType::Int32, false),
                Field::new("h_c_d_id", DataType::Int32, false),
                Field::new("h_c_w_id", DataType::Int32, false),
                Field::new("h_d_id", DataType::Int32, false),
                Field::new("h_w_id", DataType::Int32, false),
                Field::new(
                    "h_date",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new("h_amount", DataType::Float64, true),
                Field::new("h_data", DataType::Utf8, true),
            ])),
        )?;
        register_table(
            ctx,
            "order",
            Arc::new(Schema::new(vec![
                Field::new("o_id", DataType::Int32, false),
                Field::new("o_d_id", DataType::Int32, false),
                Field::new("o_w_id", DataType::Int32, false),
                Field::new("o_c_id", DataType::Int32, false),
                Field::new(
                    "o_entry_d",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new("o_carrier_id", DataType::Int32, true),
                Field::new("o_ol_cnt", DataType::Int32, true),
                Field::new("o_all_local", DataType::Int32, true),
            ])),
        )?;
        register_table(
            ctx,
            "new_order",
            Arc::new(Schema::new(vec![
                Field::new("no_o_id", DataType::Int32, false),
                Field::new("no_d_id", DataType::Int32, false),
                Field::new("no_w_id", DataType::Int32, false),
            ])),
        )?;
        register_table(
            ctx,
            "order_line",
            Arc::new(Schema::new(vec![
                Field::new("ol_o_id", DataType::Int32, false),
                Field::new("ol_d_id", DataType::Int32, false),
                Field::new("ol_w_id", DataType::Int32, false),
                Field::new("ol_number", DataType::Int32, false),
                Field::new("ol_i_id", DataType::Int32, false),
                Field::new("ol_supply_w_id", DataType::Int32, false),
                Field::new(
                    "ol_delivery_d",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new("ol_quantity", DataType::Float64, true),
                Field::new("ol_amount", DataType::Float64, true),
                Field::new("ol_dist_info", DataType::Utf8, true),
            ])),
        )?;
        register_table(
            ctx,
            "item",
            Arc::new(Schema::new(vec![
                Field::new("i_id", DataType::Int32, false),
                Field::new("i_name", DataType::Utf8, true),
                Field::new("i_price", DataType::Float64, true),
                Field::new("i_data", DataType::Utf8, true),
                Field::new("i_im_id", DataType::Int32, true),
            ])),
        )?;
        register_table(
            ctx,
            "stock",
            Arc::new(Schema::new(vec![
                Field::new("s_i_id", DataType::Int32, false),
                Field::new("s_w_id", DataType::Int32, false),
                Field::new("s_quantity", DataType::Int32, true),
                Field::new("s_ytd", DataType::Float64, true),
                Field::new("s_order_cnt", DataType::Int32, true),
                Field::new("s_remote_cnt", DataType::Int32, true),
                Field::new("s_data", DataType::Utf8, true),
            ])),
        )?;
        Ok(())
    }

    fn load_sql_cases() -> Vec<String> {
        let base = std::env::current_dir().unwrap();
        let candidates = vec![
            base.join(PathBuf::from("scripts"))
                .join("sql_cases_tpcc.sql"),
            base.join(PathBuf::from("federated_query_engine"))
                .join("scripts")
                .join("sql_cases_tpcc.sql"),
        ];
        let path = candidates.into_iter().find(|p| p.exists()).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        content
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    fn find_cache_dir() -> Option<PathBuf> {
        let base = std::env::current_dir().unwrap();
        let candidates = vec![
            base.join(PathBuf::from("cache")).join("yashandb"),
            base.join(PathBuf::from("federated_query_engine"))
                .join("cache")
                .join("yashandb"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }

    fn split_safe_name(file_name: &str) -> Option<String> {
        let name = file_name.strip_suffix(".parquet")?;
        let mut parts = name.rsplitn(2, '_');
        let hash = parts.next()?;
        let prefix = parts.next()?;
        if !hash.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(prefix.to_string())
    }

    fn table_name_for_safe_name(safe_name: &str) -> Option<&'static str> {
        let mappings = [
            ("bmsql_warehouse", "warehouse"),
            ("bmsql_district", "district"),
            ("bmsql_customer", "customer"),
            ("bmsql_history", "history"),
            ("bmsql_order_line", "order_line"),
            ("bmsql_order", "order"),
            ("bmsql_new_order", "new_order"),
            ("bmsql_item", "item"),
            ("bmsql_stock", "stock"),
        ];
        for (suffix, table_name) in mappings {
            if safe_name.ends_with(suffix) {
                return Some(table_name);
            }
        }
        None
    }

    fn discover_tpcc_parquet_tables(cache_dir: &PathBuf) -> Result<HashMap<String, PathBuf>> {
        let entries = std::fs::read_dir(cache_dir)
            .map_err(|e| DataFusionError::Execution(format!("Failed to read cache dir: {}", e)))?;
        let mut selected: HashMap<String, (PathBuf, SystemTime)> = HashMap::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| DataFusionError::Execution(format!("Invalid dir entry: {}", e)))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("parquet") {
                continue;
            }
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };
            let safe_name = match split_safe_name(file_name) {
                Some(name) => name,
                None => continue,
            };
            let table_name = match table_name_for_safe_name(&safe_name) {
                Some(name) => name.to_string(),
                None => continue,
            };
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            match selected.get(&table_name) {
                Some((_, prev)) if *prev >= modified => {}
                _ => {
                    selected.insert(table_name, (path, modified));
                }
            }
        }
        Ok(selected
            .into_iter()
            .map(|(name, (path, _))| (name, path))
            .collect())
    }

    fn load_oracle_config() -> Option<OracleConfig> {
        let user = env::var("ORACLE_USER").ok()?.trim().to_string();
        let pass = env::var("ORACLE_PASS").ok()?.trim().to_string();
        let host = env::var("ORACLE_HOST").ok()?.trim().to_string();
        let service = env::var("ORACLE_SERVICE").ok()?.trim().to_string();
        if user.is_empty() || pass.is_empty() || host.is_empty() || service.is_empty() {
            return None;
        }
        let port = env::var("ORACLE_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(1521);
        let schema = env::var("ORACLE_TPCC_SCHEMA")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "TPCC".to_string());
        let prefix = env::var("ORACLE_TPCC_TABLE_PREFIX")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "BMSQL_".to_string());
        Some(OracleConfig {
            user,
            pass,
            host,
            port,
            service,
            schema,
            prefix,
        })
    }

    fn oracle_table_name(cfg: &OracleConfig, logical: &str) -> String {
        let base = format!("{}{}", cfg.prefix, logical.to_uppercase());
        if cfg.schema.trim().is_empty() {
            base
        } else {
            format!("{}.{}", cfg.schema, base)
        }
    }

    fn required_tpcc_tables() -> Vec<&'static str> {
        vec![
            "warehouse",
            "district",
            "customer",
            "history",
            "order_line",
            "order",
            "new_order",
            "item",
            "stock",
        ]
    }

    async fn execute_scalar_i64(ctx: &SessionContext, sql: &str) -> Result<i64> {
        let df = ctx.sql(sql).await?;
        let batches = df.collect().await?;
        let batch = batches
            .first()
            .ok_or_else(|| DataFusionError::Execution("Empty result batches".to_string()))?;
        if batch.num_rows() == 0 {
            return Err(DataFusionError::Execution("Empty result rows".to_string()));
        }
        let array = batch.column(0);
        if let Some(arr) = array.as_any().downcast_ref::<Int64Array>() {
            if arr.is_null(0) {
                return Err(DataFusionError::Execution("Null scalar value".to_string()));
            }
            return Ok(arr.value(0));
        }
        if let Some(arr) = array.as_any().downcast_ref::<UInt64Array>() {
            if arr.is_null(0) {
                return Err(DataFusionError::Execution("Null scalar value".to_string()));
            }
            return Ok(arr.value(0) as i64);
        }
        Err(DataFusionError::Execution(
            "Unexpected scalar type for i64".to_string(),
        ))
    }

    async fn execute_scalar_f64(ctx: &SessionContext, sql: &str) -> Result<f64> {
        let df = ctx.sql(sql).await?;
        let batches = df.collect().await?;
        let batch = batches
            .first()
            .ok_or_else(|| DataFusionError::Execution("Empty result batches".to_string()))?;
        if batch.num_rows() == 0 {
            return Err(DataFusionError::Execution("Empty result rows".to_string()));
        }
        let array = batch.column(0);
        if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
            if arr.is_null(0) {
                return Err(DataFusionError::Execution("Null scalar value".to_string()));
            }
            return Ok(arr.value(0));
        }
        Err(DataFusionError::Execution(
            "Unexpected scalar type for f64".to_string(),
        ))
    }

    fn batches_have_non_null(batches: &[RecordBatch]) -> bool {
        let mut has_rows = false;
        let mut has_non_null = false;
        for batch in batches {
            if batch.num_rows() == 0 {
                continue;
            }
            has_rows = true;
            for column in batch.columns() {
                for row in 0..batch.num_rows() {
                    if !column.is_null(row) {
                        has_non_null = true;
                        break;
                    }
                }
                if has_non_null {
                    break;
                }
            }
            if has_non_null {
                break;
            }
        }
        has_rows && has_non_null
    }

    async fn assert_sql_non_null(ctx: &SessionContext, sql: &str) -> Result<()> {
        let df = ctx.sql(sql).await?;
        let batches = df.collect().await?;
        if batches.is_empty() {
            return Err(DataFusionError::Execution(
                "Empty result batches".to_string(),
            ));
        }
        if !batches_have_non_null(&batches) {
            return Err(DataFusionError::Execution(
                "All rows are null or empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn assert_alias_column(
        ctx: &SessionContext,
        sql: &str,
        expected_alias: &str,
    ) -> Result<()> {
        let df = ctx.sql(sql).await?;
        let batches = df.collect().await?;
        let batch = batches
            .first()
            .ok_or_else(|| DataFusionError::Execution("Empty result batches".to_string()))?;
        let schema = batch.schema();
        let mut found = false;
        for field in schema.fields() {
            if field.name() == expected_alias {
                found = true;
                break;
            }
        }
        if !found {
            return Err(DataFusionError::Execution(format!(
                "Missing alias column: {}",
                expected_alias
            )));
        }
        Ok(())
    }

    #[tokio::test]
    async fn tpcc_sql_parser_from_cases_file() {
        let ctx = build_tpcc_context();
        let sqls = load_sql_cases();
        let mut failures = Vec::new();
        for sql in sqls {
            if let Err(e) = ctx.state().create_logical_plan(&sql).await {
                failures.push(format!("{} => {}", sql, e));
            }
        }
        assert!(
            failures.is_empty(),
            "parse failures:\n{}",
            failures.join("\n")
        );
    }

    #[tokio::test]
    async fn tpcc_sql_integration_with_parquet_cache() -> Result<()> {
        let cache_dir = find_cache_dir().ok_or_else(|| {
            DataFusionError::Execution("Missing cache/yashandb directory".to_string())
        })?;
        let tables = discover_tpcc_parquet_tables(&cache_dir)?;
        let required = required_tpcc_tables();
        let mut missing = Vec::new();
        for name in &required {
            if !tables.contains_key(*name) {
                missing.push(*name);
            }
        }
        let mut config = SessionConfig::new();
        let _ = config
            .options_mut()
            .set("datafusion.sql_parser.enable_ident_normalization", "true");
        let runtime_env = RuntimeEnvBuilder::new().build().unwrap();
        let ctx = build_session_context(config, Arc::new(runtime_env));

        if missing.is_empty() {
            let mut table_names: Vec<String> = tables.keys().cloned().collect();
            table_names.sort();
            for name in &table_names {
                let path = tables.get(name).unwrap().to_string_lossy().to_string();
                let ds = ParquetDataSource::new(name.to_string(), path);
                ds.register(&ctx).await?;
            }
        } else {
            let cfg = match load_oracle_config() {
                Some(cfg) => cfg,
                None => return Ok(()),
            };
            for name in &required {
                let table = oracle_table_name(&cfg, name);
                let ds = OracleDataSource::new(
                    (*name).to_string(),
                    cfg.user.clone(),
                    cfg.pass.clone(),
                    cfg.host.clone(),
                    cfg.port,
                    cfg.service.clone(),
                    table,
                )?;
                ds.register(&ctx).await?;
            }
        }

        let warehouse_count = execute_scalar_i64(&ctx, "select count(*) from warehouse").await?;
        assert!(warehouse_count > 0);
        let sum_ytd = execute_scalar_f64(&ctx, "select sum(w_ytd) from warehouse").await?;
        assert!(sum_ytd.is_finite());
        let derived = execute_scalar_f64(
            &ctx,
            "select total_ytd from (select sum(w_ytd) as total_ytd from warehouse) t",
        )
        .await?;
        assert!((derived - sum_ytd).abs() < 1e-6);

        assert_alias_column(
            &ctx,
            "select sum(w_ytd) as total_ytd from warehouse",
            "total_ytd",
        )
        .await?;
        assert_alias_column(
            &ctx,
            "select avg(d_tax) as avg_tax from district",
            "avg_tax",
        )
        .await?;

        if tables.contains_key("district") {
            let district_count = execute_scalar_i64(&ctx, "select count(*) from district").await?;
            assert!(district_count > 0);
        }

        if tables.contains_key("history") {
            let history_count = execute_scalar_i64(&ctx, "select count(*) from history").await?;
            assert!(history_count > 0);
        }

        let sqls = load_sql_cases();
        let mut failures = Vec::new();
        for sql in sqls {
            if let Err(e) = assert_sql_non_null(&ctx, &sql).await {
                failures.push(format!("{} => {}", sql, e));
            }
        }
        assert!(
            failures.is_empty(),
            "sql execution failures:\n{}",
            failures.join("\n")
        );

        Ok(())
    }
}

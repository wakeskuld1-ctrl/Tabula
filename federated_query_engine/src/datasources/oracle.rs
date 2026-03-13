use crate::resources::oracle_manager::OracleConnectionManager;
use crate::resources::pool_manager::{DbConfig, DbType, PoolManager};
use arrow::datatypes::{i256, DataType, Field, Schema, SchemaRef, TimeUnit};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use datafusion::datasource::memory::MemorySchemaProvider;
use datafusion::catalog::Session;
use datafusion::common::stats::Precision;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::logical_expr::TableProviderFilterPushDown;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::logical_expr::Expr;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
    SendableRecordBatchStream, Statistics,
};
use datafusion::prelude::*;
use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use arrow::array::{
    new_null_array, ArrayRef, BinaryArray, Decimal128Array, Decimal256Array, Float32Array,
    Float64Array, Int64Array, StringArray,
};
use oracle::sql_type::ToSql;
use oracle::Connection;
use regex::Regex;
use tokio_stream::wrappers::ReceiverStream;

use crate::datasources::{map_numeric_precision_scale, DataSource};

type OracleTableStats = (String, String, Option<i64>, Option<i64>);

const CHANGE_NOTES: &[&str] = &[
    "变更备注 2026-02-28: 移除未使用的Oracle缓存文件名函数，原因是清理clippy告警并减少冗余",
    "变更备注 2026-02-28: 修复OracleExec仅构建Float64/Utf8列导致Arrow列数不匹配，原因是包含Int64/Decimal列会直接查询失败",
    "变更备注 2026-02-28: 添加OracleExec列缓冲单元测试，原因是防止再出现列数不匹配",
    "变更备注 2026-03-12: 收敛Oracle执行日志噪音并输出关键指标，原因是避免fetched进度刷屏影响排障",
];
const ORACLE_PROGRESS_LOG_STEP: usize = 1_000_000;
const ORACLE_PROGRESS_LOG_INTERVAL_SECS: u64 = 5;

use crate::datasources::sql_dialect::OracleDialect;
use datafusion::scalar::ScalarValue;

#[derive(Clone, Debug)]
pub struct OracleDataSource {
    pub table_name: String,
    pub sql_table: String,
    pub pool: Option<r2d2::Pool<OracleConnectionManager>>, // Changed to Option for testing
    #[allow(dead_code)]
    pub schema: SchemaRef,
    #[allow(dead_code)]
    pub projected_schema: Option<SchemaRef>,
    pub user: String,
    pub pass: String,
    pub host: String,
    pub port: u16,
    pub service: String,
}

impl OracleDataSource {
    /// 创建 OracleDataSource 实例
    ///
    /// **实现方案**:
    /// 1. 初始化数据库配置 `DbConfig`。
    /// 2. 从 `PoolManager` 获取连接池。
    /// 3. 启动一个阻塞线程进行 Schema 推断 (`infer_schema`)，避免阻塞异步运行时。
    /// 4. 返回包含 Schema 和连接池的 `OracleDataSource`。
    ///
    /// **关键问题点**:
    /// - 阻塞操作：Schema 推断涉及网络 IO，必须在 `spawn_blocking` 或 `std::thread` 中执行。
    /// - 错误处理：连接池获取失败或 Schema 推断失败将导致创建失败。
    pub fn new(
        table_name: String,
        user: String,
        pass: String,
        host: String,
        port: u16,
        service: String,
        sql_table: String,
    ) -> Result<Self> {
        let config = DbConfig {
            db_type: DbType::Oracle,
            host: host.clone(),
            port,
            user: user.clone(),
            pass: pass.clone(),
            service: Some(service.clone()),
            max_pool_size: 10, // Default size
        };

        let _ = CHANGE_NOTES;
        let pool = PoolManager::instance()
            .get_oracle_pool(&config)
            .map_err(DataFusionError::Execution)?;

        // Use blocking task for schema inference to avoid blocking runtime
        let (schema, pool_clone, sql_table_clone) =
            std::thread::scope(|s| {
                s.spawn(|| {
                    let conn = pool
                        .get()
                        .map_err(|e| DataFusionError::Execution(format!("Pool error: {}", e)))?;
                    let schema = Self::infer_schema(&conn, &sql_table)?;
                    Ok::<(SchemaRef, r2d2::Pool<OracleConnectionManager>, String), DataFusionError>(
                        (schema, pool.clone(), sql_table.clone()),
                    )
                })
                .join()
                .unwrap()
            })?;

        Ok(Self {
            table_name,
            sql_table: sql_table_clone,
            pool: Some(pool_clone),
            schema,
            projected_schema: None,
            user,
            pass,
            host,
            port,
            service,
        })
    }

    // Helper for testing without DB connection
    #[cfg(test)]
    pub fn new_test(table_name: String, schema: SchemaRef) -> Self {
        Self {
            table_name: table_name.clone(),
            sql_table: table_name,
            pool: None,
            schema,
            projected_schema: None,
            user: "test".to_string(),
            pass: "test".to_string(),
            host: "localhost".to_string(),
            port: 1521,
            service: "orcl".to_string(),
        }
    }

    /// 清理和修复 Oracle SQL 语句
    ///
    /// **实现方案**:
    /// 使用正则表达式移除 Oracle 不支持的 `AS` 关键字（用于表别名）。
    /// Oracle 语法中，表别名不能带 `AS`，例如 `FROM table AS t` 是错误的，应为 `FROM table t`。
    ///
    /// **调用链路**:
    /// - 被 `create_pushdown_provider` 调用。
    ///
    /// **关键问题点**:
    /// - 正则表达式：覆盖了子查询别名、JOIN 表别名、普通表别名等多种情况。
    pub fn clean_oracle_sql(sql: &str) -> String {
        let mut clean = sql.to_string();

        // 1. Remove "AS" for subquery aliases: ") AS alias" -> ") alias"
        if let Ok(re) = Regex::new(r"(?i)\)\s+AS\s+([a-zA-Z0-9_]+)") {
            if re.is_match(&clean) {
                let before = clean.clone();
                clean = re.replace_all(&clean, ") $1").to_string();
                crate::app_log!(
                    "Oracle SQL Fix: Removed AS from subquery alias.\nBefore: {}\nAfter: {}",
                    before,
                    clean
                );
            }
        }

        // 2. Remove "AS" for table aliases in FROM/JOIN (supports comma and quoted tables)
        // Regex: (?i)(\bFROM|\bJOIN|,)\s+([a-zA-Z0-9_.]+|"[^"]+")\s+AS\s+([a-zA-Z0-9_]+)
        if let Ok(re) =
            Regex::new(r#"(?i)(\bFROM|\bJOIN|,)\s+([a-zA-Z0-9_.]+|"[^"]+")\s+AS\s+([a-zA-Z0-9_]+)"#)
        {
            if re.is_match(&clean) {
                let before = clean.clone();
                clean = re.replace_all(&clean, "$1 $2 $3").to_string();
                crate::app_log!(
                    "Oracle SQL Fix: Removed AS from table alias.\nBefore: {}\nAfter: {}",
                    before,
                    clean
                );
            }
        }

        clean
    }
}

#[cfg(test)]
mod tests {
    use arrow::array::Array;

    #[test]
    fn test_clean_oracle_sql_comma_join() {
        let sql = "SELECT * FROM t1 AS a, t2 AS b";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM t1 a, t2 b");
    }

    #[test]
    fn test_clean_oracle_sql_quoted_table() {
        let sql = "SELECT * FROM \"MyTable\" AS t";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM \"MyTable\" t");
    }

    use super::*;

    #[test]
    fn test_clean_oracle_sql_subquery_alias() {
        let sql = "SELECT * FROM (SELECT 1 FROM DUAL) AS t";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM (SELECT 1 FROM DUAL) t");
    }

    #[test]
    fn test_clean_oracle_sql_table_alias() {
        let sql = "SELECT * FROM my_table AS t";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM my_table t");
    }

    #[test]
    fn test_clean_oracle_sql_join_alias() {
        let sql = "SELECT * FROM t1 JOIN t2 AS t ON t1.id = t.id";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM t1 JOIN t2 t ON t1.id = t.id");
    }

    #[test]
    fn test_clean_oracle_sql_case_insensitive() {
        let sql = "SELECT * FROM table as t";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM table t");
    }

    #[test]
    fn test_clean_oracle_sql_no_change() {
        let sql = "SELECT * FROM table t";
        let cleaned = OracleDataSource::clean_oracle_sql(sql);
        assert_eq!(cleaned, "SELECT * FROM table t");
    }

    #[test]
    fn test_parse_type_precision_scale_number() {
        let (dt, precision, scale) = OracleDataSource::parse_type_precision_scale("NUMBER(10,2)");
        assert_eq!(dt, "NUMBER");
        assert_eq!(precision, Some(10));
        assert_eq!(scale, Some(2));
    }

    #[test]
    fn test_map_oracle_type_number_int64() {
        let dt = OracleDataSource::map_oracle_type("NUMBER", Some(10), Some(0));
        assert_eq!(dt, DataType::Int64);
    }

    #[test]
    fn test_map_oracle_type_number_decimal() {
        let dt = OracleDataSource::map_oracle_type("NUMBER", Some(10), Some(2));
        assert_eq!(dt, DataType::Decimal128(10_u8, 2_i8));
    }

    #[test]
    fn test_map_oracle_type_number_no_precision() {
        let dt = OracleDataSource::map_oracle_type("NUMBER", None, None);
        assert_eq!(dt, DataType::Float64);
    }

    #[test]
    fn test_decimal_string_to_i128_scale() {
        let v = decimal_string_to_i128("12.34", 2);
        assert_eq!(v, Some(1234_i128));
    }

    #[test]
    fn test_decimal_string_to_i256_scale() {
        let v = decimal_string_to_i256("-0.50", 2);
        assert_eq!(v, i256::from_string("-050"));
    }

    #[test]
    fn test_decimal_string_scientific_notation_none() {
        let v = decimal_string_to_i128("1.2e3", 2);
        assert_eq!(v, None);
    }

    #[test]
    fn test_build_arrays_from_buffers_column_count() {
        let col_types = vec![DataType::Int64, DataType::Utf8, DataType::Float64];
        let mut buffers = init_column_buffers(&col_types, 4);
        if let ColumnBuffer::Int64(values) = &mut buffers[0] {
            values.push(Some(10));
        }
        if let ColumnBuffer::Utf8(values) = &mut buffers[1] {
            values.push(Some("a".to_string()));
        }
        if let ColumnBuffer::Float64(values) = &mut buffers[2] {
            values.push(Some(3.5));
        }
        let columns = build_arrays_from_buffers(&mut buffers, 1).unwrap();
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].len(), 1);
        assert_eq!(columns[1].len(), 1);
        assert_eq!(columns[2].len(), 1);
    }

    #[tokio::test]
    async fn test_oracle_scan_pushdown() {
        use datafusion::logical_expr::{col, lit};
        use datafusion::prelude::SessionContext;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let source = OracleDataSource::new_test("test_table".to_string(), schema.clone());
        let table = OracleTable::new(source, schema, None);

        let ctx = SessionContext::new();
        let state = ctx.state();

        // Create a filter expression: id = 1
        let filter = col("id").eq(lit(1));
        let filters = vec![filter];

        let plan = table.scan(&state, None, &filters, None).await.unwrap();
        
        let debug_str = format!("{:?}", plan);
        
        assert!(debug_str.contains("WHERE") && debug_str.contains(":1"), 
            "OracleExec should contain pushed down SQL with placeholders, but got: {}", debug_str);
        assert!(debug_str.contains("Int32(1)"), 
            "OracleExec should contain parameters, but got: {}", debug_str);
    }

    #[test]
    fn test_build_arrays_from_buffers_date_timestamp_nulls() {
        let col_types = vec![
            DataType::Date32,
            DataType::Timestamp(TimeUnit::Nanosecond, None),
        ];
        let mut buffers = init_column_buffers(&col_types, 2);
        let columns = build_arrays_from_buffers(&mut buffers, 2).unwrap();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].len(), 2);
        assert!(columns[0].is_null(0));
        assert!(columns[0].is_null(1));
        assert_eq!(columns[1].len(), 2);
        assert!(columns[1].is_null(0));
        assert!(columns[1].is_null(1));
    }
}


impl OracleDataSource {
    /// 获取数据库连接
    ///
    /// **实现方案**:
    /// 从内部连接池 (`r2d2`) 获取一个可用连接。
    pub fn get_connection(
        &self,
    ) -> std::result::Result<r2d2::PooledConnection<OracleConnectionManager>, oracle::Error> {
        if let Some(pool) = &self.pool {
            pool.get()
                .map_err(|e| oracle::Error::new(oracle::ErrorKind::Other, format!("Pool error: {}", e)))
        } else {
            Err(oracle::Error::new(
                oracle::ErrorKind::Other,
                "Connection pool is not available (test mode)",
            ))
        }
    }

    /// 测试连接并列出表
    ///
    /// **实现方案**:
    /// 1. 尝试建立连接（支持 Service Name 和 SID 两种格式的自动回退）。
    /// 2. 查询 `ALL_TABLES` 获取当前用户下的表列表。
    /// 3. 如果当前用户无表，尝试查询系统中的前 100 张表作为示例。
    ///
    /// **调用链路**:
    /// - API 层调用，用于连接测试和表发现。
    pub fn test_connection(
        user: &str,
        pass: &str,
        host: &str,
        port: u16,
        service: &str,
    ) -> Result<Vec<OracleTableStats>> {
        let conn_string_svc = format!("//{}:{}/{}", host, port, service);

        let conn = match Connection::connect(user, pass, &conn_string_svc) {
            Ok(c) => c,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("ORA-12514") {
                    // Fallback: Try SID format
                    let conn_string_sid = format!("{}:{}:{}", host, port, service);
                    crate::app_log!(
                        "Retrying Oracle connection with SID format: {}",
                        conn_string_sid
                    );
                    Connection::connect(user, pass, &conn_string_sid)
                        .map_err(Self::map_oracle_error)?
                } else {
                    return Err(Self::map_oracle_error(e));
                }
            }
        };

        let sql = format!("SELECT OWNER, TABLE_NAME, NUM_ROWS, AVG_ROW_LEN FROM ALL_TABLES WHERE OWNER = UPPER('{}') AND ROWNUM <= 100", user);
        let params: [&dyn ToSql; 0] = [];
        let rows = conn
            .query(&sql, &params[..])
            .map_err(Self::map_oracle_error)?;

        let mut tables = Vec::new();
        for row_result in rows {
            let row = row_result.map_err(Self::map_oracle_error)?;
            let owner: String = row.get(0).map_err(Self::map_oracle_error)?;
            let table_name: String = row.get(1).map_err(Self::map_oracle_error)?;
            let num_rows: Option<i64> = row.get(2).ok();
            let avg_row_len: Option<i64> = row.get(3).ok();
            tables.push((owner, table_name, num_rows, avg_row_len));
        }

        if tables.is_empty() {
            let sql_any = "SELECT OWNER, TABLE_NAME, NUM_ROWS, AVG_ROW_LEN FROM ALL_TABLES WHERE ROWNUM <= 100";
            let params: [&dyn ToSql; 0] = [];
            let rows_any = conn
                .query(sql_any, &params[..])
                .map_err(Self::map_oracle_error)?;
            for row_result in rows_any {
                let row = row_result.map_err(Self::map_oracle_error)?;
                let owner: String = row.get(0).map_err(Self::map_oracle_error)?;
                let table_name: String = row.get(1).map_err(Self::map_oracle_error)?;
                let num_rows: Option<i64> = row.get(2).ok();
                let avg_row_len: Option<i64> = row.get(3).ok();
                tables.push((owner, table_name, num_rows, avg_row_len));
            }
        }

        Ok(tables)
    }

    /// 获取表统计信息
    ///
    /// **实现方案**:
    /// 1. 使用 `EXPLAIN PLAN` 分析查询语句。
    /// 2. 从 `PLAN_TABLE` 中提取 `CARDINALITY` (行数) 和 `BYTES` (字节数)。
    /// 3. 计算平均行长。
    ///
    /// **关键问题点**:
    /// - 依赖 `PLAN_TABLE` 存在。
    /// - 使用 `STATEMENT_ID` 隔离并发查询。
    pub fn get_table_stats(&self) -> Result<Option<(i64, i64)>> {
        let conn = self.get_connection().map_err(Self::map_oracle_error)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let stmt_id = format!("STATS_{}", timestamp);

        let explain_sql = format!(
            "EXPLAIN PLAN SET STATEMENT_ID = '{}' FOR SELECT * FROM {}",
            stmt_id, self.sql_table
        );

        crate::app_log!("Fetching Oracle stats with: {}", explain_sql);

        if let Err(e) = conn.execute(&explain_sql, &[]) {
            crate::app_log!("Oracle Explain failed: {}", e);
            return Ok(None);
        }

        let fetch_sql = format!(
            "SELECT CARDINALITY, BYTES FROM PLAN_TABLE WHERE STATEMENT_ID = '{}' AND ID = 0",
            stmt_id
        );

        let mut stats = None;
        let fetch_params: [&dyn ToSql; 0] = [];
        if let Ok(rows) = conn.query(&fetch_sql, &fetch_params[..]) {
            for row in rows.flatten() {
                let card: Option<i64> = row.get(0).ok();
                let bytes: Option<i64> = row.get(1).ok();
                if let (Some(c), Some(b)) = (card, bytes) {
                    let avg = if c > 0 { b / c } else { 100 };
                    stats = Some((c, avg));
                    crate::app_log!("Oracle Stats fetched: rows={}, bytes={}, avg={}", c, b, avg);
                }
            }
        }

        let _ = conn.execute(
            &format!("DELETE FROM PLAN_TABLE WHERE STATEMENT_ID = '{}'", stmt_id),
            &[],
        );
        let _ = conn.commit();

        Ok(stats)
    }

    fn map_oracle_error(e: oracle::Error) -> DataFusionError {
        let msg = e.to_string();
        if msg.contains("DPI-1047") {
            DataFusionError::Execution(format!("Oracle Client Error: 未找到 Oracle 客户端库 (DPI-1047)。请确保已安装 Oracle Instant Client 并将其目录添加到 PATH 环境变量中。原始错误: {}", msg))
        } else {
            DataFusionError::Execution(format!("Oracle error: {}", msg))
        }
    }

    fn parse_type_precision_scale(oracle_type: &str) -> (String, Option<i64>, Option<i64>) {
        let trimmed = oracle_type.trim();
        if let Some(start) = trimmed.find('(') {
            if trimmed.ends_with(')') {
                let base = trimmed[..start].trim().to_uppercase();
                let inner = &trimmed[start + 1..trimmed.len() - 1];
                let mut parts = inner.split(',').map(|p| p.trim());
                let precision = parts.next().and_then(|v| v.parse::<i64>().ok());
                let scale = parts.next().and_then(|v| v.parse::<i64>().ok());
                return (base, precision, scale);
            }
        }
        (trimmed.to_uppercase(), None, None)
    }

    fn map_oracle_type(data_type: &str, precision: Option<i64>, scale: Option<i64>) -> DataType {
        let dt = data_type.trim().to_uppercase();
        match dt.as_str() {
            "NUMBER" => map_numeric_precision_scale(precision, scale),
            "FLOAT" => DataType::Float64,
            "BINARY_FLOAT" => DataType::Float32,
            "BINARY_DOUBLE" => DataType::Float64,
            "INT" | "INTEGER" | "SMALLINT" | "BIGINT" => DataType::Int64,
            "DATE" => DataType::Date32,
            "TIMESTAMP" => DataType::Timestamp(TimeUnit::Nanosecond, None),
            "TIMESTAMP WITH TIME ZONE" => DataType::Timestamp(TimeUnit::Nanosecond, None),
            "TIMESTAMP WITH LOCAL TIME ZONE" => DataType::Timestamp(TimeUnit::Nanosecond, None),
            dt if dt.starts_with("TIMESTAMP") => DataType::Timestamp(TimeUnit::Nanosecond, None),
            "CHAR" | "NCHAR" | "VARCHAR2" | "NVARCHAR2" | "CLOB" | "NCLOB" | "LONG" => {
                DataType::Utf8
            }
            "RAW" | "BLOB" => DataType::Binary,
            _ => DataType::Utf8,
        }
    }

    fn parse_simple_table_name(sql_table: &str) -> Option<(Option<String>, String)> {
        let trimmed = sql_table.trim().trim_end_matches(';');
        if trimmed.starts_with('(') {
            return None;
        }
        if trimmed.contains(' ') {
            return None;
        }
        if trimmed.contains('"') {
            return None;
        }
        let mut parts = trimmed.split('.');
        let first = parts.next()?;
        let second = parts.next();
        if parts.next().is_some() {
            return None;
        }
        match second {
            Some(table) => Some((Some(first.to_uppercase()), table.to_uppercase())),
            None => Some((None, first.to_uppercase())),
        }
    }

    /// 从系统字典表推断 Schema
    ///
    /// **实现方案**:
    /// 1. 解析表名（支持 `Schema.Table` 格式）。
    /// 2. 查询 `ALL_TAB_COLUMNS` 获取列信息。
    /// 3. 映射 Oracle 数据类型到 Arrow 数据类型。
    ///
    /// **调用链路**:
    /// - 被 `infer_schema` 优先调用。
    fn infer_schema_from_dictionary(
        conn: &Connection,
        sql_table: &str,
    ) -> Result<Option<SchemaRef>> {
        let (owner_opt, table) = match Self::parse_simple_table_name(sql_table) {
            Some(parsed) => parsed,
            None => return Ok(None),
        };
        let owner = match owner_opt {
            Some(o) => o,
            None => {
                let params: [&dyn ToSql; 0] = [];
                let mut rows = conn
                    .query("SELECT USER FROM DUAL", &params[..])
                    .map_err(Self::map_oracle_error)?;
                let row = match rows.next() {
                    Some(row_result) => row_result.map_err(Self::map_oracle_error)?,
                    None => return Ok(None),
                };
                row.get(0).map_err(Self::map_oracle_error)?
            }
        };
        let sql = "SELECT COLUMN_NAME, DATA_TYPE, DATA_PRECISION, DATA_SCALE FROM ALL_TAB_COLUMNS WHERE OWNER = :1 AND TABLE_NAME = :2 ORDER BY COLUMN_ID";
        let params: [&dyn ToSql; 2] = [&owner, &table];
        let rows = conn
            .query(sql, &params[..])
            .map_err(Self::map_oracle_error)?;
        let mut fields = Vec::new();
        for row_result in rows {
            let row = row_result.map_err(Self::map_oracle_error)?;
            let name: String = row.get(0).map_err(Self::map_oracle_error)?;
            let data_type: String = row.get(1).map_err(Self::map_oracle_error)?;
            let precision: Option<i64> = row.get(2).ok();
            let scale: Option<i64> = row.get(3).ok();
            let dt = Self::map_oracle_type(&data_type, precision, scale);
            fields.push(Field::new(name.to_lowercase(), dt, true));
        }
        if fields.is_empty() {
            return Ok(None);
        }
        Ok(Some(Arc::new(Schema::new(fields))))
    }

    /// 推断表 Schema
    ///
    /// **实现方案**:
    /// 1. 优先尝试 `infer_schema_from_dictionary` 从元数据字典获取。
    /// 2. 如果失败（例如是复杂查询），则执行 `SELECT * ... WHERE ROWNUM <= 1`，通过结果集元数据推断。
    ///
    /// **调用链路**:
    /// - 被 `new` 和 `create_pushdown_provider` 调用。
    fn infer_schema(conn: &Connection, sql_table: &str) -> Result<SchemaRef> {
        if let Ok(Some(schema)) = Self::infer_schema_from_dictionary(conn, sql_table) {
            return Ok(schema);
        }
        let sql = format!("SELECT * FROM {} WHERE ROWNUM <= 1", sql_table);
        let params: [&dyn ToSql; 0] = [];
        let rows = conn
            .query(&sql, &params[..])
            .map_err(|e| DataFusionError::Execution(format!("Oracle query error: {}", e)))?;

        let col_infos = rows.column_info();
        let mut fields = Vec::with_capacity(col_infos.len());

        crate::app_log!("Inferring schema for Oracle table: {}", sql_table);
        for (idx, col) in col_infos.iter().enumerate() {
            let name = col.name().to_string();
            let name_lower = name.to_lowercase();
            let oracle_type = col.oracle_type().to_string();
            let (base_type, precision, scale) = Self::parse_type_precision_scale(&oracle_type);

            crate::app_log!(
                "  Col {}: {} ({}) -> {}",
                idx,
                name,
                oracle_type,
                name_lower
            );

            let dt = Self::map_oracle_type(&base_type, precision, scale);

            fields.push(Field::new(&name_lower, dt, true));
        }
        Ok(Arc::new(Schema::new(fields)))
    }

    /// 为下推查询创建 Provider
    ///
    /// **实现方案**:
    /// 1. 解析配置 JSON。
    /// 2. 清理 SQL 语句（移除 AS 别名等）。
    /// 3. 创建 `OracleDataSource`，将 SQL 作为子查询包装 `(clean_sql)`。
    /// 4. 异步推断 Schema 和统计信息。
    /// 5. 返回 `OracleTable`。
    ///
    /// **调用链路**:
    /// - 被 `QueryRewriter` 调用，处理复杂子查询下推。
    pub async fn create_pushdown_provider(
        config_json: String,
        sql: String,
    ) -> Result<Arc<dyn TableProvider>> {
        let config: serde_json::Value = serde_json::from_str(&config_json).map_err(|e| {
            DataFusionError::Execution(format!("Failed to parse Oracle config: {}", e))
        })?;

        let user = config["user"].as_str().unwrap_or("").to_string();
        let pass = config["pass"].as_str().unwrap_or("").to_string();
        let host = config["host"].as_str().unwrap_or("").to_string();
        let port = config["port"].as_u64().unwrap_or(1521) as u16;
        let service = config["service"].as_str().unwrap_or("").to_string();

        let clean_sql_str = Self::clean_oracle_sql(&sql);
        let clean_sql = clean_sql_str.trim().trim_end_matches(';');
        let wrapped_sql = format!("({})", clean_sql);

        let source = OracleDataSource::new(
            "pushdown_query".to_string(),
            user,
            pass,
            host,
            port,
            service,
            wrapped_sql,
        )?;

        let this = source.clone();
        let schema_join = tokio::task::spawn_blocking(move || {
            let conn = this.get_connection().map_err(Self::map_oracle_error)?;
            Self::infer_schema(&conn, &this.sql_table)
        });

        let this_stats = source.clone();
        let stats_join = tokio::task::spawn_blocking(move || this_stats.get_table_stats());

        let schema = schema_join
            .await
            .map_err(|e| DataFusionError::Execution(format!("Task join error: {}", e)))??;
        let stats = stats_join
            .await
            .map_err(|e| DataFusionError::Execution(format!("Task join error: {}", e)))?
            .unwrap_or(None);

        Ok(Arc::new(OracleTable::new(source, schema, stats)))
    }
}

#[async_trait]
impl DataSource for OracleDataSource {
    fn name(&self) -> &str {
        &self.table_name
    }

    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        self.register_with_name(ctx, &self.table_name).await
    }
}

impl OracleDataSource {
    async fn create_provider(&self) -> Result<OracleTable> {
        let this = self.clone();
        let schema_join = tokio::task::spawn_blocking(move || {
            let conn = this.get_connection().map_err(Self::map_oracle_error)?;
            Self::infer_schema(&conn, &this.sql_table)
        });

        let this_stats = self.clone();
        let stats_join = tokio::task::spawn_blocking(move || this_stats.get_table_stats());

        let schema = schema_join
            .await
            .map_err(|e| DataFusionError::Execution(format!("Task join error: {}", e)))??;
        let stats = stats_join
            .await
            .map_err(|e| DataFusionError::Execution(format!("Task join error: {}", e)))?
            .unwrap_or(None);

        Ok(OracleTable::new(self.clone(), schema, stats))
    }

    pub async fn register_with_name(
        &self,
        ctx: &SessionContext,
        register_name: &str,
    ) -> Result<()> {
        let table_provider = self.create_provider().await?;
        ctx.register_table(register_name, Arc::new(table_provider))?;
        Ok(())
    }

    pub async fn register_with_schema(
        &self,
        ctx: &SessionContext,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        let table_provider = self.create_provider().await?;
        
        let catalog = ctx.catalog("datafusion").ok_or(DataFusionError::Internal("Default catalog not found".to_string()))?;
        
        if catalog.schema(schema_name).is_none() {
             let schema_provider = Arc::new(MemorySchemaProvider::new());
             catalog.register_schema(schema_name, schema_provider)?;
             crate::app_log!("Created new schema in DataFusion: {}", schema_name);
        }
        
        let schema_provider = catalog.schema(schema_name).ok_or(DataFusionError::Internal(format!("Schema {} not found", schema_name)))?;
        schema_provider.register_table(table_name.to_string(), Arc::new(table_provider))?;
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct OracleTable {
    source: OracleDataSource,
    schema: SchemaRef,
    stats: Option<(i64, i64)>,
}

impl OracleTable {
    pub fn new(source: OracleDataSource, schema: SchemaRef, stats: Option<(i64, i64)>) -> Self {
        Self {
            source,
            schema,
            stats,
        }
    }

    pub fn source(&self) -> &OracleDataSource {
        &self.source
    }
}

enum ColumnBuffer {
    Int64(Vec<Option<i64>>),
    Float64(Vec<Option<f64>>),
    Float32(Vec<Option<f32>>),
    Utf8(Vec<Option<String>>),
    Binary(Vec<Option<Vec<u8>>>),
    Decimal128 {
        values: Vec<Option<i128>>,
        precision: u8,
        scale: i8,
    },
    Decimal256 {
        values: Vec<Option<i256>>,
        precision: u8,
        scale: i8,
    },
    Null(DataType),
}

fn normalize_decimal_string(value: &str, scale: i8) -> Option<String> {
    let mut s = value.trim();
    if s.is_empty() {
        return None;
    }
    if s.contains('e') || s.contains('E') {
        return None;
    }
    let mut sign = "";
    if let Some(rest) = s.strip_prefix('-') {
        sign = "-";
        s = rest;
    } else if let Some(rest) = s.strip_prefix('+') {
        s = rest;
    }
    let mut parts = s.splitn(2, '.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = parts.next().unwrap_or("");
    let int_digits = if int_part.is_empty() { "0" } else { int_part };
    let scale = scale.max(0) as usize;
    let mut frac = frac_part.to_string();
    if frac.len() < scale {
        frac.push_str(&"0".repeat(scale - frac.len()));
    } else if frac.len() > scale {
        frac.truncate(scale);
    }
    let mut digits = format!("{}{}", int_digits, frac);
    while digits.starts_with('0') && digits.len() > 1 {
        digits.remove(0);
    }
    Some(format!("{}{}", sign, digits))
}

fn decimal_string_to_i128(value: &str, scale: i8) -> Option<i128> {
    let normalized = normalize_decimal_string(value, scale)?;
    normalized.parse::<i128>().ok()
}

fn decimal_string_to_i256(value: &str, scale: i8) -> Option<i256> {
    let normalized = normalize_decimal_string(value, scale)?;
    i256::from_string(&normalized)
}

fn init_column_buffers(col_types: &[DataType], batch_size: usize) -> Vec<ColumnBuffer> {
    col_types
        .iter()
        .map(|dt| match dt {
            DataType::Int64 => ColumnBuffer::Int64(Vec::with_capacity(batch_size)),
            DataType::Float64 => ColumnBuffer::Float64(Vec::with_capacity(batch_size)),
            DataType::Float32 => ColumnBuffer::Float32(Vec::with_capacity(batch_size)),
            DataType::Utf8 => ColumnBuffer::Utf8(Vec::with_capacity(batch_size)),
            DataType::Binary => ColumnBuffer::Binary(Vec::with_capacity(batch_size)),
            DataType::Decimal128(precision, scale) => ColumnBuffer::Decimal128 {
                values: Vec::with_capacity(batch_size),
                precision: *precision,
                scale: *scale,
            },
            DataType::Decimal256(precision, scale) => ColumnBuffer::Decimal256 {
                values: Vec::with_capacity(batch_size),
                precision: *precision,
                scale: *scale,
            },
            _ => ColumnBuffer::Null(dt.clone()),
        })
        .collect()
}

fn build_arrays_from_buffers(
    buffers: &mut [ColumnBuffer],
    row_count: usize,
) -> Result<Vec<ArrayRef>> {
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(buffers.len());
    for buffer in buffers.iter_mut() {
        match buffer {
            ColumnBuffer::Int64(values) => {
                columns.push(Arc::new(Int64Array::from(values.clone())));
                values.clear();
            }
            ColumnBuffer::Float64(values) => {
                columns.push(Arc::new(Float64Array::from(values.clone())));
                values.clear();
            }
            ColumnBuffer::Float32(values) => {
                columns.push(Arc::new(Float32Array::from(values.clone())));
                values.clear();
            }
            ColumnBuffer::Utf8(values) => {
                columns.push(Arc::new(StringArray::from(values.clone())));
                values.clear();
            }
            ColumnBuffer::Binary(values) => {
                let array = BinaryArray::from_iter(values.iter().map(|v| v.as_deref()));
                columns.push(Arc::new(array));
                values.clear();
            }
            ColumnBuffer::Decimal128 {
                values,
                precision,
                scale,
            } => {
                let array =
                    Decimal128Array::from(values.clone()).with_precision_and_scale(
                        *precision,
                        *scale,
                    );
                let array = array.map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                columns.push(Arc::new(array));
                values.clear();
            }
            ColumnBuffer::Decimal256 {
                values,
                precision,
                scale,
            } => {
                let array =
                    Decimal256Array::from(values.clone()).with_precision_and_scale(
                        *precision,
                        *scale,
                    );
                let array = array.map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                columns.push(Arc::new(array));
                values.clear();
            }
            ColumnBuffer::Null(dt) => {
                columns.push(new_null_array(dt, row_count));
            }
        }
    }
    Ok(columns)
}

#[async_trait]
impl TableProvider for OracleTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        let dialect = OracleDialect::new(false);
        Ok(filters
            .iter()
            .map(|f| {
                if dialect.expr_to_sql_with_params(f).is_ok() {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect())
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let projected_schema = if let Some(indices) = projection {
            let fields: Vec<Field> = indices
                .iter()
                .map(|i| self.schema.field(*i).clone())
                .collect();
            Arc::new(Schema::new(fields))
        } else {
            self.schema.clone()
        };

        let dialect = OracleDialect::new(false);
        let mut pushdown_exprs = Vec::new();
        for filter in filters {
            if dialect.expr_to_sql_with_params(filter).is_ok() {
                pushdown_exprs.push(filter.clone());
            } else {
                crate::app_log!("Skipping unsupported filter for pushdown: {:?}", filter);
            }
        }

        let mut source = self.source.clone();
        let mut params = Vec::new();

        if !pushdown_exprs.is_empty() {
            let combined_filter = pushdown_exprs
                .into_iter()
                .reduce(|acc, expr| acc.and(expr))
                .unwrap();

            if let Ok((where_clause, p)) = dialect.expr_to_sql_with_params(&combined_filter) {
                // Simplify: just append WHERE to the subquery/table instead of wrapping in another SELECT
                // This avoids ORA-00907 and generates cleaner SQL (e.g. "T WHERE cond" instead of "(SELECT * FROM T WHERE cond)")
                let wrapped_sql = format!("{} WHERE {}", source.sql_table, where_clause);
                source.sql_table = wrapped_sql;
                params = p;
                crate::app_log!("Oracle Pushdown SQL: {}", source.sql_table);
            }
        }

        Ok(Arc::new(OracleExec::new(
            source,
            projected_schema,
            projection.cloned(),
            limit,
            self.stats,
            params,
        )))
    }
}

#[derive(Debug, Clone)]
pub struct OracleExec {
    source: OracleDataSource,
    schema: SchemaRef,
    projection: Option<Vec<usize>>,
    _limit: Option<usize>,
    properties: PlanProperties,
    _metrics: ExecutionPlanMetricsSet,
    stats: Option<(i64, i64)>,
    params: Vec<ScalarValue>,
}

impl OracleExec {
    pub fn new(
        source: OracleDataSource,
        schema: SchemaRef,
        projection: Option<Vec<usize>>,
        limit: Option<usize>,
        stats: Option<(i64, i64)>,
        params: Vec<ScalarValue>,
    ) -> Self {
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema.clone()),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Self {
            source,
            schema,
            projection,
            _limit: limit,
            properties,
            _metrics: ExecutionPlanMetricsSet::new(),
            stats,
            params,
        }
    }
}

impl DisplayAs for OracleExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "OracleExec: table={}", self.source.table_name)
            }
            DisplayFormatType::TreeRender => todo!(),
        }
    }
}

impl ExecutionPlan for OracleExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        "OracleExec"
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn statistics(&self) -> Result<Statistics> {
        match self.stats {
            Some((rows, avg_len)) => {
                let num_rows = Precision::Exact(rows as usize);
                let total_byte_size = Precision::Inexact((rows * avg_len) as usize);

                Ok(Statistics {
                    num_rows,
                    total_byte_size,
                    column_statistics: vec![],
                })
            }
            None => Ok(Statistics::new_unknown(&self.schema())),
        }
    }

    /// 执行 Oracle 查询并流式返回数据
    ///
    /// **实现方案**:
    /// 1. 创建 `mpsc` 通道。
    /// 2. 启动独立线程 (`std::thread::spawn`) 执行阻塞的 JDBC 操作：
    ///    - 获取连接。
    ///    - 准备 SQL 语句和参数。
    ///    - 执行查询。
    ///    - 分批次 (`batch_size=8192`) 读取结果集，将 Oracle 类型转换为 Arrow 数组。
    ///    - 发送 `RecordBatch` 到通道。
    ///
    /// **关键问题点**:
    /// - 线程模型：使用 OS 线程而非 Tokio 任务，因为 `oracle` crate 的操作是同步阻塞的，且可能长时间运行。
    /// - 类型转换：手动处理 Decimal 到 Float/Int 的转换，以及 Null 值处理。
    /// - 错误传播：捕获 JDBC 错误并通过通道发送给接收端。
    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let (tx, rx) = mpsc::channel(2);
        let source = self.source.clone();
        let schema = self.schema.clone();
        let projection = self.projection.clone();
        let params_vec = self.params.clone();

        let target_schema = if let Some(_proj) = &projection {
            schema.clone()
        } else {
            schema.clone()
        };

        let col_names: Vec<String> = target_schema
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();
        let col_types: Vec<DataType> = target_schema
            .fields()
            .iter()
            .map(|f| f.data_type().clone())
            .collect();

        std::thread::spawn(move || {
            let conn_res = source.get_connection();
            if let Err(e) = conn_res {
                let _ = tx.blocking_send(Err(DataFusionError::Execution(format!(
                    "Oracle connection error: {}",
                    e
                ))));
                return;
            }
            let conn = conn_res.unwrap();

            let columns_sql = if col_names.is_empty() {
                "*".to_string()
            } else {
                col_names.join(", ")
            };

            let sql = format!("SELECT {} FROM {}", columns_sql, source.sql_table);

            let sql_preview: String = sql.chars().take(200).collect();
            let sql_preview = if sql.chars().count() > 200 {
                format!("{}...", sql_preview)
            } else {
                sql_preview
            };
            crate::app_log!(
                "OracleExec start: table={}, cols={}, params={}, sql={}",
                source.table_name,
                target_schema.fields().len(),
                params_vec.len(),
                sql_preview
            );

            let mut sql_params: Vec<Box<dyn ToSql>> = Vec::with_capacity(params_vec.len());
            for p in &params_vec {
                 match p {
                     ScalarValue::Int32(Some(v)) => sql_params.push(Box::new(*v) as Box<dyn ToSql>),
                     ScalarValue::Int64(Some(v)) => sql_params.push(Box::new(*v) as Box<dyn ToSql>),
                     ScalarValue::Float64(Some(v)) => sql_params.push(Box::new(*v) as Box<dyn ToSql>),
                     ScalarValue::Utf8(Some(s)) => sql_params.push(Box::new(s.clone()) as Box<dyn ToSql>),
                     ScalarValue::Boolean(Some(v)) => {
                         let val: i32 = if *v { 1 } else { 0 };
                         sql_params.push(Box::new(val) as Box<dyn ToSql>);
                     }
                     ScalarValue::Decimal128(Some(v), _, scale) => {
                        if *scale == 0 {
                            if let Ok(val) = i64::try_from(*v) {
                                sql_params.push(Box::new(val) as Box<dyn ToSql>);
                            } else {
                                sql_params.push(Box::new(v.to_string()) as Box<dyn ToSql>);
                            }
                        } else {
                             let f = (*v as f64) / 10f64.powi(*scale as i32);
                             sql_params.push(Box::new(f) as Box<dyn ToSql>);
                        }
                     }
                     _ => {
                         let _ = tx.blocking_send(Err(DataFusionError::Execution(format!(
                             "Unsupported parameter type for Oracle: {:?}",
                             p
                         ))));
                         return;
                     }
                 }
            }
            let sql_params_refs: Vec<&dyn ToSql> = sql_params.iter().map(|b| b.as_ref()).collect();

            let rows_res = conn.query(&sql, &sql_params_refs);
            if let Err(e) = rows_res {
                let _ = tx.blocking_send(Err(DataFusionError::Execution(format!(
                    "Oracle query error: {}",
                    e
                ))));
                return;
            }
            let rows = rows_res.unwrap();

            let batch_size = 8192;
            let mut current_batch_rows = 0;

            let mut buffers = init_column_buffers(&col_types, batch_size);

            let start_time = Instant::now();
            let mut total_rows: usize = 0;
            let mut last_log_rows: usize = 0;
            let mut last_log_time = Instant::now();

            for row_result in rows {
                let row = match row_result {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(DataFusionError::Execution(format!(
                            "Oracle row error: {}",
                            e
                        ))));
                        return;
                    }
                };

                for (col_idx, buffer) in buffers.iter_mut().enumerate() {
                    match buffer {
                        ColumnBuffer::Int64(values) => {
                            let val: Option<i64> = row.get(col_idx).ok();
                            values.push(val);
                        }
                        ColumnBuffer::Float64(values) => {
                            let val: Option<f64> = row.get(col_idx).ok();
                            values.push(val);
                        }
                        ColumnBuffer::Float32(values) => {
                            let val: Option<f32> = row.get(col_idx).ok();
                            values.push(val);
                        }
                        ColumnBuffer::Utf8(values) => {
                            let val: Option<String> = row.get(col_idx).ok();
                            values.push(val);
                        }
                        ColumnBuffer::Binary(values) => {
                            let val: Option<Vec<u8>> = row.get(col_idx).ok();
                            values.push(val);
                        }
                        ColumnBuffer::Decimal128 { values, precision: _, scale } => {
                             let val_str: Option<String> = row.get(col_idx).ok();
                             let val = if let Some(s) = val_str {
                                 decimal_string_to_i128(&s, *scale)
                             } else {
                                 None
                             };
                             values.push(val);
                        }
                        ColumnBuffer::Decimal256 { values, precision: _, scale } => {
                             let val_str: Option<String> = row.get(col_idx).ok();
                             let val = if let Some(s) = val_str {
                                 decimal_string_to_i256(&s, *scale)
                             } else {
                                 None
                             };
                             values.push(val);
                        }
                        ColumnBuffer::Null(_) => {}
                    }
                }

                current_batch_rows += 1;
                total_rows += 1;

                if current_batch_rows >= batch_size {
                     match build_arrays_from_buffers(&mut buffers, current_batch_rows) {
                         Ok(columns) => {
                             let batch = RecordBatch::try_new(target_schema.clone(), columns)
                                 .map_err(|e| DataFusionError::ArrowError(Box::new(e), None));
                             if tx.blocking_send(batch).is_err() {
                                 return; // Receiver dropped
                             }
                         }
                         Err(e) => {
                             let _ = tx.blocking_send(Err(e));
                             return;
                         }
                     }
                     current_batch_rows = 0;
                }

                if total_rows - last_log_rows >= ORACLE_PROGRESS_LOG_STEP
                    && last_log_time.elapsed().as_secs() >= ORACLE_PROGRESS_LOG_INTERVAL_SECS
                {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = if elapsed > 0.0 {
                        total_rows as f64 / elapsed
                    } else {
                        0.0
                    };
                    crate::app_log!(
                        "OracleExec progress: table={}, rows={}, elapsed_s={:.1}, rate_rps={:.0}",
                        source.table_name,
                        total_rows,
                        elapsed,
                        rate
                    );
                    last_log_rows = total_rows;
                    last_log_time = Instant::now();
                }
            }

            if current_batch_rows > 0 {
                 match build_arrays_from_buffers(&mut buffers, current_batch_rows) {
                     Ok(columns) => {
                         let batch = RecordBatch::try_new(target_schema.clone(), columns)
                             .map_err(|e| DataFusionError::ArrowError(Box::new(e), None));
                         let _ = tx.blocking_send(batch);
                     }
                     Err(e) => {
                         let _ = tx.blocking_send(Err(e));
                     }
                 }
            }
            let elapsed = start_time.elapsed().as_secs_f64();
            let avg_rate = if elapsed > 0.0 {
                total_rows as f64 / elapsed
            } else {
                0.0
            };
            crate::app_log!(
                "OracleExec finished: table={}, rows={}, elapsed_s={:.1}, avg_rate_rps={:.0}",
                source.table_name,
                total_rows,
                elapsed,
                avg_rate
            );
        });

        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            ReceiverStream::new(rx),
        )))
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use datafusion::prelude::*;
    use std::env;
    use arrow::array::StringArray;

    #[tokio::test]
    #[ignore]
    async fn test_oracle_pushdown_integration_tpcc() -> Result<()> {
        let host = env::var("ORACLE_HOST").unwrap_or_else(|_| "192.168.23.3".to_string());
        let port = env::var("ORACLE_PORT")
            .unwrap_or_else(|_| "1521".to_string())
            .parse::<u16>()
            .unwrap_or(1521);
        let service = env::var("ORACLE_SERVICE").unwrap_or_else(|_| "cyccbdata".to_string());
        let user = env::var("ORACLE_USER").unwrap_or_else(|_| "i2".to_string());
        let pass = env::var("ORACLE_PASS").unwrap_or_else(|_| "i2".to_string());

        println!("---------------------------------------------------");
        println!("Integration Test: Oracle Pushdown (TPCC Schema)");
        println!("Connecting to {}:{}/{} as {}", host, port, service, user);
        println!("---------------------------------------------------");

        // Discovery: List tables
        println!("Listing available tables...");
        match OracleDataSource::test_connection(&user, &pass, &host, port, &service) {
            Ok(tables) => {
                println!("Found {} tables.", tables.len());
                for (owner, table, _, _) in tables.iter().take(10) {
                    println!("  {}.{}", owner, table);
                }
            }
            Err(e) => println!("Failed to list tables: {}", e),
        }

        // 1. Register WAREHOUSE
        let table_warehouse = "WAREHOUSE";
        let sql_warehouse = "I2.T1"; 
        println!("Creating OracleDataSource for table: {}", table_warehouse);
        let source_w = OracleDataSource::new(
            table_warehouse.to_string(),
            user.clone(),
            pass.clone(),
            host.clone(),
            port,
            service.clone(),
            sql_warehouse.to_string(),
        )?;

        // 2. Register DISTRICT
        let table_district = "DISTRICT";
        let sql_district = "I2.T1";
        println!("Creating OracleDataSource for table: {}", table_district);
        let source_d = OracleDataSource::new(
            table_district.to_string(),
            user.clone(),
            pass.clone(),
            host.clone(),
            port,
            service.clone(),
            sql_district.to_string(),
        )?;

        let ctx = SessionContext::new();
        println!("Registering tables in DataFusion...");
        source_w.register(&ctx).await?;
        source_d.register(&ctx).await?;

        // 3. Execute Multi-Table Join Query
        // Using dummy join on same table since I don't see TPCC tables
        // T1 only has ID column based on previous error
        let sql = "SELECT a.ID, b.ID FROM WAREHOUSE a JOIN DISTRICT b ON a.ID = b.ID WHERE a.ID = 1";
        println!("Executing SQL via DataFusion: {}", sql);

        let df = ctx.sql(sql).await?;

        println!("Logical Plan:");
        let explain_df = df.clone().explain(false, false)?;
        let explain_batches = explain_df.collect().await?;
        let plan_str = explain_batches[0].column(1).as_any().downcast_ref::<StringArray>().unwrap().value(0);
        println!("{}", plan_str);
        
        println!("Collecting results...");
        let batches = df.collect().await?;
        println!("Result Batches: {}", batches.len());
        
        if !batches.is_empty() {
            let row_count: usize = batches.iter().map(|b| b.num_rows()).sum();
            println!("Total Rows: {}", row_count);
            arrow::util::pretty::print_batches(&batches)?;
        } else {
            println!("No rows returned.");
        }

        Ok(())
    }
}

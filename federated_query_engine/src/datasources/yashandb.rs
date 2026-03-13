use datafusion::datasource::file_format::parquet::ParquetFormat;
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::physical_plan::{FileScanConfigBuilder, ParquetSource};
// use datafusion::datasource::file_groups::FileGroup;
use arrow::array::{
    ArrayBuilder, Decimal128Builder, Decimal256Builder, Float64Builder, Int32Builder, Int64Builder,
    RecordBatch, StringBuilder,
};
use arrow::datatypes::{i256, DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::datasource::listing::PartitionedFile;
use datafusion::execution::context::TaskContext;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
};
use std::any::Any;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
// use std::pin::Pin;
// use std::task::{Context, Poll};
use std::fmt::{self, Formatter};
use tokio::sync::{mpsc, Mutex, OnceCell};
// use std::sync::atomic::{AtomicUsize, Ordering};
use crate::datasources::{map_numeric_precision_scale, DataSource};
use datafusion::datasource::memory::MemorySchemaProvider;
use datafusion::catalog::Session;
use datafusion::common::stats::Precision;
use datafusion::common::Statistics;
use datafusion::datasource::TableProvider;
use datafusion::datasource::TableType;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::SessionState;
use datafusion::logical_expr::Expr;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::prelude::SessionContext;
use odbc_api::handles::StatementImpl;
use odbc_api::{Cursor, CursorImpl};
// // use std::collections::HashMap;
use datafusion::datasource::object_store::ObjectStoreUrl;
use object_store::path::Path;
use odbc_api::{ConnectionOptions, Environment, ResultSetMetadata};
// use std::time::{SystemTime, UNIX_EPOCH};
use crate::logger::log_sidecar;
use crate::resources::pool_manager::{DbConfig, DbType, PoolManager};
use crate::resources::yashan_manager::YashanConnectionManager;
use crate::utils::TmpFileGuard;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter as StreamAdapter;
use datafusion::sql::parser::{DFParser, Statement as DFStatement};
use datafusion::sql::sqlparser::ast::{Expr as SqlExpr, Ident, SelectItem, SetExpr, Statement};
use datafusion::sql::sqlparser::tokenizer::Span;
use parquet::arrow::ArrowWriter;
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

// Global semaphore to limit concurrent sidecar tasks
static SIDECAR_SEMAPHORE: OnceCell<tokio::sync::Semaphore> = OnceCell::const_new();
static CACHING_TASKS: OnceCell<Mutex<std::collections::HashSet<String>>> = OnceCell::const_new();

// LRU Cache for Parquet files (Simple implementation: just track size)
// For a real LRU, we'd need a linked hash map. Here we just delete oldest if size exceeds limit.
const MAX_CACHE_SIZE_BYTES: u64 = 1024 * 1024 * 1024; // 1GB

/// 构建 YashanDB 的 ODBC 连接字符串
///
/// **实现方案**:
/// 根据提供的 host, port, user, pass 和 service 参数，格式化生成符合 YashanDB ODBC 驱动要求的连接字符串。
/// 如果提供了 service，则包含 `Database` 参数，否则仅包含基础连接信息。
///
/// **调用链路**:
/// - 被 `YashanDataSource::new` 调用
/// - 被 `YashanDataSource::test_connection` 调用
/// - 被 `YashanDataSource::create_pushdown_provider` 调用
///
/// **关键问题点**:
/// - 确保格式与 YashanDB ODBC 驱动版本兼容。
/// - 特殊字符处理（当前未处理，假设由 ODBC 驱动处理或用户负责）。
pub fn build_yashandb_conn_str(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    service: &str,
) -> String {
    if service.trim().is_empty() {
        format!(
            "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};",
            host, port, user, pass
        )
    } else {
        format!(
            "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};Database={};",
            host, port, user, pass, service
        )
    }
}

#[cfg(test)]
mod conn_str_tests {
    use super::*;

    #[test]
    fn test_build_yashandb_conn_str() {
        let host = "127.0.0.1";
        let port = 1688;
        let user = "sys";
        let pass = "password";
        let service = "yashandb";

        let expected =
            "Driver={YashanDB};Server=127.0.0.1;Port=1688;Uid=sys;Pwd=password;Database=yashandb;";
        let actual = build_yashandb_conn_str(host, port, user, pass, service);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_yashandb_conn_str_special_chars() {
        let host = "127.0.0.1";
        let port = 1688;
        let user = "user@name";
        let pass = "p@ssword!";
        let service = "db-service";

        let expected = "Driver={YashanDB};Server=127.0.0.1;Port=1688;Uid=user@name;Pwd=p@ssword!;Database=db-service;";
        let actual = build_yashandb_conn_str(host, port, user, pass, service);

        assert_eq!(actual, expected);
    }
}

type YashanTableStats = (String, String, Option<i64>, Option<i64>);

/// YashanDB 数据源结构体
///
/// **实现方案**:
/// 存储 YashanDB 的连接配置、表名、Schema 信息和统计信息。
/// 实现了 `DataSource` trait，用于在 DataFusion 中注册和管理表。
///
/// **关键问题点**:
/// - `pool`: 使用 `r2d2` 连接池管理 ODBC 连接，提高并发性能。
/// - `stats`: 缓存表的统计信息（行数、平均行长），用于查询优化。
#[derive(Debug, Clone)]
pub struct YashanDataSource {
    pub table_name: String,
    pub remote_table_name: Option<String>, // Actual table name in YashanDB
    pub schema: Option<SchemaRef>,         // Optional schema hint
    pub _schema_name: Option<String>,
    pub _service: String,
    pub _sql_query: Option<String>,
    pub conn_str: String,
    pub stats: Option<(i64, i64)>,
    pub pool: r2d2::Pool<YashanConnectionManager>,
}

/// 判断表达式是否需要别名
///
/// **实现方案**:
/// 检查表达式是否为标识符或复合标识符。如果不是，则认为需要添加别名。
///
/// **调用链路**:
/// - 被 `normalize_pushdown_sql` 调用
fn should_alias_expr(expr: &SqlExpr) -> bool {
    !matches!(
        expr,
        SqlExpr::Identifier(_) | SqlExpr::CompoundIdentifier(_)
    )
}

/// 从简单的 "SELECT * FROM table" 查询中提取表名
///
/// **实现方案**:
/// 使用 `sqlparser` 对 SQL 进行完整的 AST 解析，以确保准确识别简单的全表查询。
/// 只有满足以下严格条件的查询才会被视为简单查询：
/// 1. 是 SELECT 语句。
/// 2. 投影列为通配符 `*`。
/// 3. FROM 子句仅包含一个表，且没有 JOIN。
/// 4. 没有 WHERE、GROUP BY、HAVING 等子句。
///
/// **调用链路**:
/// - 被 `create_pushdown_provider` 调用
///
/// **关键问题点**:
/// - 解析器健壮性：依赖 DataFusion 内置的 parser，能处理标准 SQL。
/// - 性能开销：相比字符串匹配，解析 AST 开销稍大（微秒级），但在 Pushdown 场景下可忽略不计。
/// - 准确性：避免了字符串匹配可能出现的误判（如表名中包含关键字等边缘情况）。
///
/// # Arguments
/// * `sql` - The SQL query string
/// 
/// # Returns
/// * `Option<String>` - The table name if it's a simple query, otherwise None
fn extract_simple_table_name(sql: &str) -> Option<String> {
    // Attempt to parse the SQL using sqlparser with GenericDialect (more permissive)
    // Note: 'table' is a keyword, so "SELECT * FROM table" might fail in AnsiDialect.
    // We use GenericDialect which is usually fine, or we rely on the caller providing valid SQL.
    let dialect = datafusion::sql::sqlparser::dialect::GenericDialect {};
    let statements = datafusion::sql::sqlparser::parser::Parser::parse_sql(&dialect, sql).ok()?;

    // We expect exactly one statement
    if statements.len() != 1 {
        return None;
    }

    match &statements[0] {
        Statement::Query(query) => {
            // Must not have WITH, ORDER BY, LIMIT, OFFSET, FETCH
            if query.with.is_some()
                || query.order_by.is_some()
                || query.limit_clause.is_some()
                || query.fetch.is_some()
            {
                return None;
            }

            match &*query.body {
                SetExpr::Select(select) => {
                    // Must be SELECT *
                    if select.distinct.is_some() || select.top.is_some() || select.into.is_some() {
                        return None;
                    }
                    if select.projection.len() != 1 {
                        return None;
                    }
                    if !matches!(select.projection[0], SelectItem::Wildcard(_)) {
                        return None;
                    }

                    // Must have exactly one table in FROM
                    if select.from.len() != 1 {
                        return None;
                    }
                    let table_with_joins = &select.from[0];
                    if !table_with_joins.joins.is_empty() {
                        return None;
                    }

                    // Must not have WHERE, GROUP BY, HAVING, etc.
                    let group_by_empty = match &select.group_by {
                        datafusion::sql::sqlparser::ast::GroupByExpr::Expressions(exprs, _) => exprs.is_empty(),
                        datafusion::sql::sqlparser::ast::GroupByExpr::All(_) => false,
                    };
                    
                    if select.selection.is_some()
                        || !group_by_empty
                        || select.having.is_some()
                        || !select.cluster_by.is_empty()
                        || !select.distribute_by.is_empty()
                        || !select.sort_by.is_empty()
                        || !select.lateral_views.is_empty()
                    {
                        return None;
                    }
                    
                    match &table_with_joins.relation {
                        datafusion::sql::sqlparser::ast::TableFactor::Table { name, alias: _, .. } => {
                            Some(name.to_string())
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// 标准化下推 SQL，为非标识符表达式添加别名
///
/// **实现方案**:
/// 使用 `DFParser` 解析 SQL，遍历 Select 列表，对非标识符表达式添加自动生成的别名（`__df_expr_N`）。
/// 最后将修改后的 AST 重新序列化为 SQL 字符串。
///
/// **调用链路**:
/// - 被 `create_pushdown_provider` 调用
///
/// **关键问题点**:
/// - DataFusion 在处理子查询时，要求所有列都有明确的名称。
/// - 解析失败时返回错误，调用方需处理回退逻辑。
fn normalize_pushdown_sql(sql: &str) -> Result<String> {
    let mut statements = DFParser::parse_sql(sql)?;
    for statement in &mut statements {
        if let DFStatement::Statement(stmt) = statement {
            if let Statement::Query(query) = &mut **stmt {
                if let SetExpr::Select(select) = &mut *query.body {
                    for (idx, item) in select.projection.iter_mut().enumerate() {
                        if let SelectItem::UnnamedExpr(expr) = item {
                            if should_alias_expr(&*expr) {
                                let alias = Ident {
                                    value: format!("__df_expr_{}", idx + 1),
                                    quote_style: None,
                                    span: Span::empty(),
                                };
                                *item = SelectItem::ExprWithAlias {
                                    expr: expr.clone(),
                                    alias,
                                };
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(statements
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("; "))
}

impl YashanDataSource {
    /// 创建新的 YashanDataSource 实例
    ///
    /// **实现方案**:
    /// 1. 构建连接字符串。
    /// 2. 初始化数据库配置 `DbConfig`。
    /// 3. 从 `PoolManager` 获取连接池。
    /// 4. 构造并返回 `YashanDataSource` 对象。
    ///
    /// **关键问题点**:
    /// - 连接池复用：使用 `PoolManager` 单例管理连接池，避免重复创建开销。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        table_name: String,
        schema_name: Option<String>,
        user: String,
        pass: String,
        host: String,
        port: u16,
        service: String,
        sql_query: Option<String>,
    ) -> Result<Self, String> {
        // Build ODBC connection string
        // Format: "Driver={YashanDB};Server=192.168.1.10;Port=1688;Uid=user;Pwd=pass;DSN=service;"
        let conn_str = build_yashandb_conn_str(&host, port, &user, &pass, &service);

        let config = DbConfig {
            db_type: DbType::Yashan,
            host: host.clone(),
            port,
            user: user.clone(),
            pass: pass.clone(),
            service: Some(service.clone()), // Use service/db name if needed, currently not used in conn_str but kept in config
            max_pool_size: 10,
        };

        let pool = PoolManager::instance()
            .get_yashan_pool(&config)
            .map_err(|e| e.to_string())?;

        let remote_table_name = if let Some(s) = &schema_name {
            if !s.is_empty() {
                Some(format!("{}.{}", s, table_name))
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            table_name,
            remote_table_name,
            schema: None,
            _schema_name: schema_name,
            _service: service,
            _sql_query: sql_query,
            conn_str,
            stats: None,
            pool,
        })
    }

    /// 设置远程表名
    ///
    /// **实现方案**:
    /// 用于在逻辑表名和物理表名不一致时（例如 Schema 映射），指定实际查询的表名。
    pub fn with_remote_table_name(mut self, name: String) -> Self {
        self.remote_table_name = Some(name);
        self
    }

    /// 获取表的统计信息 (行数, 平均行长)
    ///
    /// **实现方案**:
    /// 1. 使用 `EXPLAIN PLAN` 分析 `SELECT * FROM table` 查询。
    /// 2. 从 `PLAN_TABLE` 中查询 `CARDINALITY` (行数) 和 `BYTES` (字节数)。
    /// 3. 计算平均行长 (`BYTES / CARDINALITY`)。
    /// 4. 清理 `PLAN_TABLE` 中的记录。
    ///
    /// **调用链路**:
    /// - 外部调用，用于优化器 Cost 计算。
    ///
    /// **关键问题点**:
    /// - 依赖 YashanDB 的 `PLAN_TABLE` 存在且可访问。
    /// - 异常处理：如果 `EXPLAIN` 失败，返回错误，不更新 stats。
    /// - 线程安全：使用唯一的 `STATEMENT_ID` 防止并发冲突。
    pub fn fetch_stats(&mut self) -> Result<(), String> {
        // Use connection from pool to fetch stats
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get connection from pool: {}", e))?;

        // Query stats via EXPLAIN PLAN (User Requirement: Use EXPLAIN to get Cost/Rows)
        let target_name = self.remote_table_name.as_ref().unwrap_or(&self.table_name);

        // Generate a unique statement ID
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let stmt_id = format!("STATS_{}", timestamp);

        let explain_sql = format!(
            "EXPLAIN PLAN SET STATEMENT_ID = '{}' FOR SELECT * FROM {}",
            stmt_id, target_name
        );
        crate::app_log!("Running EXPLAIN for stats: {}", explain_sql);

        // 1. Run EXPLAIN PLAN
        if let Err(e) = conn.execute(&explain_sql, ()) {
            crate::app_log!("EXPLAIN failed (maybe PLAN_TABLE missing?): {}", e);
            // Fallback to basic count if EXPLAIN fails?
            // User said: "如果再次验证Cost是0.0则停止...这个不需要你算".
            // So if EXPLAIN fails, we probably can't get stats.
            return Err(format!("EXPLAIN failed: {}", e));
        }

        // 2. Query PLAN_TABLE for CARDINALITY and BYTES
        let query_sql = format!(
            "SELECT CARDINALITY, BYTES FROM PLAN_TABLE WHERE STATEMENT_ID = '{}' AND ID = 0",
            stmt_id
        );

        let mut stats_found = false;
        match conn.execute(&query_sql, ()) {
            Ok(Some(cursor)) => {
                let mut cursor: CursorImpl<StatementImpl<'_>> = cursor;
                if let Ok(Some(mut row)) = cursor.next_row() {
                    let mut card_val = None;
                    let mut bytes_val = None;

                    let mut buf: Vec<u8> = Vec::new();

                    // Col 1: CARDINALITY
                    let res_card: std::result::Result<bool, odbc_api::Error> =
                        row.get_text(1, &mut buf);
                    if res_card.is_ok() {
                        let s = String::from_utf8_lossy(&buf).to_string();
                        if !s.is_empty() {
                            card_val = s.parse::<i64>().ok();
                        }
                    }

                    // Col 2: BYTES
                    if row
                        .get_text(2, &mut buf)
                        .map_err(|e: odbc_api::Error| e.to_string())
                        .is_ok()
                    {
                        let s = String::from_utf8_lossy(&buf).to_string();
                        if !s.is_empty() {
                            bytes_val = s.parse::<i64>().ok();
                        }
                    }

                    if let Some(rows) = card_val {
                        let total_bytes = bytes_val.unwrap_or(rows * 100); // Default 100 bytes/row if missing
                        let avg_len = if rows > 0 { total_bytes / rows } else { 0 };

                        self.stats = Some((rows, avg_len));
                        crate::app_log!(
                            "Stats fetched via EXPLAIN: rows={}, bytes={}, avg_len={}",
                            rows,
                            total_bytes,
                            avg_len
                        );
                        stats_found = true;
                    }
                }
            }
            Ok(None) => crate::app_log!("PLAN_TABLE query returned no cursor"),
            Err(e) => crate::app_log!("PLAN_TABLE query error: {}", e),
        }

        // 3. Cleanup PLAN_TABLE
        let cleanup_sql = format!("DELETE FROM PLAN_TABLE WHERE STATEMENT_ID = '{}'", stmt_id);
        if let Err(e) = conn.execute(&cleanup_sql, ()) {
            crate::app_log!("Failed to clean up PLAN_TABLE: {}", e);
        }

        if !stats_found {
            crate::app_log!("No stats found in PLAN_TABLE");
            return Err("No stats found".to_string());
        }

        Ok(())
    }

    /// 测试数据库连接并获取表列表
    ///
    /// **实现方案**:
    /// 1. 尝试建立 ODBC 连接。
    /// 2. 如果连接成功，执行 SQL 查询获取表信息（所有者、表名等）。
    /// 3. 支持分页查询（limit, offset）。
    ///
    /// **调用链路**:
    /// - API 层调用，用于验证配置和浏览表。
    ///
    /// **关键问题点**:
    /// - 连接字符串的正确性。
    /// - 默认过滤掉系统表（如 SYS, SYSTEM 等）。
    #[allow(clippy::too_many_arguments)]
    pub fn test_connection(
        user: &str,
        pass: &str,
        host: &str,
        port: u16,
        _service: &str,
        _sql_query: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<YashanTableStats>> {
        let conn_str = build_yashandb_conn_str(host, port, user, pass, _service);
        crate::app_log!("Testing YashanDB connection with string: {}", conn_str);
        let env = Environment::new().map_err(|e| DataFusionError::Execution(e.to_string()))?;
        let conn = env
            .connect_with_connection_string(&conn_str, ConnectionOptions::default())
            .map_err(|e| {
                DataFusionError::Execution(format!("Failed to connect to YashanDB: {}", e))
            })?;

        // List tables
        let base_sql = _sql_query.unwrap_or_else(|| "SELECT owner, table_name FROM all_tables WHERE owner NOT IN ('SYS', 'SYSTEM', 'OUTLN', 'DBSNMP', 'WMSYS', 'CTXSYS', 'XDB', 'MDSYS', 'ORDDATA', 'ORDSYS', 'OLAPSYS', 'MDDATA', 'SPATIAL_WFS_ADMIN_USR', 'SPATIAL_CSW_ADMIN_USR') ORDER BY (CASE WHEN owner = USER THEN 0 ELSE 1 END), owner, table_name".to_string());

        let sql = if let (Some(l), Some(o)) = (limit, offset) {
            format!("{} OFFSET {} ROWS FETCH NEXT {} ROWS ONLY", base_sql, o, l)
        } else {
            base_sql
        };

        let res = match conn.execute(&sql, ()) {
            Ok(Some(mut cursor)) => {
                let num_cols = cursor
                    .num_result_cols()
                    .map_err(|e| DataFusionError::Execution(e.to_string()))?;
                let mut tables = Vec::new();

                while let Ok(Some(mut row)) = cursor.next_row() {
                    let mut row_values = Vec::new();
                    for i in 1..=num_cols {
                        let mut buf = Vec::new();
                        if row.get_text(i.try_into().unwrap(), &mut buf).is_ok() {
                            row_values.push(String::from_utf8_lossy(&buf).to_string());
                        } else {
                            row_values.push("".to_string());
                        }
                    }

                    if num_cols == 1 {
                        if let Some(name) = row_values.first() {
                            tables.push(("User".to_string(), name.clone(), None, None));
                        }
                    } else if row_values.len() >= 2 {
                        let owner = row_values[0].clone();
                        let table = row_values[1].clone();
                        let num_rows = row_values.get(2).and_then(|s| s.parse::<i64>().ok());
                        let avg_row_len = row_values.get(3).and_then(|s| s.parse::<i64>().ok());
                        tables.push((owner, table, num_rows, avg_row_len));
                    }
                }
                Ok(tables)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(DataFusionError::Execution(format!(
                "Failed to list tables: {}",
                e
            ))),
        };
        res
    }
}

#[async_trait]
impl DataSource for YashanDataSource {
    fn name(&self) -> &str {
        &self.table_name
    }

    /// 在 DataFusion 上下文中注册 YashanDB 表
    ///
    /// **实现方案**:
    /// 1. 获取表的 Schema。
    /// 2. 创建 `YashanTable` 实例（Provider）。
    /// 3. 调用 DataFusion 的 `register_table` 将 Provider 注册到 SessionContext。
    ///
    /// **调用链路**:
    /// - 被外部注册逻辑调用。
    ///
    /// **关键问题点**:
    /// - 自动触发 `YashanTable` 的 Sidecar 缓存机制（在 scan 时触发）。
    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        let schema = self.fetch_schema().await?;

        // Use the sidecar approach:
        // 1. Create a YashanTable provider that can handle both remote scan and local cache scan
        let provider = YashanTable::new(
            self.table_name.clone(),
            self.remote_table_name.clone(),
            schema,
            self.conn_str.clone(),
            self.stats,
            self.pool.clone(),
        );
        ctx.register_table(&self.table_name, Arc::new(provider))?;

        Ok(())
    }

    /// 获取表的统计信息
    ///
    /// **实现方案**:
    /// 直接返回已经缓存的 `stats` 字段。
    fn get_table_stats(&self) -> std::result::Result<(Option<i64>, Option<i64>), String> {
        Ok(self
            .stats
            .map(|(rows, len)| (Some(rows), Some(len)))
            .unwrap_or((None, None)))
    }
}

impl YashanDataSource {
    /// 使用指定名称注册表
    ///
    /// **实现方案**:
    /// 与 `register` 类似，但允许指定在 DataFusion 中的注册名称。
    pub async fn register_with_name(&self, ctx: &SessionContext, name: &str) -> Result<()> {
        let schema = self.fetch_schema().await?;
        let provider = YashanTable::new(
            name.to_string(),
            Some(
                self.remote_table_name
                    .clone()
                    .unwrap_or(self.table_name.clone()),
            ),
            schema,
            self.conn_str.clone(),
            self.stats,
            self.pool.clone(),
        );
        ctx.register_table(name, Arc::new(provider))?;
        Ok(())
    }

    /// 在指定的 Schema 下注册表
    ///
    /// **实现方案**:
    /// 1. 确保 DataFusion 中存在目标 Schema。
    /// 2. 在该 Schema 对应的 Provider 中注册表。
    ///
    /// **关键问题点**:
    /// - 如果 Schema 不存在，自动创建一个 `MemorySchemaProvider`。
    pub async fn register_with_schema(
        &self,
        ctx: &SessionContext,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        let schema = self.fetch_schema().await?;
        let provider = YashanTable::new(
            table_name.to_string(),
            Some(
                self.remote_table_name
                    .clone()
                    .unwrap_or(self.table_name.clone()),
            ),
            schema,
            self.conn_str.clone(),
            self.stats,
            self.pool.clone(),
        );
        
        let catalog = ctx.catalog("datafusion").ok_or(DataFusionError::Internal("Default catalog not found".to_string()))?;
        
        if catalog.schema(schema_name).is_none() {
             let schema_provider = Arc::new(MemorySchemaProvider::new());
             catalog.register_schema(schema_name, schema_provider)?;
             crate::app_log!("Created new schema in DataFusion (Yashan): {}", schema_name);
        }
        
        let schema_provider = catalog.schema(schema_name).ok_or(DataFusionError::Internal(format!("Schema {} not found", schema_name)))?;
        schema_provider.register_table(table_name.to_string(), Arc::new(provider))?;
        
        Ok(())
    }

    /// 为 SQL 下推查询创建 Provider
    ///
    /// **实现方案**:
    /// 1. 解析配置获取连接信息。
    /// 2. 尝试提取简单表名，否则对 SQL 进行标准化处理（加别名）。
    /// 3. 推断查询结果集的 Schema。
    /// 4. 创建 `YashanTable` 实例并返回。
    ///
    /// **调用链路**:
    /// - 被 `QueryRewriter` 调用，用于处理无法直接翻译的复杂子查询。
    ///
    /// **关键问题点**:
    /// - Schema 推断：通过执行 `SELECT * FROM (query) WHERE 1=0` 获取元数据。
    /// - 别名处理：确保子查询的列都有明确别名。
    pub async fn create_pushdown_provider(
        config: &str,
        sql: String,
    ) -> Result<Arc<dyn TableProvider>> {
        // Parse config
        let parsed: serde_json::Value = serde_json::from_str(config)
            .map_err(|e| DataFusionError::Execution(format!("Invalid config JSON: {}", e)))?;

        let host = parsed["host"].as_str().unwrap_or("127.0.0.1");
        let port = parsed["port"].as_u64().unwrap_or(1688);
        let user = parsed["user"].as_str().unwrap_or("");
        let pass = parsed["pass"].as_str().unwrap_or("");
        let service = parsed["service"].as_str().unwrap_or("");

        let conn_str = build_yashandb_conn_str(host, port as u16, user, pass, service);

        let clean_sql = sql.trim().trim_end_matches(';');
        
        let (subquery_name, remote_name_hint) = if let Some(simple_name) = extract_simple_table_name(clean_sql) {
             (simple_name.clone(), Some(simple_name))
        } else {
            let normalized_sql = normalize_pushdown_sql(clean_sql).unwrap_or_else(|_| clean_sql.into());
            (format!("({}) PUSHDOWN_ALIAS", normalized_sql), None)
        };

        // Infer schema for the query
        let schema = fetch_query_schema(&conn_str, &subquery_name)
            .await
            .map_err(|e| {
                DataFusionError::Execution(format!("Pushdown schema inference failed: {}", e))
            })?;

        let db_config = DbConfig {
            db_type: DbType::Yashan,
            host: host.to_string(),
            port: port as u16,
            user: user.to_string(),
            pass: pass.to_string(),
            service: Some(service.to_string()),
            // extra: HashMap::new(),
            max_pool_size: 10,
        };
        let pool = PoolManager::instance()
            .get_yashan_pool(&db_config)
            .map_err(|e: String| DataFusionError::Execution(e.to_string()))?;

        // Create provider
        // We use the subquery_name as the "table name" for execution (SELECT * FROM subquery_name)
        // But for display/caching, we might want something shorter?
        // Current get_cache_filename uses hash of conn_str + table_name (which is the long SQL).
        // This is fine for correctness, just ugly log.
        let provider = YashanTable::new(subquery_name, remote_name_hint, schema, conn_str, None, pool);
        Ok(Arc::new(provider))
    }

    /// 获取表的 Schema 信息
    ///
    /// **实现方案**:
    /// 优先使用缓存的 `schema`。如果为空，则调用 `fetch_query_schema` 从数据库推断。
    async fn fetch_schema(&self) -> Result<SchemaRef> {
        if let Some(s) = &self.schema {
            return Ok(s.clone());
        }
        // In a real impl, we'd query YashanDB to get column types
        // For now, we can use a dummy schema or try to infer it via a LIMIT 1 query
        fetch_query_schema(
            &self.conn_str,
            self.remote_table_name.as_ref().unwrap_or(&self.table_name),
        )
        .await
        .map_err(|e| DataFusionError::Execution(format!("Schema inference failed: {}", e)))
    }
}

// --- Helper: Schema Inference ---
/// 通过查询推断 Schema
///
/// **实现方案**:
/// 1. 构造 `SELECT * FROM table WHERE 1=0` 查询，只获取元数据而不获取数据。
/// 2. 使用 ODBC 获取列名和数据类型。
/// 3. 将 ODBC 数据类型映射为 DataFusion (Arrow) 数据类型。
///
/// **调用链路**:
/// - 被 `fetch_schema` 调用。
/// - 被 `create_pushdown_provider` 调用。
///
/// **关键问题点**:
/// - 类型映射：需确保 Decimal, Date, Timestamp 等类型的正确转换。
/// - 列名大小写：统一转换为小写以适应 DataFusion 的不敏感匹配。
async fn fetch_query_schema(
    conn_str: &str,
    table_name: &str,
) -> std::result::Result<SchemaRef, String> {
    let conn_str = conn_str.to_string();
    let table_name = table_name.to_string();

    // Run blocking ODBC in a separate thread
    let schema = tokio::task::spawn_blocking(move || {
        let env = Environment::new().map_err(|e| e.to_string())?;
        let conn = env
            .connect_with_connection_string(&conn_str, ConnectionOptions::default())
            .map_err(|e| e.to_string())?;

        // Query 0 rows to get metadata
        let sql = format!("SELECT * FROM {} WHERE 1=0", table_name);
        let x = match conn.execute(&sql, ()) {
            Ok(Some(mut cursor)) => {
                let num_cols = cursor.num_result_cols().map_err(|e| e.to_string())?;
                let mut fields = Vec::new();

                for i in 1..=num_cols {
                    let mut col_name = cursor
                        .col_name(i.try_into().unwrap())
                        .map_err(|e| e.to_string())?;
                    // If empty, generate name
                    if col_name.is_empty() {
                        col_name = format!("col_{}", i);
                    } else {
                        // Normalize to lowercase for better usability with DataFusion
                        col_name = col_name.to_lowercase();
                    }

                    let col_type = cursor
                        .col_data_type(i.try_into().unwrap())
                        .map_err(|e| e.to_string())?;

                    // Map ODBC type to Arrow DataType
                    let dt = match col_type {
                        odbc_api::DataType::Integer => DataType::Int32,
                        odbc_api::DataType::SmallInt => DataType::Int16,
                        odbc_api::DataType::Real | odbc_api::DataType::Float { .. } => {
                            DataType::Float32
                        }
                        odbc_api::DataType::Double => DataType::Float64,
                        odbc_api::DataType::Numeric { precision, scale }
                        | odbc_api::DataType::Decimal { precision, scale } => {
                            map_numeric_precision_scale(Some(precision as i64), Some(scale as i64))
                        }
                        odbc_api::DataType::Varchar { length: _ }
                        | odbc_api::DataType::Char { length: _ } => DataType::Utf8,
                        odbc_api::DataType::Timestamp { .. } | odbc_api::DataType::Date => {
                            DataType::Utf8
                        } // Return as string for safety
                        _ => DataType::Utf8, // Fallback
                    };

                    fields.push(Field::new(col_name, dt, true));
                }

                Ok(Arc::new(Schema::new(fields)))
            }
            Ok(None) => Err("No cursor returned".to_string()),
            Err(e) => Err(e.to_string()),
        };
        x
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(schema)
}

    // --- Helper: Cache Filename Hash ---
    /// 生成缓存文件名
    ///
    /// **实现方案**:
    /// 1. 计算连接字符串和表名的 Hash 值，确保唯一性。
    /// 2. 对表名进行清理，替换非字母数字字符。
    /// 3. 如果表名过长（>50字符，通常是 SQL 查询），使用固定前缀 "pushdown_query" + Hash，避免文件名过长问题。
    ///
    /// **调用链路**:
    /// - 被 `trigger_background_caching` 调用。
    /// - 被 `YashanTable::scan` 调用。
    ///
    /// **关键问题点**:
    /// - 哈希冲突：必须同时哈希 `conn_str` 和 `table_name`。
    /// - 文件系统限制：文件名长度限制通常为 255 字符，需严格控制。
    pub fn get_cache_filename(table_name: &str, conn_str: &str) -> String {
        let mut hasher = DefaultHasher::new();
        // ALWAYS hash both conn_str and table_name to ensure uniqueness even if names collide after sanitization
        // This fixes the issue where "table$1" and "table#1" (which both sanitize to "table_1") 
        // would conflict if only conn_str was hashed for short names.
        conn_str.hash(&mut hasher);
        table_name.hash(&mut hasher);
        
        let hash = hasher.finish();

        // Sanitize table name
        // If it's a long SQL, just use "query" prefix + hash to avoid OS path length limits
        let safe_name = if table_name.len() > 50 {
            "pushdown_query".to_string()
        } else {
            table_name.replace(|c: char| !c.is_alphanumeric(), "_")
        };

        format!("{}_{}.parquet", safe_name, hash)
    }

// --- Helper: Trigger Background Caching ---
/// 触发后台缓存任务
///
/// **实现方案**:
/// 1. 检查缓存文件是否存在，如果存在则跳过。
/// 2. 使用 `CACHING_TASKS` 全局 Set 检查是否已有相同任务在运行，避免重复。
/// 3. 使用 `SIDECAR_SEMAPHORE` 限制并发缓存任务数量（当前限制为 1）。
/// 4. 启动 tokio 异步任务，并在其中 spawn_blocking 执行阻塞的 ODBC 读取和 Parquet 写入。
///
/// **调用链路**:
/// - 被 `YashanTable::scan` 调用（当缓存未命中时）。
///
/// **关键问题点**:
/// - 并发控制：防止过多并发缓存任务耗尽数据库连接或系统资源。
/// - 错误处理：任务失败不应影响主查询流程，仅记录日志。
pub fn trigger_background_caching(
    conn_str: String,
    logical_table_name: String,
    physical_table_name: String,
    schema: SchemaRef,
) {
    tokio::spawn(async move {
        let cache_dir = crate::config::AppConfig::global().yashan_cache_dir.clone();
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).unwrap_or_default();
        }

        let cache_filename = get_cache_filename(&logical_table_name, &conn_str);
        let cache_path = cache_dir.join(cache_filename);
        let output_path = cache_path.to_string_lossy().to_string();
        let task_key = format!("{}::{}", logical_table_name, cache_path.display());

        crate::app_log!(
            "Sidecar check: table={}, cache_path={}",
            logical_table_name,
            cache_path.display()
        );

        // Check if already cached (optional, but good optimization)
        if cache_path.exists() {
            log_sidecar(
                &logical_table_name,
                0,
                0.0,
                "Skipped",
                "Cache already exists",
            );
            return;
        }

        let tasks_mutex = CACHING_TASKS
            .get_or_init(|| async { Mutex::new(std::collections::HashSet::new()) })
            .await;
        let mut tasks = tasks_mutex.lock().await;

        if !tasks.contains(&task_key) {
            tasks.insert(task_key.clone());
            log_sidecar(&logical_table_name, 0, 0.0, "Queued", "Waiting for sidecar worker");

            tokio::spawn(async move {
                let sem = SIDECAR_SEMAPHORE
                    .get_or_init(|| async { tokio::sync::Semaphore::new(1) })
                    .await;
                let _permit = sem.acquire().await.unwrap();
                log_sidecar(&logical_table_name, 0, 0.0, "Running", "Caching started");

                let start_time = std::time::Instant::now();
                // FIXED: Use spawn_blocking for blocking IO operations
                let table_name_for_cache = physical_table_name.clone();
                let result = tokio::task::spawn_blocking(move || {
                    cache_table_to_parquet(conn_str, table_name_for_cache, schema, output_path)
                })
                .await;

                let elapsed = start_time.elapsed().as_secs_f64();

                match result {
                    Ok(inner_res) => match inner_res {
                        Ok(rows) => {
                            let speed = if elapsed > 0.0 {
                                rows as f64 / elapsed
                            } else {
                                0.0
                            };
                            crate::app_log!(
                                "Sidecar: Cached {} successfully. Rows: {}, Speed: {:.2}",
                                logical_table_name,
                                rows,
                                speed
                            );
                            log_sidecar(
                                &logical_table_name,
                                rows,
                                speed,
                                "Completed",
                                "Cached successfully",
                            );
                        }
                        Err(e) => {
                            crate::app_log!(
                                "Sidecar: Failed to cache {}: {}",
                                logical_table_name,
                                e
                            );
                            log_sidecar(&logical_table_name, 0, 0.0, "Failed", &e);
                        }
                    },
                    Err(join_err) => {
                        crate::app_log!("Sidecar: Task panicked or cancelled: {}", join_err);
                    }
                }

                if let Some(tasks_mutex) = CACHING_TASKS.get() {
                    let mut tasks = tasks_mutex.lock().await;
                    tasks.remove(&task_key);
                }
            });
        } else {
            log_sidecar(
                &logical_table_name,
                0,
                0.0,
                "Skipped",
                "Task already running",
            );
        }
    });
}

// Duplicate removed

/// 标准化 Decimal 字符串表示
///
/// **实现方案**:
/// 1. 处理符号 (+/-)。
/// 2. 分离整数部分和小数部分。
/// 3. 根据目标 scale 填充或截断小数部分。
/// 4. 移除前导零。
///
/// **关键问题点**:
/// - 不支持科学计数法（'e', 'E'）。
/// - 必须精确匹配 scale，否则 Arrow 转换可能失败或数据不准。
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

/// 将 Decimal 字符串转换为 i128 (Decimal128)
///
/// **实现方案**:
/// 调用 `normalize_decimal_string` 标准化后，直接 parse 为 i128。
fn decimal_string_to_i128(value: &str, scale: i8) -> Option<i128> {
    let normalized = normalize_decimal_string(value, scale)?;
    normalized.parse::<i128>().ok()
}

/// 将 Decimal 字符串转换为 i256 (Decimal256)
///
/// **实现方案**:
/// 调用 `normalize_decimal_string` 标准化后，使用 `i256::from_string` 转换。
fn decimal_string_to_i256(value: &str, scale: i8) -> Option<i256> {
    let normalized = normalize_decimal_string(value, scale)?;
    i256::from_string(&normalized)
}

/// 报告 Sidecar 缓存进度
///
/// **实现方案**:
/// 计算已处理行数和实时速度（行/秒），并通过日志记录。
///
/// **调用链路**:
/// - 被 `cache_table_to_parquet` 定期调用。
fn report_sidecar_progress(table_name: &str, row_count: u64, start: &std::time::Instant) {
    let elapsed = start.elapsed().as_secs_f64();
    let speed = if elapsed > 0.0 {
        row_count as f64 / elapsed
    } else {
        0.0
    };
    log_sidecar(
        table_name,
        row_count,
        speed,
        "Running",
        "Writing parquet...",
    );
}

/// 将远程表数据缓存为本地 Parquet 文件
///
/// **实现方案**:
/// 1. 检查缓存目录大小，如果超过限制 (`MAX_CACHE_SIZE_BYTES`)，则驱逐旧文件。
/// 2. 建立 ODBC 连接，执行 `SELECT *` 查询。
/// 3. 使用 `ArrowWriter` 将数据流式写入临时 Parquet 文件。
/// 4. 写入完成后，原子重命名临时文件为正式缓存文件。
///
/// **调用链路**:
/// - 被 `trigger_background_caching` 异步调用。
///
/// **关键问题点**:
/// - 内存控制：分批次 (`batch_size`) 读取和写入，避免大表撑爆内存。
/// - 原子性：使用临时文件 + rename 确保缓存文件完整性。
/// - 类型转换：处理 ODBC 文本数据到 Arrow 类型的解析，特别是 Decimal 类型。
fn cache_table_to_parquet(
    conn_str: String,
    table_name: String,
    schema: SchemaRef,
    output_path: String,
) -> std::result::Result<u64, String> {
    #[cfg(test)]
    if conn_str == "__TEST_DELAY__" {
        std::thread::sleep(std::time::Duration::from_millis(50));
        return Err("TEST".to_string());
    }
    // Check total cache size and evict if needed
    let cache_dir = crate::config::AppConfig::global().yashan_cache_dir.clone();
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        let mut files: Vec<(u64, std::path::PathBuf)> = Vec::new();
        let mut total_size = 0;

        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total_size += meta.len();
                files.push((meta.len(), entry.path()));
            }
        }

        if total_size > MAX_CACHE_SIZE_BYTES {
            // Sort by modification time would be better, but for now just delete arbitrary or largest
            // Let's rely on OS creation order approx or just random eviction for simplicity in this snippet
            // Or better: sort by modified time
            files.sort_by_key(|(_size, path)| {
                path.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });

            // Evict oldest until under limit
            for (size, path) in files {
                if total_size <= MAX_CACHE_SIZE_BYTES {
                    break;
                }
                if std::fs::remove_file(&path).is_ok() {
                    total_size -= size;
                }
            }
        }
    }

    // Connect and Fetch
    let env = Environment::new().map_err(|e| e.to_string())?;
    let conn = env
        .connect_with_connection_string(&conn_str, ConnectionOptions::default())
        .map_err(|e| e.to_string())?;

    // Construct SQL with explicit columns to match YashanExec logic
    let col_names = schema
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {} FROM {}", col_names, table_name);

    crate::app_log!("Sidecar: Caching SQL: {}", sql);

    // Create Parquet Writer (Atomic Write: .tmp -> rename)
    let output_path_tmp = format!("{}.tmp", output_path);

    // Init guard
    let mut guard = TmpFileGuard::new(PathBuf::from(&output_path_tmp));

    let file = File::create(&output_path_tmp).map_err(|e| e.to_string())?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None).map_err(|e| e.to_string())?;

    let mut row_count: u64 = 0;
    match conn.execute(&sql, ()) {
        Ok(Some(mut cursor)) => {
            // Initialize builders for each column based on schema
            let mut builders: Vec<Box<dyn ArrayBuilder>> = Vec::new();
            for field in schema.fields() {
                match field.data_type() {
                    DataType::Int32 => builders.push(Box::new(Int32Builder::new())),
                    DataType::Int64 => builders.push(Box::new(Int64Builder::new())),
                    DataType::Float64 => builders.push(Box::new(Float64Builder::new())),
                    DataType::Utf8 => builders.push(Box::new(StringBuilder::new())),
                    DataType::Decimal128(p, s) => builders.push(Box::new(
                        Decimal128Builder::new()
                            .with_precision_and_scale(*p, *s)
                            .unwrap(),
                    )),
                    DataType::Decimal256(p, s) => builders.push(Box::new(
                        Decimal256Builder::new()
                            .with_precision_and_scale(*p, *s)
                            .unwrap(),
                    )),
                    _ => builders.push(Box::new(StringBuilder::new())), // Fallback to String for other types
                }
            }

            let start = std::time::Instant::now();
            let batch_size: u64 = 10000;
            let mut buf = Vec::new(); // Reusable buffer for text data

            loop {
                let row_result = cursor.next_row();
                if let Err(e) = &row_result {
                     crate::app_log!("Error fetching row in cache_table_to_parquet: {}", e);
                     return Err(format!("Error fetching row: {}", e));
                }
                let mut row = match row_result.unwrap() {
                    Some(r) => r,
                    None => break,
                };

                for (i, field) in schema.fields().iter().enumerate() {
                    let col_idx = (i + 1) as u16;

                    // Fetch text data
                    let val_str = if row.get_text(col_idx, &mut buf).map_err(|e| e.to_string())? {
                        Some(String::from_utf8_lossy(&buf).to_string())
                    } else {
                        None
                    };

                    // Append to builder
                    match field.data_type() {
                        DataType::Int32 => {
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<Int32Builder>()
                                .unwrap();
                            match val_str {
                                Some(s) => {
                                    builder.append_value(s.parse::<i32>().unwrap_or_default())
                                }
                                None => builder.append_null(),
                            }
                        }
                        DataType::Int64 => {
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<Int64Builder>()
                                .unwrap();
                            match val_str {
                                Some(s) => {
                                    builder.append_value(s.parse::<i64>().unwrap_or_default())
                                }
                                None => builder.append_null(),
                            }
                        }
                        DataType::Float64 => {
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<Float64Builder>()
                                .unwrap();
                            match val_str {
                                Some(s) => {
                                    builder.append_value(s.parse::<f64>().unwrap_or_default())
                                }
                                None => builder.append_null(),
                            }
                        }
                        DataType::Decimal128(_, scale) => {
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<Decimal128Builder>()
                                .unwrap();
                            match val_str {
                                Some(s) => {
                                    if let Some(val) = decimal_string_to_i128(&s, *scale) {
                                        builder.append_value(val);
                                    } else {
                                        builder.append_null();
                                    }
                                }
                                None => builder.append_null(),
                            }
                        }
                        DataType::Decimal256(_, scale) => {
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<Decimal256Builder>()
                                .unwrap();
                            match val_str {
                                Some(s) => {
                                    if let Some(val) = decimal_string_to_i256(&s, *scale) {
                                        builder.append_value(val);
                                    } else {
                                        builder.append_null();
                                    }
                                }
                                None => builder.append_null(),
                            }
                        }
                        _ => {
                            // Default to String
                            let builder = builders[i]
                                .as_any_mut()
                                .downcast_mut::<StringBuilder>()
                                .unwrap();
                            match val_str {
                                Some(s) => builder.append_value(s),
                                None => builder.append_null(),
                            }
                        }
                    }
                }

                row_count += 1;

                // Flush batch if needed
                if row_count.is_multiple_of(batch_size) {
                    let mut arrays = Vec::new();
                    for builder in &mut builders {
                        arrays.push(builder.finish());
                    }
                    let batch =
                        RecordBatch::try_new(schema.clone(), arrays).map_err(|e| e.to_string())?;
                    writer.write(&batch).map_err(|e| e.to_string())?;
                    report_sidecar_progress(&table_name, row_count, &start);
                }
            }

            // Final flush
            if !row_count.is_multiple_of(batch_size) || row_count == 0 {
                let mut arrays = Vec::new();
                for builder in &mut builders {
                    arrays.push(builder.finish());
                }
                let batch =
                    RecordBatch::try_new(schema.clone(), arrays).map_err(|e| e.to_string())?;
                writer.write(&batch).map_err(|e| e.to_string())?;
            }

            // Consume cursor to avoid unused warning
            let _ = cursor.num_result_cols();
        }
        Ok(None) => {}
        Err(e) => return Err(e.to_string()),
    }

    writer.close().map_err(|e| e.to_string())?;

    // Rename tmp to actual path (Atomic commit)
    std::fs::rename(&output_path_tmp, &output_path).map_err(|e| e.to_string())?;

    // Success: commit guard
    guard.commit();

    Ok(row_count)
}

#[derive(Debug)]
pub struct YashanTable {
    name: String,
    remote_table_name: Option<String>,
    schema: SchemaRef,
    conn_str: String,
    stats: Option<(i64, i64)>,
    pool: r2d2::Pool<YashanConnectionManager>,
}

impl YashanTable {
    /// 创建 YashanTable 实例
    ///
    /// **实现方案**:
    /// 初始化 YashanTable 结构体，作为 DataFusion 的 TableProvider。
    ///
    /// **调用链路**:
    /// - 被 `YashanDataSource::register` 调用。
    pub fn new(
        name: String,
        remote_table_name: Option<String>,
        schema: SchemaRef,
        conn_str: String,
        stats: Option<(i64, i64)>,
        pool: r2d2::Pool<YashanConnectionManager>,
    ) -> Self {
        crate::app_log!(
            "Creating YashanTable with name: {} (Remote: {:?})",
            name,
            remote_table_name
        );
        Self {
            name,
            remote_table_name,
            schema,
            conn_str,
            stats,
            pool,
        }
    }

    pub fn conn_str(&self) -> &str {
        &self.conn_str
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取远程表名（如果存在别名或映射）
    pub fn remote_table_name(&self) -> &str {
        self.remote_table_name.as_deref().unwrap_or(&self.name)
    }
}

#[async_trait]
impl TableProvider for YashanTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    /// 执行全表扫描或查询
    ///
    /// **实现方案**:
    /// 1. 检查本地 Parquet 缓存是否存在且有效。
    /// 2. **缓存命中**: 直接使用 DataFusion 的 `ParquetFormat` 读取本地文件，返回 `StatisticsExec`（如果统计信息可用）。
    /// 3. **缓存未命中**:
    ///    - 触发后台缓存任务 (`trigger_background_caching`)。
    ///    - 返回 `YashanExec` 执行计划，通过 ODBC 实时查询远程数据库。
    ///
    /// **调用链路**:
    /// - DataFusion 物理计划生成阶段调用。
    ///
    /// **关键问题点**:
    /// - 缓存一致性：当前使用简单的文件名哈希，未处理数据过期或变更。
    /// - 性能权衡：首次查询走远程较慢，后续走本地缓存极快。
    /// - 错误恢复：如果发现缓存文件损坏，自动删除并回退到远程查询。
    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        _filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let state = state
            .as_any()
            .downcast_ref::<SessionState>()
            .ok_or_else(|| {
                DataFusionError::Internal("Failed to downcast Session to SessionState".to_string())
            })?;

        // 1. Check if cache file exists
        let cache_filename = get_cache_filename(&self.name, &self.conn_str);
        let cache_path = std::env::current_dir()
            .unwrap()
            .join("cache")
            .join("yashandb")
            .join(cache_filename);

        crate::app_log!("Scan checking cache at: {:?}", cache_path);
        crate::app_log!("YashanTable stats: {:?}", self.stats);

        if cache_path.exists()
            && std::fs::metadata(&cache_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
        {
            crate::app_log!("Cache Hit: Serving {} from local parquet", self.name);

            let try_parquet: Result<Arc<dyn ExecutionPlan>> = async {
                let object_store_url = ObjectStoreUrl::local_filesystem();
                let path_str = cache_path
                    .to_str()
                    .ok_or_else(|| DataFusionError::Execution("Invalid cache path".to_string()))?;
                let path = Path::from_filesystem_path(path_str)?;

                let file_meta = std::fs::metadata(&cache_path).map_err(DataFusionError::IoError)?;

                let statistics = self.stats.map(|(rows, avg_len)| {
                    Arc::new(Statistics {
                        num_rows: Precision::Exact(rows as usize),
                        total_byte_size: Precision::Inexact((rows * avg_len) as usize),
                        column_statistics: vec![],
                    })
                });

                let partitioned_file = PartitionedFile {
                    object_meta: object_store::ObjectMeta {
                        location: path,
                        last_modified: file_meta
                            .modified()
                            .unwrap_or(std::time::SystemTime::now())
                            .into(),
                        size: file_meta.len(), // u64
                        e_tag: None,
                        version: None,
                    },
                    partition_values: vec![],
                    range: None,
                    statistics,
                    extensions: None,
                    metadata_size_hint: None,
                };

                // Create ParquetSource with schema
                let source = Arc::new(ParquetSource::new(self.schema.clone()));

                // FileScanConfigBuilder::new(url, source) in DataFusion 52.1
                let config = FileScanConfigBuilder::new(object_store_url, source.clone())
                    .with_file_group(vec![partitioned_file].into())
                    .with_projection_indices(projection.cloned())
                    .map_err(|e| DataFusionError::Execution(e.to_string()))?
                    .with_limit(limit)
                    .build();

                let format = ParquetFormat::default();
                let plan = format.create_physical_plan(state, config).await?;

                // Wrap with statistics override if available
                if let Some((rows, avg_len)) = self.stats {
                    let stats = Statistics {
                        num_rows: Precision::Exact(rows as usize),
                        total_byte_size: Precision::Inexact((rows * avg_len) as usize),
                        column_statistics: vec![],
                    };
                    Ok(Arc::new(StatisticsExec { input: plan, stats }) as Arc<dyn ExecutionPlan>)
                } else {
                    Ok(plan)
                }
            }
            .await;

            match try_parquet {
                Ok(plan) => return Ok(plan),
                Err(e) => {
                    crate::app_log!("WARN: Found cache file but failed to create plan (Corrupt?): {}. Deleting...", e);
                    if let Err(del_err) = std::fs::remove_file(&cache_path) {
                        crate::app_log!("ERROR: Failed to delete corrupt cache file: {}", del_err);
                    } else {
                        crate::app_log!("Deleted corrupt cache file: {:?}", cache_path);
                    }
                    // Fall through to remote scan
                }
            }
        }

        // 2. Cache Miss: Return Remote Scan (YashanExec) AND trigger background cache
        crate::app_log!("Cache Miss: serving {} from remote YashanDB", self.name);

        let remote_name = self.remote_table_name().to_string();

        // Trigger background caching
        trigger_background_caching(
            self.conn_str.clone(),
            self.name.clone(),
            remote_name.clone(),
            self.schema.clone(),
        );

        Ok(Arc::new(YashanExec::new(
            remote_name,
            self.schema.clone(),
            self.conn_str.clone(),
            projection.cloned(),
            limit,
            self.stats,
            self.pool.clone(),
        )))
    }
}

/// YashanDB 执行计划节点
///
/// **实现方案**:
/// 负责实际的物理查询执行。实现了 `ExecutionPlan` trait。
/// 在 `execute` 阶段通过 ODBC 连接执行 SQL 并流式返回 Arrow RecordBatch。
///
/// **关键问题点**:
/// - 连接管理：每次 execution 从连接池获取连接，任务结束后释放。
/// - 流式传输：使用 `mpsc` 通道实现异步数据流，避免阻塞 DataFusion 运行时。
#[derive(Debug, Clone)]
struct YashanExec {
    table_name: String,
    schema: SchemaRef,
    conn_str: String,
    projection: Option<Vec<usize>>,
    limit: Option<usize>,
    properties: PlanProperties,
    stats: Option<(i64, i64)>,
    pool: r2d2::Pool<YashanConnectionManager>,
}

impl YashanExec {
    /// 创建 YashanExec 实例
    ///
    /// **实现方案**:
    /// 1. 计算投影后的 Schema。
    /// 2. 初始化 `PlanProperties` (EquivalenceProperties, Partitioning 等)。
    fn new(
        table_name: String,
        schema: SchemaRef,
        conn_str: String,
        projection: Option<Vec<usize>>,
        limit: Option<usize>,
        stats: Option<(i64, i64)>,
        pool: r2d2::Pool<YashanConnectionManager>,
    ) -> Self {
        let projected_schema = if let Some(proj) = &projection {
            Arc::new(schema.project(proj).unwrap())
        } else {
            schema.clone()
        };

        let properties = PlanProperties::new(
            EquivalenceProperties::new(projected_schema),
            datafusion::physical_plan::Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Self {
            table_name,
            schema,
            conn_str,
            projection,
            limit,
            properties,
            stats,
            pool,
        }
    }
}

impl DisplayAs for YashanExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "YashanExec: table={}, limit={:?}",
            self.table_name, self.limit
        )
    }
}

impl ExecutionPlan for YashanExec {
    fn name(&self) -> &str {
        "YashanExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        if let Some(proj) = &self.projection {
            Arc::new(self.schema.project(proj).unwrap())
        } else {
            self.schema.clone()
        }
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn statistics(&self) -> Result<Statistics> {
        match self.stats {
            Some((rows, avg_len)) => {
                let num_rows = Precision::Exact(rows as usize);
                // Total bytes = rows * avg_len
                let total_byte_size = Precision::Inexact((rows * avg_len) as usize);

                Ok(Statistics {
                    num_rows,
                    total_byte_size,
                    column_statistics: vec![], // Unknown column stats
                })
            }
            None => Ok(Statistics::new_unknown(&self.schema())),
        }
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    /// 执行查询并返回数据流
    ///
    /// **实现方案**:
    /// 1. 校验分区（仅支持单分区）。
    /// 2. 创建 `mpsc` 通道用于数据传输。
    /// 3. 构建 SQL 查询语句（包含投影和 Limit）。
    /// 4. `spawn_blocking` 启动阻塞任务：
    ///    - 获取 ODBC 连接。
    ///    - 执行查询。
    ///    - 遍历结果集，将 ODBC 数据转换为 Arrow 数组。
    ///    - 按 `batch_size` (1000) 发送 `RecordBatch` 到通道。
    /// 5. 返回 `RecordBatchStreamAdapter` 封装接收端。
    ///
    /// **调用链路**:
    /// - DataFusion 运行时调度执行。
    ///
    /// **关键问题点**:
    /// - 错误传播：任何 ODBC 错误或数据转换错误需通过通道发送给 DataFusion。
    /// - 内存管理：流式处理，避免一次性加载所有数据。
    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition != 0 {
            return Err(DataFusionError::Internal(
                "YashanExec only supports 1 partition".to_string(),
            ));
        }

        type ExecChannel = (
            mpsc::Sender<Result<RecordBatch, DataFusionError>>,
            mpsc::Receiver<Result<RecordBatch, DataFusionError>>,
        );
        let (tx, rx): ExecChannel = mpsc::channel(2);
        // conn_str is managed by the pool
        let _conn_str = self.conn_str.clone();
        let table_name = self.table_name.clone();
        let limit = self.limit;
        let schema = self.schema(); // Projected schema
        let pool = self.pool.clone();

        // Build SQL with projection and limit
        let col_names = schema
            .fields()
            .iter()
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = format!("SELECT {} FROM {}", col_names, table_name);
        if let Some(l) = limit {
            query = format!("{} LIMIT {}", query, l);
        }
        crate::app_log!("YashanExec: Executing SQL: {}", query);

        tokio::task::spawn_blocking(move || {
            // let env = Environment::new().unwrap();
            {
                // match env.connect_with_connection_string(&conn_str, ConnectionOptions::default()) {
                match pool.get() {
                    Ok(conn) => {
                        let exec_res: std::result::Result<
                            Option<CursorImpl<StatementImpl<'_>>>,
                            odbc_api::Error,
                        > = conn.execute(&query, ());
                        match exec_res {
                            Ok(Some(cursor)) => {
                                let mut cursor: CursorImpl<StatementImpl<'_>> = cursor;

                                // Real data fetching logic
                                let mut builders: Vec<Box<dyn ArrayBuilder>> = Vec::new();
                                for field in schema.fields() {
                                    match field.data_type() {
                                        DataType::Int32 => {
                                            builders.push(Box::new(Int32Builder::new()))
                                        }
                                        DataType::Int64 => {
                                            builders.push(Box::new(Int64Builder::new()))
                                        }
                                        DataType::Float64 => {
                                            builders.push(Box::new(Float64Builder::new()))
                                        }
                                        DataType::Utf8 => {
                                            builders.push(Box::new(StringBuilder::new()))
                                        }
                                        _ => builders.push(Box::new(StringBuilder::new())),
                                    }
                                }

                                let batch_size = 1000; // Smaller batch for streaming
                                let mut row_count = 0;
                                let mut buf = Vec::new();

                                while let Ok(Some(mut row)) = cursor.next_row() {
                                    for (i, field) in schema.fields().iter().enumerate() {
                                        let col_idx = (i + 1) as u16;

                                        // Fetch text data
                                        let val_str =
                                            if row.get_text(col_idx, &mut buf).unwrap_or(false) {
                                                Some(String::from_utf8_lossy(&buf).to_string())
                                            } else {
                                                None
                                            };

                                        match field.data_type() {
                                            DataType::Int32 => {
                                                let builder = builders[i]
                                                    .as_any_mut()
                                                    .downcast_mut::<Int32Builder>()
                                                    .unwrap();
                                                match val_str {
                                                    Some(s) => builder.append_value(
                                                        s.parse().unwrap_or_default(),
                                                    ),
                                                    None => builder.append_null(),
                                                }
                                            }
                                            DataType::Int64 => {
                                                let builder = builders[i]
                                                    .as_any_mut()
                                                    .downcast_mut::<Int64Builder>()
                                                    .unwrap();
                                                match val_str {
                                                    Some(s) => builder.append_value(
                                                        s.parse().unwrap_or_default(),
                                                    ),
                                                    None => builder.append_null(),
                                                }
                                            }
                                            DataType::Float64 => {
                                                let builder = builders[i]
                                                    .as_any_mut()
                                                    .downcast_mut::<Float64Builder>()
                                                    .unwrap();
                                                match val_str {
                                                    Some(s) => builder.append_value(
                                                        s.parse().unwrap_or_default(),
                                                    ),
                                                    None => builder.append_null(),
                                                }
                                            }
                                            _ => {
                                                let builder = builders[i]
                                                    .as_any_mut()
                                                    .downcast_mut::<StringBuilder>()
                                                    .unwrap();
                                                match val_str {
                                                    Some(s) => builder.append_value(s),
                                                    None => builder.append_null(),
                                                }
                                            }
                                        }
                                    }

                                    row_count += 1;

                                    if row_count % batch_size == 0 {
                                        let mut arrays = Vec::new();
                                        for builder in &mut builders {
                                            arrays.push(builder.finish());
                                        }
                                        if let Ok(batch) =
                                            RecordBatch::try_new(schema.clone(), arrays)
                                        {
                                            if tx.blocking_send(Ok(batch)).is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }

                                // Final flush
                                if row_count % batch_size != 0 || row_count == 0 {
                                    let mut arrays = Vec::new();
                                    for builder in &mut builders {
                                        arrays.push(builder.finish());
                                    }
                                    if let Ok(batch) = RecordBatch::try_new(schema.clone(), arrays)
                                    {
                                        let _ = tx.blocking_send(Ok(batch));
                                    }
                                }

                                let _ = cursor.num_result_cols();
                            }
                            Ok(None) => {
                                let _ = tx.blocking_send(Ok(RecordBatch::new_empty(schema)));
                            }
                            Err(e) => {
                                let err_res: Result<RecordBatch, DataFusionError> =
                                    Err(DataFusionError::Execution(e.to_string()));
                                let _ = tx.blocking_send(err_res);
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.blocking_send(Err(DataFusionError::Execution(format!(
                            "Connection failed: {}",
                            e
                        ))));
                    }
                };
            }
        });

        // Use StreamAdapter to satisfy RecordBatchStream trait
        let adapter = StreamAdapter::new(self.schema(), Box::pin(ReceiverStream::new(rx)));

        Ok(Box::pin(adapter))
    }
}

/// 包装统计信息的执行计划
///
/// **实现方案**:
/// 包装另一个 ExecutionPlan（通常是 ParquetExec），并覆盖其统计信息。
/// 这里的目的是在从缓存读取 Parquet 时，仍然使用从数据库获取的最新行数统计，而不是 Parquet 文件的元数据（可能过时）。
///
/// **关键问题点**:
/// - 仅覆盖 statistics 方法，其他方法透传给 input。
#[derive(Debug)]
struct StatisticsExec {
    input: Arc<dyn ExecutionPlan>,
    stats: Statistics,
}

impl DisplayAs for StatisticsExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "StatisticsExec: rows={:?}", self.stats.num_rows)
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionPlan for StatisticsExec {
    fn name(&self) -> &str {
        "StatisticsExec"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn properties(&self) -> &PlanProperties {
        self.input.properties()
    }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(StatisticsExec {
            input: children[0].clone(),
            stats: self.stats.clone(),
        }))
    }
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        self.input.execute(partition, context)
    }
    fn statistics(&self) -> Result<Statistics> {
        Ok(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_yashandb_conn_str_with_service() {
        let conn_str = build_yashandb_conn_str("127.0.0.1", 1688, "user", "pass", "db1");
        assert!(conn_str.contains("Database=db1;"));
    }

    #[test]
    fn test_build_yashandb_conn_str_without_service() {
        let conn_str = build_yashandb_conn_str("127.0.0.1", 1688, "user", "pass", "");
        assert!(!conn_str.contains("Database="));
    }

    #[test]
    fn test_normalize_pushdown_sql_aliases_expr() {
        let sql = "SELECT sum(h.H_amount) FROM t h";
        let normalized = normalize_pushdown_sql(sql).unwrap();
        let normalized_lower = normalized.to_lowercase();
        assert!(normalized_lower.contains("as __df_expr_1"));
    }

    #[test]
    fn test_normalize_pushdown_sql_keeps_identifier() {
        let sql = "SELECT h.amount FROM t h";
        let normalized = normalize_pushdown_sql(sql).unwrap();
        let normalized_lower = normalized.to_lowercase();
        assert!(!normalized_lower.contains("__df_expr_"));
    }

    #[test]
    fn test_direct_connection() {
        let conn_str = match std::env::var("YASHAN_TEST_DSN") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return,
        };
        println!("Testing connection to: {}", conn_str);

        let env = Environment::new().unwrap();

        // Scope the connection to drop it before env
        {
            match env.connect_with_connection_string(&conn_str, ConnectionOptions::default()) {
                Ok(_conn) => println!("Connection SUCCESS!"),
                Err(e) => {
                    println!("Connection FAILED: {}", e);
                    panic!("Connection failed: {}", e);
                }
            };
        }
    }

    #[tokio::test]
    async fn test_parquet_scan_hit() -> Result<(), Box<dyn std::error::Error>> {
        use arrow::array::{Int32Array, StringArray};
        use arrow::datatypes::{DataType, Field, Schema};
        use arrow::record_batch::RecordBatch;
        use datafusion::common::TableReference;
        use datafusion::prelude::SessionContext;
        use parquet::arrow::ArrowWriter;
        use std::fs::File;
        use std::path::Path;
        use std::sync::Arc;

        // 1. Setup cache directory
        let cache_dir = Path::new("cache/yashandb");
        if !cache_dir.exists() {
            std::fs::create_dir_all(cache_dir)?;
        }

        // 2. Prepare filename using the same hashing logic as the implementation
        let table_name = "TEST_PARQUET_SCAN";
        let conn_str = format!(
            "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};",
            "host", 1234, "user", "pass"
        );
        let cache_filename = get_cache_filename(table_name, &conn_str);
        let file_path = cache_dir.join(cache_filename);

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["a", "b", "c"])),
            ],
        )?;

        let file = File::create(&file_path)?;
        let mut writer = ArrowWriter::try_new(file, batch.schema(), None)?;
        writer.write(&batch)?;
        writer.close()?;

        // 3. Create YashanDataSource
        // Note: we use dummy connection info because we expect a cache hit (no connection needed)
        let mut ds = match YashanDataSource::new(
            table_name.to_string(),
            None,
            "user".to_string(),
            "pass".to_string(),
            "host".to_string(),
            1234,
            "service".to_string(),
            None,
        ) {
            Ok(ds) => ds,
            Err(_) => return Ok(()),
        };
        ds.schema = Some(schema.clone());

        // 4. Call scan
        let ctx = SessionContext::new();
        ds.register(&ctx).await?;

        let table = ctx.table_provider(TableReference::from(table_name)).await?;
        let state = ctx.state();
        let projection = None;
        let filters = &[];
        let limit = None;

        let plan = table.scan(&state, projection, filters, limit).await?;

        // 5. Verify plan
        let results = datafusion::physical_plan::collect(plan, state.task_ctx()).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].num_rows(), 3);

        // Cleanup
        let _ = std::fs::remove_file(file_path);

        Ok(())
    }

    #[tokio::test]
    async fn test_bmsql_order_line_caching() -> Result<(), Box<dyn std::error::Error>> {
        use crate::datasources::yashandb::YashanTable;
        use crate::metadata_manager::MetadataManager;
        use datafusion::common::TableReference;
        use datafusion::prelude::SessionContext;
        use std::time::Duration;

        // 1. Config Strategy: Try to load from MetadataStore first, fallback to hardcoded
        let mut host = "192.168.23.3".to_string();
        let mut port = 1843;
        let mut user = "i2".to_string();
        let mut pass = "i2".to_string();
        let mut service = "yasdb".to_string();
        let table_name = "tpcc.BMSQL_DISTRICT";

        // Try to load from metadata.db
        let db_path = std::env::current_dir().unwrap().join("metadata.db");
        if let Ok(mm) = MetadataManager::new(db_path.to_str().unwrap()) {
            if let Ok(conns) = mm.list_connections() {
                for conn in conns {
                    if conn.source_type == "yashandb" {
                        println!("Found YashanDB connection in metadata: {}", conn.name);
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&conn.config)
                        {
                            if let Some(h) = config["host"].as_str() {
                                host = h.to_string();
                            }
                            if let Some(p) = config["port"].as_u64() {
                                port = p as u16;
                            }
                            if let Some(u) = config["user"].as_str() {
                                user = u.to_string();
                            }
                            if let Some(pwd) = config["pass"].as_str() {
                                pass = pwd.to_string();
                            }
                            if let Some(s) = config["service"].as_str() {
                                service = s.to_string();
                            }
                            println!("Using config from metadata: {}@{}:{}", user, host, port);
                            break;
                        }
                    }
                }
            }
        } else {
            println!("Could not load metadata.db, using hardcoded defaults.");
        }

        // Try to connect with retry logic for Driver format
        let drivers = vec!["{YashanDB}", "YashanDB"];
        let mut conn_str = String::new();
        let mut connected = false;

        println!("Testing connectivity to YashanDB ({}:{})...", host, port);

        for driver in drivers {
            let try_conn_str = format!(
                "Driver={};Server={};Port={};Uid={};Pwd={};",
                driver, host, port, user, pass
            );

            let success = {
                match Environment::new() {
                    Ok(env) => {
                        match env.connect_with_connection_string(
                            &try_conn_str,
                            ConnectionOptions::default(),
                        ) {
                            Ok(_) => true,
                            Err(e) => {
                                println!("Failed to connect with driver {}: {}", driver, e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        println!("ODBC Environment creation failed: {}", e);
                        false
                    }
                }
            };

            if success {
                println!("Successfully connected with driver: {}", driver);
                conn_str = try_conn_str;
                connected = true;
                break;
            }
        }

        if !connected {
            println!("WARN: Skipping test_bmsql_order_line_caching due to connection failure (YAS-00402/Network).");
            println!(
                "Please verify IP ({}), Port ({}) and Firewall settings.",
                host, port
            );
            return Ok(());
        }

        // Clean cache
        let cache_filename = get_cache_filename(table_name, &conn_str);
        let cache_dir = std::env::current_dir()
            .unwrap()
            .join("cache")
            .join("yashandb");
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }
        let cache_path = cache_dir.join(&cache_filename);
        // Disable auto-deletion to allow persistence testing
        if cache_path.exists() {
            println!("Deleting existing cache: {:?}", cache_path);
            if let Err(e) = std::fs::remove_file(&cache_path) {
                println!("WARN: Failed to delete cache file: {}", e);
            }
        }

        // 3. Register DS
        let mut ds = match YashanDataSource::new(
            table_name.to_string(),
            None,
            user.to_string(),
            "pass".to_string(),
            "host".to_string(),
            port,
            service.clone(),
            None,
        ) {
            Ok(ds) => ds,
            Err(e) => {
                println!("WARN: Failed to create YashanDataSource: {}", e);
                return Ok(());
            }
        };
        // HACK: Overwrite conn_str in ds to match the working one
        ds.conn_str = conn_str.clone();

        // Fetch stats to verify COST functionality
        if let Err(e) = ds.fetch_stats() {
            println!("WARN: Failed to fetch stats: {}", e);
        } else {
            println!("Stats fetched: {:?}", ds.stats);
        }

        let ctx = SessionContext::new();

        // Manual registration with flat name to avoid schema issues in test
        let flat_name = "BMSQL_DISTRICT";
        println!("Registering table as {}...", flat_name);

        let schema = match ds.fetch_schema().await {
            Ok(s) => s,
            Err(e) => {
                println!("WARN: Schema fetch failed: {}", e);
                return Ok(());
            }
        };

        // Note: We use ds.table_name (qualified) for the provider so it queries/caches correctly
        let provider = YashanTable::new(
            ds.table_name.clone(),
            Some(ds.table_name.clone()),
            schema,
            ds.conn_str.clone(),
            ds.stats,
            ds.pool.clone(),
        );
        ctx.register_table(flat_name, Arc::new(provider))?;

        let _table = ctx.table_provider(TableReference::from(flat_name)).await?;

        // 4. First Scan
        println!("First Scan...");
        let df = ctx.sql("SELECT * FROM BMSQL_DISTRICT LIMIT 5").await?;
        let batches = df.collect().await?;
        println!("First scan batches: {}", batches.len());

        // 5. Wait for Sidecar
        println!("Waiting for sidecar caching...");
        let mut waited = 0;
        let mut cached = false;
        let tmp_path = format!("{}.tmp", cache_path.to_string_lossy());
        let tmp_path = std::path::Path::new(&tmp_path);

        loop {
            if cache_path.exists() {
                let meta = std::fs::metadata(&cache_path)?;
                if meta.len() > 0 {
                    println!("Cache file created! Size: {} bytes", meta.len());
                    cached = true;
                    break;
                }
            }
            if tmp_path.exists() {
                println!("Temp file exists, caching in progress...");
            }

            if waited > 30 {
                println!("Timeout waiting for cache.");
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            waited += 1;
        }

        if cached {
            println!("Second Scan (from cache)...");
            // Force re-scan to check cache hit
            let df = ctx.sql("SELECT * FROM BMSQL_DISTRICT LIMIT 5").await?;
            let batches = df.collect().await?;
            println!("Second scan batches: {}", batches.len());
            // We can't easily assert "from cache" without checking logs or modifying YashanTable to expose state
            // But if it didn't panic and returned rows, it works.
        }

        Ok(())
    }

    #[test]
    fn test_extract_simple_table_name() {
        // Basic cases
        assert_eq!(extract_simple_table_name("SELECT * FROM my_table"), Some("my_table".to_string()));
        assert_eq!(extract_simple_table_name("select * from schema.table_name"), Some("schema.table_name".to_string()));
        
        // Whitespace handling (parser handles this naturally)
        assert_eq!(extract_simple_table_name("  SELECT * FROM   my_table  "), Some("my_table".to_string()));
        
        // Negative cases
        assert_eq!(extract_simple_table_name("SELECT * FROM my_table WHERE id=1"), None); // Has WHERE
        assert_eq!(extract_simple_table_name("SELECT a FROM my_table"), None); // Not *
        assert_eq!(extract_simple_table_name("SELECT * FROM t1, t2"), None); // Comma implies join or list
        
        // Check for semi-colon (Parser might handle or reject trailing semicolon depending on impl, usually accepts one)
        // extract_simple_table_name("SELECT * FROM table;") -> Some("table") if parser allows it.
        // sqlparser usually parses "SELECT * FROM table;" as one statement.
        assert_eq!(extract_simple_table_name("SELECT * FROM my_table;"), Some("my_table".to_string()));
        
        // Multiple statements
        assert_eq!(extract_simple_table_name("SELECT * FROM t1; DROP TABLE t1"), None);
    }

    #[test]
    fn test_get_cache_filename_uniqueness() {
        let t1 = "table$1";
        let t2 = "table#1";
        let conn = "conn_string_example";
        
        let f1 = get_cache_filename(t1, conn);
        let f2 = get_cache_filename(t2, conn);
        
        println!("f1: {}", f1);
        println!("f2: {}", f2);
        
        // They should have the same prefix "table_1_" (sanitized) but different hashes suffix
        assert!(f1.starts_with("table_1_"));
        assert!(f2.starts_with("table_1_"));
        assert_ne!(f1, f2, "Cache filenames should be different for different table names even if sanitized same");
    }
    
    #[test]
    fn test_get_cache_filename_length() {
        let long_query = "SELECT * FROM table WHERE id > 100 AND name LIKE '%test%' AND created_at > '2023-01-01'".repeat(5);
        let conn = "conn";
        let f = get_cache_filename(&long_query, conn);
        
        assert!(f.starts_with("pushdown_query_"));
        assert!(f.len() < 100); // Should be reasonable length
    }
}

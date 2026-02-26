#![cfg(feature = "oracle")]

use async_trait::async_trait;
use datafusion::arrow::array::{Float64Builder, Int64Builder, RecordBatch, StringBuilder};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::error::{DataFusionError, Result};
use datafusion::parquet::arrow::{AsyncArrowWriter, ParquetRecordBatchStreamBuilder};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::partitioning::Partitioning;
use datafusion::physical_plan::{
    DisplayAs, ExecutionPlan, PlanProperties, RecordBatchStreamAdapter, SendableRecordBatchStream,
};
use datafusion::prelude::{Expr, SessionContext};
use futures::StreamExt;
use oracle::{Connection, Row};
use std::any::Any;
use std::fmt;
use std::sync::Arc;
use tokio::fs::File as TokioFile;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::sql_dialect::{OracleDialect, SqlDialect};
use crate::cache_manager::{CacheManager, CachePolicy, FlightGuard};
use crate::datasources::DataSource;

#[derive(Debug, Clone, Copy)]
pub enum OracleFetchStrategy {
    Pagination12c,    // OFFSET ... FETCH NEXT ... (Oracle 12c+)
    PaginationLegacy, // ROWNUM based pagination (Oracle 11g and older)
}

#[derive(Clone)]
pub struct OracleDataSource {
    name: String,
    user: String,
    pass: String,
    conn_str: String,
    table_name: String,
    batch_size: usize,
    use_legacy_pagination: bool,
}

impl OracleDataSource {
    pub fn new(
        name: String,
        user: String,
        pass: String,
        conn_str: String,
        table_name: String,
    ) -> Self {
        Self {
            name,
            user,
            pass,
            conn_str,
            table_name,
            batch_size: 8192,
            use_legacy_pagination: false,
        }
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_legacy_pagination(mut self, use_legacy: bool) -> Self {
        self.use_legacy_pagination = use_legacy;
        self
    }

    fn get_schema(&self) -> Result<SchemaRef> {
        let conn = Connection::connect(&self.user, &self.pass, &self.conn_str)
            .map_err(|e| DataFusionError::Execution(format!("Oracle connection failed: {}", e)))?;

        // Query with condition that returns no rows to get metadata
        let sql = format!("SELECT * FROM {} WHERE 1=0", self.table_name);
        let stmt = conn.statement(&sql).build().map_err(|e| {
            DataFusionError::Execution(format!("Failed to prepare statement: {}", e))
        })?;

        let rows = stmt
            .query(&[])
            .map_err(|e| DataFusionError::Execution(format!("Failed to query schema: {}", e)))?;

        let col_info = rows.column_info();
        let mut fields = Vec::new();

        for col in col_info {
            // Simplified type mapping
            let dt = match col.oracle_type().to_string().as_str() {
                "NUMBER" => DataType::Float64, // Oracle NUMBER can be anything, Float64 is safest generic
                "FLOAT" | "BINARY_FLOAT" | "BINARY_DOUBLE" => DataType::Float64,
                "VARCHAR2" | "CHAR" | "NCHAR" | "NVARCHAR2" | "CLOB" => DataType::Utf8,
                "DATE" | "TIMESTAMP" | "TIMESTAMP WITH TIME ZONE" => DataType::Utf8, // Return as string for now to avoid complex parsing
                _ => DataType::Utf8,
            };
            fields.push(Field::new(col.name(), dt, true));
        }

        Ok(Arc::new(Schema::new(fields)))
    }
}

#[async_trait]
impl DataSource for OracleDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        let schema = self.get_schema()?;
        let table = OracleTable::new(
            self.user.clone(),
            self.pass.clone(),
            self.conn_str.clone(),
            self.table_name.clone(),
            schema,
            self.batch_size,
            if self.use_legacy_pagination {
                OracleFetchStrategy::PaginationLegacy
            } else {
                OracleFetchStrategy::Pagination12c
            },
        );
        ctx.register_table(&self.name, Arc::new(table))?;
        Ok(())
    }
}

#[derive(Debug)]
struct OracleTable {
    user: String,
    pass: String,
    conn_str: String,
    table_name: String,
    schema: SchemaRef,
    batch_size: usize,
    fetch_strategy: OracleFetchStrategy,
}

impl OracleTable {
    fn new(
        user: String,
        pass: String,
        conn_str: String,
        table_name: String,
        schema: SchemaRef,
        batch_size: usize,
        fetch_strategy: OracleFetchStrategy,
    ) -> Self {
        Self {
            user,
            pass,
            conn_str,
            table_name,
            schema,
            batch_size,
            fetch_strategy,
        }
    }
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
    ) -> Result<Vec<datafusion::logical_expr::TableProviderFilterPushDown>> {
        let dialect = OracleDialect::new(matches!(
            self.fetch_strategy,
            OracleFetchStrategy::PaginationLegacy
        ));
        let support = filters
            .iter()
            .map(|expr| match dialect.expr_to_sql(expr) {
                Ok(_) => datafusion::logical_expr::TableProviderFilterPushDown::Exact,
                Err(_) => datafusion::logical_expr::TableProviderFilterPushDown::Unsupported,
            })
            .collect();
        Ok(support)
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let projected_schema = if let Some(indices) = projection {
            Arc::new(self.schema.project(indices)?)
        } else {
            self.schema.clone()
        };

        let dialect = OracleDialect::new(matches!(
            self.fetch_strategy,
            OracleFetchStrategy::PaginationLegacy
        ));
        let where_clause = if filters.is_empty() {
            None
        } else {
            let sql_filters: Vec<String> = filters
                .iter()
                .map(|expr| dialect.expr_to_sql(expr))
                .collect::<Result<Vec<_>>>()
                .map_err(|e| {
                    DataFusionError::Execution(format!(
                        "Failed to generate SQL from filters: {}",
                        e
                    ))
                })?;

            if sql_filters.is_empty() {
                None
            } else {
                Some(sql_filters.join(" AND "))
            }
        };

        Ok(Arc::new(OracleExec::new(
            self.user.clone(),
            self.pass.clone(),
            self.conn_str.clone(),
            self.table_name.clone(),
            projected_schema,
            projection.cloned(),
            self.batch_size,
            self.fetch_strategy,
            where_clause,
        )))
    }
}

#[derive(Debug)]
struct OracleExec {
    user: String,
    pass: String,
    conn_str: String,
    table_name: String,
    schema: SchemaRef,
    _projection: Option<Vec<usize>>,
    batch_size: usize,
    properties: PlanProperties,
    fetch_strategy: OracleFetchStrategy,
    where_clause: Option<String>,
}

impl OracleExec {
    fn new(
        user: String,
        pass: String,
        conn_str: String,
        table_name: String,
        schema: SchemaRef,
        projection: Option<Vec<usize>>,
        batch_size: usize,
        fetch_strategy: OracleFetchStrategy,
        where_clause: Option<String>,
    ) -> Self {
        let eq_properties = EquivalenceProperties::new(schema.clone());
        let properties = PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            user,
            pass,
            conn_str,
            table_name,
            schema,
            _projection: projection,
            batch_size,
            properties,
            fetch_strategy,
            where_clause,
        }
    }
}

impl DisplayAs for OracleExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "OracleExec: table={}", self.table_name)
    }
}

impl ExecutionPlan for OracleExec {
    fn name(&self) -> &str {
        "OracleExec"
    }
    fn as_any(&self) -> &dyn Any {
        self
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

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<datafusion::execution::TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let user = self.user.clone();
        let pass = self.pass.clone();
        let conn_str = self.conn_str.clone();
        let table_name = self.table_name.clone();
        let batch_size = self.batch_size;
        let schema = self.schema.clone();
        let _projection = self._projection.clone();
        let fetch_strategy = self.fetch_strategy;
        let where_clause = self.where_clause.clone();

        // 1. Get Source SCN (System Change Number) for Consistency & Cache Key
        // Try MAX(ORA_ROWSCN) first. If it fails or table is empty, use 0.
        // This is a blocking call but fast enough for metadata phase.
        let source_scn = {
            match Connection::connect(&user, &pass, &conn_str) {
                Ok(conn) => {
                    // Note: ORA_ROWSCN is conservative. For row-level dependency it needs table created with ROWDEPENDENCIES.
                    // Otherwise it is block-level SCN.
                    let sql = format!("SELECT max(ORA_ROWSCN) FROM {}", table_name);
                    match conn.query_as::<Option<i64>>(&sql, &[]) {
                        Ok(mut rows) => {
                            if let Some(Ok(Some(scn))) = rows.next() {
                                scn as u64
                            } else {
                                0 // Empty table or null
                            }
                        }
                        Err(_) => 0, // Fallback
                    }
                }
                Err(_) => 0, // Connection error, will be caught later in read task
            }
        };

        // 2. Generate Cache Key
        let cache_key = CacheManager::generate_key(
            &table_name,
            where_clause.as_deref(),
            self._projection.as_ref(),
            source_scn,
        );

        // 3. Volatility Check (Circuit Breaker)
        use crate::cache_manager::CachePolicy;
        let volatility_policy = CacheManager::check_volatility(&table_name, source_scn);
        if volatility_policy == CachePolicy::Bypass {
            println!(
                "[OracleExec] Volatility Bypass: Table '{}' (SCN: {}) is volatile. Skipping cache.",
                table_name, source_scn
            );
        }

        // 4. L2 Cache Check (Memory)
        if volatility_policy == CachePolicy::UseCache {
            if let Some(batches) = CacheManager::get_l2(&cache_key) {
                println!(
                    "[OracleExec] Cache Hit (L2): Reading from Memory for key {}",
                    cache_key
                );
                return Ok(Box::pin(MemoryStream::try_new(batches, schema, None)?));
            }

            // 5. L1 Cache Check (Disk/Parquet)
            if let Some(cache_path) = CacheManager::get_l1_file(&cache_key) {
                println!(
                    "[OracleExec] Cache Hit (L1): Reading from Parquet for key {}",
                    cache_key
                );
                let schema = self.schema.clone();

                // Promotion Logic
                let should_promote = CacheManager::should_promote_to_l2(&cache_key);
                let promote_key = if should_promote {
                    Some(cache_key.clone())
                } else {
                    None
                };

                // Async read from Parquet
                let (tx, rx) = mpsc::channel(2);
                tokio::spawn(async move {
                    match TokioFile::open(&cache_path).await {
                        Ok(file) => {
                            match ParquetRecordBatchStreamBuilder::new(file).await {
                                Ok(builder) => {
                                    let mut stream = builder.build().unwrap();
                                    let mut l2_buffer = if promote_key.is_some() {
                                        Some(Vec::new())
                                    } else {
                                        None
                                    };
                                    let start_time = std::time::Instant::now();

                                    while let Some(batch_result) = stream.next().await {
                                        if let Ok(batch) = batch_result {
                                            // Collect for promotion
                                            if let Some(buf) = &mut l2_buffer {
                                                buf.push(batch.clone());
                                                if buf.len() > 1000 {
                                                    l2_buffer = None;
                                                }
                                            }

                                            if tx.send(Ok(batch)).await.is_err() {
                                                break;
                                            }
                                        }
                                    }

                                    // Execute Promotion
                                    if let Some(key) = promote_key {
                                        if let Some(buf) = l2_buffer {
                                            if !buf.is_empty() {
                                                let cost = start_time.elapsed().as_millis() as u64;
                                                println!(
                                                    "[OracleExec] Promoting L1 -> L2 for key {}",
                                                    key
                                                );
                                                CacheManager::put_l2(key, buf, cost);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[OracleExec] Corrupt cache file: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[OracleExec] Failed to open cache file: {:?}", e);
                        }
                    }
                });
                return Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema,
                    ReceiverStream::new(rx),
                )));
            }
        }

        // 6. Singleflight: Request Coalescing
        let mut flight_guard = None;

        if volatility_policy == CachePolicy::UseCache {
            loop {
                match CacheManager::join_or_start_flight(cache_key.clone()) {
                    crate::cache_manager::FlightResult::IsLeader(guard) => {
                        flight_guard = Some(guard);
                        break;
                    }
                    crate::cache_manager::FlightResult::IsFollower(rx) => {
                        println!(
                            "[OracleExec] Cache Stampede Protection: Waiting for key {}",
                            cache_key
                        );
                        let (tx, stream_rx) = mpsc::channel(10);
                        let key_clone = cache_key.clone();

                        // Clone params for retry
                        let user_retry = user.clone();
                        let pass_retry = pass.clone();
                        let conn_str_retry = conn_str.clone();
                        let table_name_retry = table_name.clone();
                        let schema_retry = schema.clone();
                        let batch_size_retry = batch_size;
                        let fetch_strategy_retry = fetch_strategy;
                        let where_clause_retry = where_clause.clone();
                        let volatility_policy_retry = volatility_policy;

                        tokio::spawn(async move {
                            let mut current_rx = rx;
                            loop {
                                let status = if current_rx.changed().await.is_err() {
                                    crate::cache_manager::FlightStatus::Failed
                                } else {
                                    *current_rx.borrow()
                                };

                                if status == crate::cache_manager::FlightStatus::Completed {
                                    if let Some(batches) = CacheManager::get_l2(&key_clone) {
                                        println!(
                                            "[OracleExec] Singleflight: Reading from L2 for key {}",
                                            key_clone
                                        );
                                        for batch in batches {
                                            if tx.send(Ok(batch)).await.is_err() {
                                                break;
                                            }
                                        }
                                        break;
                                    }
                                    if let Some(path) = CacheManager::get_l1_file(&key_clone) {
                                        println!(
                                            "[OracleExec] Singleflight: Reading from L1 for key {}",
                                            key_clone
                                        );
                                        match TokioFile::open(&path).await {
                                            Ok(file) => {
                                                match ParquetRecordBatchStreamBuilder::new(file)
                                                    .await
                                                {
                                                    Ok(builder) => match builder.build() {
                                                        Ok(mut stream) => {
                                                            while let Some(res) =
                                                                stream.next().await
                                                            {
                                                                let res = res.map_err(|e| {
                                                                    DataFusionError::External(
                                                                        Box::new(e),
                                                                    )
                                                                });
                                                                if tx.send(res).await.is_err() {
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let _ = tx
                                                                .send(Err(
                                                                    DataFusionError::External(
                                                                        Box::new(e),
                                                                    ),
                                                                ))
                                                                .await;
                                                        }
                                                    },
                                                    Err(e) => {
                                                        let _ = tx
                                                            .send(Err(DataFusionError::External(
                                                                Box::new(e),
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let _ =
                                                    tx.send(Err(DataFusionError::IoError(e))).await;
                                            }
                                        }
                                        break;
                                    }
                                    println!("[OracleExec] Cache entry missing after flight completion. Retrying...");
                                } else if status == crate::cache_manager::FlightStatus::InProgress {
                                    continue;
                                }

                                // Failed or Cancelled or Missing -> Retry
                                println!("[OracleExec] Flight failed/cancelled. Retrying as Leader for key {}", key_clone);
                                match CacheManager::join_or_start_flight(key_clone.clone()) {
                                    crate::cache_manager::FlightResult::IsLeader(guard) => {
                                        spawn_oracle_fetch_and_sidecar(
                                            tx,
                                            user_retry,
                                            pass_retry,
                                            conn_str_retry,
                                            table_name_retry,
                                            schema_retry,
                                            batch_size_retry,
                                            fetch_strategy_retry,
                                            where_clause_retry,
                                            key_clone,
                                            volatility_policy_retry,
                                            Some(guard),
                                        );
                                        break;
                                    }
                                    crate::cache_manager::FlightResult::IsFollower(new_rx) => {
                                        current_rx = new_rx;
                                    }
                                }
                            }
                        });
                        return Ok(Box::pin(RecordBatchStreamAdapter::new(
                            self.schema.clone(),
                            ReceiverStream::new(stream_rx),
                        )));
                    }
                }
            }
        }

        // 7. L0 (Source): Read Oracle + Sidecar
        let (tx, rx) = mpsc::channel(10);

        spawn_oracle_fetch_and_sidecar(
            tx,
            user,
            pass,
            conn_str,
            table_name,
            schema.clone(),
            batch_size,
            fetch_strategy,
            where_clause,
            cache_key,
            volatility_policy,
            flight_guard,
        );

        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            ReceiverStream::new(rx),
        )))
    }
}

fn read_oracle_data_pagination(
    tx: mpsc::Sender<Result<RecordBatch>>,
    cache_tx: Option<mpsc::Sender<RecordBatch>>, // Optional Sidecar Channel
    user: String,
    pass: String,
    conn_str: String,
    table_name: String,
    schema: SchemaRef,
    batch_size: usize,
    fetch_strategy: OracleFetchStrategy,
    where_clause: Option<String>,
) -> Result<()> {
    let conn = Connection::connect(&user, &pass, &conn_str)
        .map_err(|e| DataFusionError::Execution(format!("Oracle connection failed: {}", e)))?;

    // Build base query
    let columns = schema
        .fields()
        .iter()
        .map(|f| f.name().as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let select_part = if columns.is_empty() {
        "1".to_string()
    } else {
        columns.clone()
    };

    let mut offset = 0;

    loop {
        // Construct SQL using Dialect
        let dialect = OracleDialect::new(matches!(
            fetch_strategy,
            OracleFetchStrategy::PaginationLegacy
        ));
        let sql = dialect.generate_pagination_sql(
            &select_part,
            &table_name,
            where_clause.as_deref(),
            batch_size,
            offset,
        );

        let stmt = conn.statement(&sql).build().map_err(|e| {
            DataFusionError::Execution(format!("Failed to prepare statement: {}", e))
        })?;

        let rows = stmt
            .query(&[])
            .map_err(|e| DataFusionError::Execution(format!("Failed to execute query: {}", e)))?;

        let col_types: Vec<DataType> = schema
            .fields()
            .iter()
            .map(|f| f.data_type().clone())
            .collect();

        // Builders
        let mut int_builders: Vec<Option<Int64Builder>> = col_types
            .iter()
            .map(|t| {
                if let DataType::Int64 = t {
                    Some(Int64Builder::new())
                } else {
                    None
                }
            })
            .collect();

        let mut float_builders: Vec<Option<Float64Builder>> = col_types
            .iter()
            .map(|t| {
                if let DataType::Float64 = t {
                    Some(Float64Builder::new())
                } else {
                    None
                }
            })
            .collect();

        let mut string_builders: Vec<Option<StringBuilder>> = col_types
            .iter()
            .map(|t| {
                if let DataType::Utf8 = t {
                    Some(StringBuilder::new())
                } else {
                    None
                }
            })
            .collect();

        let mut current_batch_rows = 0;

        for row_result in rows {
            let row = row_result
                .map_err(|e| DataFusionError::Execution(format!("Error reading row: {}", e)))?;

            for (i, dt) in col_types.iter().enumerate() {
                match dt {
                    DataType::Int64 => {
                        // Oracle usually returns Number/Float. Try to cast.
                        let val: Option<i64> = row.get(i).ok();
                        if let Some(builder) = &mut int_builders[i] {
                            builder.append_option(val);
                        }
                    }
                    DataType::Float64 => {
                        let val: Option<f64> = row.get(i).ok();
                        if let Some(builder) = &mut float_builders[i] {
                            builder.append_option(val);
                        }
                    }
                    DataType::Utf8 => {
                        let val: Option<String> = row.get(i).ok();
                        if let Some(builder) = &mut string_builders[i] {
                            builder.append_option(val);
                        }
                    }
                    _ => {}
                }
            }
            current_batch_rows += 1;
        }

        if current_batch_rows == 0 {
            break;
        }

        let batch = build_batch(
            &schema,
            &col_types,
            &mut int_builders,
            &mut float_builders,
            &mut string_builders,
            current_batch_rows,
        )?;

        // 1. Send to Downstream (DataFusion)
        if tx.blocking_send(Ok(batch.clone())).is_err() {
            return Ok(());
        }

        // 2. Send to Sidecar (Cache) - if enabled
        if let Some(cache) = &cache_tx {
            // Non-blocking send (drop if full? No, use blocking to ensure integrity as per guidelines)
            // But blocking here might slow down query.
            // Guideline says: "always use blocking_send (backpressure) instead of try_send (drop)" for sidecar.
            // So we use blocking_send.
            if cache.blocking_send(batch).is_err() {
                // Sidecar closed, ignore
            }
        }

        if current_batch_rows < batch_size {
            break; // Last page
        }

        offset += batch_size;
    }

    Ok(())
}

// Helper to build batch (Copied/Adapted from sqlite.rs or similar utility)
// Since this is a separate file, we need to duplicate or move to utils.
// For now, I'll duplicate it to keep the file self-contained as requested.
fn build_batch(
    schema: &SchemaRef,
    col_types: &[DataType],
    int_builders: &mut [Option<Int64Builder>],
    float_builders: &mut [Option<Float64Builder>],
    string_builders: &mut [Option<StringBuilder>],
    num_rows: usize,
) -> Result<RecordBatch> {
    let mut columns: Vec<std::sync::Arc<dyn datafusion::arrow::array::Array>> = Vec::new();

    for (i, dt) in col_types.iter().enumerate() {
        match dt {
            DataType::Int64 => {
                if let Some(builder) = &mut int_builders[i] {
                    columns.push(Arc::new(builder.finish()));
                    // Re-init builder
                    *builder = Some(Int64Builder::new());
                }
            }
            DataType::Float64 => {
                if let Some(builder) = &mut float_builders[i] {
                    columns.push(Arc::new(builder.finish()));
                    *builder = Some(Float64Builder::new());
                }
            }
            DataType::Utf8 => {
                if let Some(builder) = &mut string_builders[i] {
                    columns.push(Arc::new(builder.finish()));
                    *builder = Some(StringBuilder::new());
                }
            }
            _ => {}
        }
    }

    RecordBatch::try_new(schema.clone(), columns).map_err(DataFusionError::ArrowError)
}

fn spawn_oracle_fetch_and_sidecar(
    tx: mpsc::Sender<Result<RecordBatch>>,
    user: String,
    pass: String,
    conn_str: String,
    table_name: String,
    schema: SchemaRef,
    batch_size: usize,
    fetch_strategy: OracleFetchStrategy,
    where_clause: Option<String>,
    cache_key: String,
    volatility_policy: CachePolicy,
    flight_guard: Option<FlightGuard>,
) {
    let cache_path = CacheManager::get_cache_file_path(&cache_key);
    let (cache_tx, mut cache_rx) = mpsc::channel::<RecordBatch>(10);
    let cache_schema = schema.clone();
    let cache_path_clone = cache_path.clone();
    let cache_key_clone = cache_key.clone();

    // Sidecar Logic
    let enable_sidecar = volatility_policy == CachePolicy::UseCache;

    if enable_sidecar {
        let flight_guard_sidecar = flight_guard;
        tokio::spawn(async move {
            let _guard = flight_guard_sidecar;

            // Ensure directory exists
            if let Some(parent) = cache_path_clone.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            // Atomic Write pattern: .tmp -> .parquet
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_path =
                cache_path_clone.with_extension(format!("parquet.tmp.{}", unique_suffix));

            let mut l2_buffer = Vec::new();
            let mut use_l2 = true;
            let mut success = false;

            match TokioFile::create(&temp_path).await {
                Ok(file) => {
                    let start_time = std::time::Instant::now();
                    let mut writer = AsyncArrowWriter::try_new(file, cache_schema, None).unwrap();

                    while let Some(batch) = cache_rx.recv().await {
                        // Write to Disk (L1)
                        if let Err(e) = writer.write(&batch).await {
                            eprintln!("[Sidecar] Write error: {:?}", e);
                            break;
                        }
                        // Write to Memory Buffer (L2)
                        if use_l2 {
                            l2_buffer.push(batch);
                            if l2_buffer.len() > 1000 {
                                use_l2 = false;
                                l2_buffer.clear();
                            }
                        }
                    }

                    // Finalize
                    if writer.close().await.is_ok() {
                        if tokio::fs::rename(&temp_path, &cache_path_clone)
                            .await
                            .is_ok()
                        {
                            let cost = start_time.elapsed().as_millis() as u64;
                            // Register L1
                            if let Ok(meta) = std::fs::metadata(&cache_path_clone) {
                                CacheManager::put_l1(
                                    cache_key_clone.clone(),
                                    cache_path_clone,
                                    meta.len(),
                                    cost,
                                );
                                // Register L2
                                if use_l2 && !l2_buffer.is_empty() {
                                    CacheManager::put_l2(cache_key_clone, l2_buffer, cost);
                                }
                                success = true;
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[Sidecar] Failed to create cache file: {:?}", e),
            }

            if success {
                if let Some(g) = &_guard {
                    g.mark_completed();
                }
            } else {
                let _ = tokio::fs::remove_file(&temp_path).await;
                if let Some(g) = &_guard {
                    g.mark_failed();
                }
            }
        });
    }

    // Spawn Main Read Task
    tokio::task::spawn_blocking(move || {
        let _permit = if !enable_sidecar { None } else { None };

        if let Err(e) = read_oracle_data_pagination(
            tx,
            if enable_sidecar { Some(cache_tx) } else { None },
            user,
            pass,
            conn_str,
            table_name,
            schema,
            batch_size,
            fetch_strategy,
            where_clause,
        ) {
            println!("Error reading oracle data: {}", e);
        }
    });
}

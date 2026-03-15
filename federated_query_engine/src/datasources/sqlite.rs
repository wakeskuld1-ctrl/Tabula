use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
// use std::time::Duration;

use arrow::array::{ArrayBuilder, ArrayRef, Float64Builder, Int64Builder, StringBuilder};
use arrow::datatypes::{DataType, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionMode, ExecutionPlan, PlanProperties,
    SendableRecordBatchStream,
};
use datafusion::prelude::SessionContext;

use datafusion::parquet::arrow::{AsyncArrowWriter, ParquetRecordBatchStreamBuilder};
use futures::StreamExt;
use tokio::fs::File as TokioFile;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use rusqlite::{types::ValueRef, Connection};

use datafusion::physical_plan::memory::MemoryStream;

use crate::cache_manager::{get_metrics_registry, CacheManager, CachePolicy, FlightGuard};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum FetchStrategy {
    Cursor,
    Pagination,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SqliteExec {
    db_path: String,
    table_name: String,
    schema: SchemaRef,
    projection: Option<Vec<usize>>,
    batch_size: usize,
    fetch_strategy: FetchStrategy,
    limit: Option<usize>,
    properties: PlanProperties,
    metrics: ExecutionPlanMetricsSet,
    where_clause: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SqliteExecParams {
    pub db_path: String,
    pub table_name: String,
    pub schema: SchemaRef,
    pub projection: Option<Vec<usize>>,
    pub batch_size: usize,
    pub fetch_strategy: FetchStrategy,
    pub limit: Option<usize>,
    pub where_clause: Option<String>,
}

impl SqliteExec {
    pub fn new(params: SqliteExecParams) -> Self {
        let SqliteExecParams {
            db_path,
            table_name,
            schema,
            projection,
            batch_size,
            fetch_strategy,
            limit,
            where_clause,
        } = params;
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema.clone()),
            datafusion::physical_plan::Partitioning::UnknownPartitioning(1),
            ExecutionMode::Bounded,
        );

        Self {
            db_path,
            table_name,
            schema,
            projection,
            batch_size,
            fetch_strategy,
            limit,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
            where_clause,
        }
    }
}

impl DisplayAs for SqliteExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(
                    f,
                    "SqliteExec: db={}, table={}",
                    self.db_path, self.table_name
                )
            }
        }
    }
}

impl ExecutionPlan for SqliteExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        "SqliteExec"
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
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let (tx, rx) = mpsc::channel(10); // Backpressure
        let schema = self.schema.clone();
        let db_path = self.db_path.clone();
        let table_name = self.table_name.clone();
        let batch_size = self.batch_size;
        let where_clause = self.where_clause.clone();

        // 0. 获取源文件元数据 (mtime) 以保证缓存一致性
        // Check both .db and .db-wal (Write-Ahead Log) for modification time
        // In WAL mode, updates often only touch the -wal file until checkpoint.
        let mut source_mtime = if let Ok(metadata) = std::fs::metadata(&db_path) {
            metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        } else {
            0
        };

        let wal_path = format!("{}-wal", db_path);
        if let Ok(metadata) = std::fs::metadata(&wal_path) {
            let wal_mtime = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            if wal_mtime > source_mtime {
                source_mtime = wal_mtime;
            }
        }

        let cache_key = CacheManager::generate_key(
            &table_name,
            where_clause.as_deref(),
            self.projection.as_ref(),
            source_mtime,
        );

        // --- Volatility Check (Adaptive Cache Circuit Breaker) ---
        // Check if table is updating too frequently (volatile). If so, bypass L1/L2.
        use crate::cache_manager::CachePolicy;
        let volatility_policy = CacheManager::check_volatility(&table_name, source_mtime);
        if volatility_policy == CachePolicy::Bypass {
            println!("[SqliteExec] Volatility Bypass: Table '{}' is volatile. Skipping cache read/write.", table_name);
        }

        // --- 缓存逻辑 (L2 -> L1 -> L0) ---
        // 1. L2 (内存): 检查内存缓存是否存在
        if volatility_policy == CachePolicy::UseCache {
            if let Some(batches) = CacheManager::get_l2(&cache_key) {
                println!(
                    "[SqliteExec] Cache Hit (L2): Reading from Memory for key {}",
                    cache_key
                );
                return Ok(Box::pin(MemoryStream::try_new(batches, schema, None)?));
            }

            // 2. L1 (磁盘): 检查Parquet缓存是否存在。
            // Use get_l1_file to trigger metadata update
            if let Some(cache_path) = CacheManager::get_l1_file(&cache_key) {
                println!(
                    "[SqliteExec] Cache Hit (L1): Reading from Parquet for key {}",
                    cache_key
                );
                let (tx, rx) = mpsc::channel(2);
                let schema = self.schema.clone();

                // Check if we should promote this L1 entry to L2 (Memory)
                let should_promote = CacheManager::should_promote_to_l2(&cache_key);
                let promote_key = if should_promote {
                    Some(cache_key.clone())
                } else {
                    None
                };

                tokio::spawn(async move {
                    match TokioFile::open(&cache_path).await {
                        Ok(file) => {
                            // Integrity Check: Validate Parquet Footer
                            // If builder creation fails, file is likely corrupt/incomplete
                            match ParquetRecordBatchStreamBuilder::new(file).await {
                                Ok(builder) => {
                                    match builder.build() {
                                        Ok(mut stream) => {
                                            let mut l2_buffer = if promote_key.is_some() {
                                                Some(Vec::new())
                                            } else {
                                                None
                                            };
                                            let start_time = std::time::Instant::now();

                                            while let Some(batch_result) = stream.next().await {
                                                let res = batch_result.map_err(|e| {
                                                    DataFusionError::External(Box::new(e))
                                                });

                                                // Capture for L2 Promotion
                                                if let Ok(batch) = &res {
                                                    if let Some(buf) = &mut l2_buffer {
                                                        buf.push(batch.clone());
                                                        if buf.len() > 1000 {
                                                            l2_buffer = None;
                                                        } // Safety limit
                                                    }
                                                }

                                                if tx.send(res).await.is_err() {
                                                    break;
                                                }
                                            }

                                            get_metrics_registry().record_l1_io_latency(
                                                start_time.elapsed().as_micros() as u64,
                                            );

                                            // Execute Promotion
                                            if let Some(key) = promote_key {
                                                if let Some(buf) = l2_buffer {
                                                    if !buf.is_empty() {
                                                        let cost =
                                                            start_time.elapsed().as_millis() as u64;
                                                        println!("[SqliteExec] Promoting L1 -> L2 for key {}", key);
                                                        CacheManager::put_l2(key, buf, cost);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx
                                                .send(Err(DataFusionError::External(Box::new(e))))
                                                .await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[SqliteExec] Corrupt L1 cache file found (invalid footer): {:?}. Deleting...", cache_path);
                                    let _ = tokio::fs::remove_file(&cache_path).await;
                                    // We can't easily fallback to source here as we already committed to L1 path in this branch
                                    // But deleting it ensures next run will be a clean miss.
                                    // For now, return error to query.
                                    let _ =
                                        tx.send(Err(DataFusionError::External(Box::new(e)))).await;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(DataFusionError::IoError(e))).await;
                        }
                    }
                });

                return Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema,
                    ReceiverStream::new(rx),
                )));
            }
        }

        // 3. Singleflight: Request Coalescing
        // If multiple requests hit the same missing key, only one goes to DB.
        // Others wait for the first one to finish (populating L1/L2) and then retry.

        let mut flight_guard = None;

        if volatility_policy == CachePolicy::UseCache {
            match CacheManager::join_or_start_flight(cache_key.clone()) {
                crate::cache_manager::FlightResult::IsLeader(guard) => {
                    flight_guard = Some(guard);
                }
                crate::cache_manager::FlightResult::IsFollower(rx) => {
                    println!(
                        "[SqliteExec] Cache Stampede Protection: Waiting for key {}",
                        cache_key
                    );

                    let (tx, stream_rx) = mpsc::channel(10);
                    let key_clone = cache_key.clone();
                    let db_path_retry = db_path.clone();
                    let table_name_retry = table_name.clone();
                    let schema_retry = schema.clone();
                    let batch_size_retry = batch_size;
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
                                        "[SqliteExec] Singleflight: Reading from L2 for key {}",
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
                                        "[SqliteExec] Singleflight: Reading from L1 for key {}",
                                        key_clone
                                    );
                                    match TokioFile::open(&path).await {
                                        Ok(file) => {
                                            match ParquetRecordBatchStreamBuilder::new(file).await {
                                                Ok(builder) => match builder.build() {
                                                    Ok(mut stream) => {
                                                        while let Some(res) = stream.next().await {
                                                            let res = res.map_err(|e| {
                                                                DataFusionError::External(Box::new(
                                                                    e,
                                                                ))
                                                            });
                                                            if tx.send(res).await.is_err() {
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let _ = tx
                                                            .send(Err(DataFusionError::External(
                                                                Box::new(e),
                                                            )))
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
                                            let _ = tx.send(Err(DataFusionError::IoError(e))).await;
                                        }
                                    }
                                    break;
                                }

                                println!(
                                    "[SqliteExec] Cache entry missing after flight completion. Retrying..."
                                );
                            } else if status == crate::cache_manager::FlightStatus::InProgress {
                                continue;
                            }

                            println!(
                                "[SqliteExec] Flight failed/cancelled. Retrying as Leader for key {}",
                                key_clone
                            );
                            match CacheManager::join_or_start_flight(key_clone.clone()) {
                                crate::cache_manager::FlightResult::IsLeader(guard) => {
                                    spawn_fetch_and_sidecar(SpawnFetchRequest {
                                        tx,
                                        db_path: db_path_retry,
                                        table_name: table_name_retry,
                                        schema: schema_retry,
                                        batch_size: batch_size_retry,
                                        where_clause: where_clause_retry,
                                        cache_key: key_clone,
                                        volatility_policy: volatility_policy_retry,
                                        flight_guard: Some(guard),
                                    });
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

        // 4. L0 (源): 读取SQLite + Sidecar写入 (L1) + 内存填充 (L2)
        spawn_fetch_and_sidecar(SpawnFetchRequest {
            tx,
            db_path,
            table_name,
            schema: schema.clone(),
            batch_size,
            where_clause,
            cache_key,
            volatility_policy,
            flight_guard,
        });

        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            ReceiverStream::new(rx),
        )))
    }
}

struct SpawnFetchRequest {
    tx: mpsc::Sender<Result<RecordBatch, DataFusionError>>,
    db_path: String,
    table_name: String,
    schema: SchemaRef,
    batch_size: usize,
    where_clause: Option<String>,
    cache_key: String,
    volatility_policy: CachePolicy,
    flight_guard: Option<FlightGuard>,
}

fn spawn_fetch_and_sidecar(req: SpawnFetchRequest) {
    let SpawnFetchRequest {
        tx,
        db_path,
        table_name,
        schema,
        batch_size,
        where_clause,
        cache_key,
        volatility_policy,
        flight_guard,
    } = req;
    let cache_path = CacheManager::get_cache_file_path(&cache_key);
    let (cache_tx, mut cache_rx) = mpsc::channel::<RecordBatch>(500);
    let cache_schema = schema.clone();
    let cache_path_clone = cache_path.clone();
    let cache_key_clone = cache_key.clone();

    // Sidecar Logic
    let enable_sidecar = volatility_policy == CachePolicy::UseCache;

    if enable_sidecar {
        let flight_guard_sidecar = flight_guard;
        tokio::spawn(async move {
            let _guard = flight_guard_sidecar;

            println!("[Sidecar] Starting for key {}", cache_key_clone);
            CacheManager::check_l1_disk_eviction();

            if let Some(parent) = cache_path_clone.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            let mut l2_buffer = Vec::new();
            let mut use_l2 = true;

            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_path =
                cache_path_clone.with_extension(format!("parquet.tmp.{}", unique_suffix));
            let final_path = cache_path_clone.clone();

            match TokioFile::create(&temp_path).await {
                Ok(file) => {
                    let start_time = std::time::Instant::now();
                    let mut writer = AsyncArrowWriter::try_new(file, cache_schema, None).unwrap();
                    let mut failed = false;

                    while let Some(batch) = cache_rx.recv().await {
                        if let Err(e) = writer.write(&batch).await {
                            eprintln!("Sidecar write error: {:?}", e);
                            failed = true;
                            break;
                        }

                        if use_l2 {
                            l2_buffer.push(batch);
                            if l2_buffer.len() > 1000 {
                                use_l2 = false;
                                l2_buffer.clear();
                            }
                        }
                        tokio::task::yield_now().await;
                    }

                    if !failed {
                        if let Err(e) = writer.close().await {
                            eprintln!("Failed to close parquet writer: {:?}", e);
                            let _ = tokio::fs::remove_file(&temp_path).await;
                            if let Some(g) = &_guard {
                                g.mark_failed();
                            }
                            return;
                        }

                        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
                            eprintln!("Failed to rename cache file: {:?}", e);
                            let _ = tokio::fs::remove_file(&temp_path).await;
                            if let Some(g) = &_guard {
                                g.mark_failed();
                            }
                            return;
                        }

                        let cost_ms = start_time.elapsed().as_millis() as u64;
                        println!(
                            "[Sidecar] L1 Write Complete: {:?} ({} ms)",
                            final_path, cost_ms
                        );

                        if let Ok(metadata) = std::fs::metadata(&final_path) {
                            let size = metadata.len();
                            CacheManager::put_l1(
                                cache_key_clone.clone(),
                                final_path,
                                size,
                                cost_ms,
                            );

                            if use_l2 && !l2_buffer.is_empty() {
                                CacheManager::put_l2(cache_key_clone.clone(), l2_buffer, cost_ms);
                                println!(
                                    "[SqliteExec] L2 Cache Populated for key {}",
                                    cache_key_clone
                                );
                            }

                            if let Some(g) = &_guard {
                                g.mark_completed();
                            }
                        } else if let Some(g) = &_guard {
                            g.mark_failed();
                        }
                    } else {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        if let Some(g) = &_guard {
                            g.mark_failed();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Sidecar] Failed to create cache file: {:?}", e);
                    if let Some(g) = &_guard {
                        g.mark_failed();
                    }
                }
            }
        });
    }

    // Main Reader Task
    tokio::spawn(async move {
        let _permit = if volatility_policy == CachePolicy::Bypass {
            Some(CacheManager::acquire_bypass_permit().await)
        } else {
            None
        };

        let res = tokio::task::spawn_blocking(move || {
            read_sqlite_data(SqliteReadRequest {
                tx,
                cache_tx: if enable_sidecar { Some(cache_tx) } else { None },
                db_path,
                table_name,
                schema,
                _projection: None,
                batch_size,
                where_clause,
            })
        })
        .await;

        if let Err(e) = res {
            eprintln!("[SqliteExec] Task Join Error: {:?}", e);
        } else if let Ok(Err(e)) = res {
            eprintln!("[SqliteExec] Read SQLite Error: {:?}", e);
        }
    });
}

struct SqliteReadRequest {
    tx: mpsc::Sender<Result<RecordBatch, DataFusionError>>,
    cache_tx: Option<mpsc::Sender<RecordBatch>>,
    db_path: String,
    table_name: String,
    schema: SchemaRef,
    _projection: Option<Vec<usize>>,
    batch_size: usize,
    where_clause: Option<String>,
}

fn read_sqlite_data(req: SqliteReadRequest) -> Result<(), DataFusionError> {
    let SqliteReadRequest {
        tx,
        cache_tx,
        db_path,
        table_name,
        schema,
        _projection: _,
        batch_size,
        where_clause,
    } = req;
    let start_exec = std::time::Instant::now();
    let metrics = get_metrics_registry();
    metrics.record_l0_request();

    let conn = Connection::open(db_path).map_err(|e| DataFusionError::Execution(e.to_string()))?;

    let query = if let Some(where_c) = where_clause {
        format!("SELECT * FROM {} WHERE {}", table_name, where_c)
    } else {
        format!("SELECT * FROM {}", table_name)
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| DataFusionError::Execution(e.to_string()))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| DataFusionError::Execution(e.to_string()))?;

    // Builders
    let mut builders: Vec<Box<dyn ArrayBuilder>> = schema
        .fields()
        .iter()
        .map(|f| match f.data_type() {
            DataType::Int64 => Box::new(Int64Builder::new()) as Box<dyn ArrayBuilder>,
            DataType::Utf8 => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
            DataType::Float64 => Box::new(Float64Builder::new()) as Box<dyn ArrayBuilder>,
            _ => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
        })
        .collect();

    let mut row_count = 0;

    while let Ok(Some(row)) = rows.next() {
        for (i, field) in schema.fields().iter().enumerate() {
            let val = row.get_ref(i).unwrap();
            match field.data_type() {
                DataType::Int64 => {
                    let builder = builders[i]
                        .as_any_mut()
                        .downcast_mut::<Int64Builder>()
                        .unwrap();
                    match val {
                        ValueRef::Integer(v) => builder.append_value(v),
                        _ => builder.append_null(),
                    }
                }
                DataType::Utf8 => {
                    let builder = builders[i]
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap();
                    match val {
                        ValueRef::Text(v) => {
                            builder.append_value(std::str::from_utf8(v).unwrap_or(""))
                        }
                        _ => builder.append_null(),
                    }
                }
                DataType::Float64 => {
                    let builder = builders[i]
                        .as_any_mut()
                        .downcast_mut::<Float64Builder>()
                        .unwrap();
                    match val {
                        ValueRef::Real(v) => builder.append_value(v),
                        ValueRef::Integer(v) => builder.append_value(v as f64),
                        _ => builder.append_null(),
                    }
                }
                _ => {
                    let builder = builders[i]
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap();
                    builder.append_null();
                }
            }
        }
        row_count += 1;

        if row_count >= batch_size {
            let arrays: Vec<ArrayRef> = builders.iter_mut().map(|b| b.finish()).collect();
            let batch = RecordBatch::try_new(schema.clone(), arrays)
                .map_err(|e| DataFusionError::ArrowError(e, None))?;

            // Send to Main
            if tx.blocking_send(Ok(batch.clone())).is_err() {
                return Ok(());
            }

            // Send to Sidecar (blocking to ensure data integrity)
            if let Some(ctx) = &cache_tx {
                if ctx.blocking_send(batch.clone()).is_err() {
                    // println!("WARN: Sidecar channel closed!");
                }
            }

            row_count = 0;
        }
    }

    // Remaining rows
    if row_count > 0 {
        let arrays: Vec<ArrayRef> = builders.iter_mut().map(|b| b.finish()).collect();
        let batch = RecordBatch::try_new(schema.clone(), arrays)
            .map_err(|e| DataFusionError::ArrowError(e, None))?;

        if tx.blocking_send(Ok(batch.clone())).is_err() {
            return Ok(());
        }
        if let Some(ctx) = &cache_tx {
            if ctx.blocking_send(batch.clone()).is_err() {
                // println!("WARN: Sidecar channel closed!");
            }
        }
    }

    metrics.record_l0_latency(start_exec.elapsed().as_micros() as u64);
    Ok(())
}

use crate::datasources::DataSource;

pub struct SqliteTable {
    db_path: String,
    table_name: String,
    schema: SchemaRef,
}

#[async_trait]
impl TableProvider for SqliteTable {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        _filters: &[datafusion::logical_expr::Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(SqliteExec::new(SqliteExecParams {
            db_path: self.db_path.clone(),
            table_name: self.table_name.clone(),
            schema: self.schema.clone(),
            projection: projection.cloned(),
            batch_size: 1024,
            fetch_strategy: FetchStrategy::Cursor,
            limit,
            where_clause: None,
        })))
    }
}

#[derive(Clone)]
pub struct SqliteDataSource {
    name: String,
    path: String,
    table_name: String,
}

impl SqliteDataSource {
    pub fn new(name: String, path: String, table_name: String) -> Self {
        Self {
            name,
            path,
            table_name,
        }
    }

    pub fn list_tables(path: &str) -> Result<Vec<String>, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut tables = Vec::new();
        for name in rows {
            tables.push(name?);
        }
        Ok(tables)
    }
}

#[async_trait]
impl DataSource for SqliteDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            arrow::datatypes::Field::new("id", DataType::Int64, true), // Dummy schema
        ]));

        let table = SqliteTable {
            db_path: self.path.clone(),
            table_name: self.table_name.clone(),
            schema,
        };
        ctx.register_table(&self.name, Arc::new(table))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use datafusion::prelude::*; // Unused
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use rusqlite::Connection;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_l1_cache_hit() {
        let db_path = "test_cache_hit.db";
        let _ = std::fs::remove_file(db_path);

        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_test (id INTEGER, val TEXT)",
            [],
        )
        .unwrap();
        for i in 0..100 {
            conn.execute(
                "INSERT INTO cache_test (id, val) VALUES (?1, ?2)",
                (i, format!("val_{}", i)),
            )
            .unwrap();
        }
        conn.close().unwrap();

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("val", DataType::Utf8, true),
        ]));

        let exec = SqliteExec::new(SqliteExecParams {
            db_path: db_path.to_string(),
            table_name: "cache_test".to_string(),
            schema: schema.clone(),
            projection: None,
            batch_size: 1024,
            fetch_strategy: FetchStrategy::Cursor,
            limit: None,
            where_clause: None,
        });

        // 0. Setup Mock Environment
        // Mock disk usage to prevent eviction (Total: 100GB, Free: 50GB -> 50% usage)
        CacheManager::set_test_disk_usage(Some((
            100 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        )));

        // 1. Cache Miss (First run)
        println!("Running first query (Cache Miss)...");
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let batches: Vec<RecordBatch> = datafusion::physical_plan::common::collect(stream)
            .await
            .unwrap();
        assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 100);

        // Wait for Sidecar to write cache
        println!("Waiting for cache to be written...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let metadata = std::fs::metadata(db_path).unwrap();
        let mtime = metadata
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let cache_key = CacheManager::generate_key("cache_test", None, None, mtime);
        let cache_path = CacheManager::get_cache_file_path(&cache_key);
        assert!(
            cache_path.exists(),
            "Cache file should exist at {:?}",
            cache_path
        );

        // 2. Cache Hit (Second run)
        println!("Running second query (Cache Hit)...");
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let batches: Vec<RecordBatch> = datafusion::physical_plan::common::collect(stream)
            .await
            .unwrap();
        assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 100);

        // Cleanup
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(cache_path);
    }
}

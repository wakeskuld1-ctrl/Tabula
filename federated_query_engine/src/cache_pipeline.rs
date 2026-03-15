use crate::cache_manager::{CacheBackend, CachePolicy, FlightGuard};
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::error::{DataFusionError, Result};
use datafusion::parquet::arrow::ParquetRecordBatchStreamBuilder;
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::SendableRecordBatchStream;
use futures::StreamExt;
use std::sync::Arc;
use tokio::fs::File as TokioFile;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub struct CachePipelineInput {
    pub cache_key: String,
    pub schema: SchemaRef,
    pub volatility_policy: CachePolicy,
    pub flight_guard: Option<FlightGuard>,
}

pub struct CachePipeline;

impl CachePipeline {
    pub async fn run(
        cache_manager: Arc<dyn CacheBackend + Send + Sync>,
        input: CachePipelineInput,
        spawn_read_and_sidecar: Arc<
            dyn Fn(
                    Arc<dyn CacheBackend + Send + Sync>,
                    mpsc::Sender<Result<RecordBatch, DataFusionError>>,
                    String,
                    CachePolicy,
                    Option<FlightGuard>,
                ) + Send
                + Sync,
        >,
    ) -> Result<SendableRecordBatchStream> {
        // **[2026-02-26]** 变更原因：避免移动后再次借用 input 字段。
        // **[2026-02-26]** 变更目的：修复编译期 borrow of moved value。
        // **[2026-02-26]** 变更说明：先解构所需字段再进入缓存分支。
        // **[2026-02-26]** 变更说明：不改变缓存命中与单飞行语义。
        // **[2026-02-26]** 变更说明：仅调整局部变量引用方式。
        // **[2026-02-26]** 变更说明：保持外部接口与返回类型不变。
        let schema = input.schema.clone();
        let cache_key = input.cache_key.clone();
        let volatility_policy = input.volatility_policy;
        let mut flight_guard = input.flight_guard;
        if volatility_policy == CachePolicy::UseCache {
            if let Some(batches) = cache_manager.get_l2(&cache_key) {
                let stream = MemoryStream::try_new(batches, schema, None)?;
                return Ok(Box::pin(stream));
            }

            if let Some(cache_path) = cache_manager.get_l1_file(&cache_key) {
                let (tx, rx) = mpsc::channel(2);
                spawn_l1_reader(
                    Arc::clone(&cache_manager),
                    tx,
                    cache_path,
                    cache_key.clone(),
                );
                return Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema,
                    ReceiverStream::new(rx),
                )));
            }
        }

        if volatility_policy == CachePolicy::UseCache {
            match cache_manager.join_or_start_flight(cache_key.clone()) {
                crate::cache_manager::FlightResult::IsLeader(guard) => {
                    flight_guard = Some(guard);
                }
                crate::cache_manager::FlightResult::IsFollower(rx) => {
                    let (tx, stream_rx) = mpsc::channel(10);
                    let cache_manager_retry = Arc::clone(&cache_manager);
                    let spawn_retry = Arc::clone(&spawn_read_and_sidecar);
                    let schema_retry = schema.clone();
                    tokio::spawn(async move {
                        let mut current_rx = rx;
                        loop {
                            let status = if current_rx.changed().await.is_err() {
                                crate::cache_manager::FlightStatus::Failed
                            } else {
                                *current_rx.borrow()
                            };

                            if status == crate::cache_manager::FlightStatus::Completed {
                                if let Some(batches) = cache_manager_retry.get_l2(&cache_key) {
                                    for batch in batches {
                                        if tx.send(Ok(batch)).await.is_err() {
                                            break;
                                        }
                                    }
                                    break;
                                }

                                if let Some(path) = cache_manager_retry.get_l1_file(&cache_key) {
                                    spawn_l1_reader(
                                        Arc::clone(&cache_manager_retry),
                                        tx,
                                        path,
                                        cache_key.clone(),
                                    );
                                    break;
                                }
                            } else if status == crate::cache_manager::FlightStatus::InProgress {
                                continue;
                            }

                            match cache_manager_retry.join_or_start_flight(cache_key.clone()) {
                                crate::cache_manager::FlightResult::IsLeader(guard) => {
                                    (spawn_retry)(
                                        Arc::clone(&cache_manager_retry),
                                        tx,
                                        cache_key.clone(),
                                        volatility_policy,
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
                        schema_retry,
                        ReceiverStream::new(stream_rx),
                    )));
                }
            }
        }

        let (tx, rx) = mpsc::channel(2);
        (spawn_read_and_sidecar)(
            Arc::clone(&cache_manager),
            tx,
            cache_key,
            volatility_policy,
            flight_guard,
        );
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            schema,
            ReceiverStream::new(rx),
        )))
    }
}

fn spawn_l1_reader(
    cache_manager: Arc<dyn CacheBackend + Send + Sync>,
    tx: mpsc::Sender<Result<RecordBatch, DataFusionError>>,
    cache_path: std::path::PathBuf,
    cache_key: String,
) {
    let should_promote = cache_manager.should_promote_to_l2(&cache_key);
    let promote_key = if should_promote {
        Some(cache_key.clone())
    } else {
        None
    };
    let cache_manager_for_l1 = Arc::clone(&cache_manager);
    tokio::spawn(async move {
        match TokioFile::open(&cache_path).await {
            Ok(file) => match ParquetRecordBatchStreamBuilder::new(file).await {
                Ok(builder) => match builder.build() {
                    Ok(mut stream) => {
                        let mut l2_buffer = if promote_key.is_some() {
                            Some(Vec::new())
                        } else {
                            None
                        };
                        let start_time = std::time::Instant::now();

                        while let Some(batch_result) = stream.next().await {
                            let res =
                                batch_result.map_err(|e| DataFusionError::External(Box::new(e)));
                            if let Ok(batch) = &res {
                                if let Some(buf) = &mut l2_buffer {
                                    buf.push(batch.clone());
                                    if buf.len() > 1000 {
                                        l2_buffer = None;
                                    }
                                }
                            }

                            if tx.send(res).await.is_err() {
                                break;
                            }
                        }

                        cache_manager_for_l1
                            .metrics_registry()
                            .record_l1_io_latency(start_time.elapsed().as_micros() as u64);

                        if let Some(key) = promote_key {
                            if let Some(buf) = l2_buffer {
                                if !buf.is_empty() {
                                    let cost = start_time.elapsed().as_millis() as u64;
                                    cache_manager_for_l1.clone().put_l2(key, buf, cost);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(DataFusionError::External(Box::new(e)))).await;
                    }
                },
                Err(e) => {
                    let _ = tokio::fs::remove_file(&cache_path).await;
                    let _ = tx.send(Err(DataFusionError::External(Box::new(e)))).await;
                }
            },
            Err(e) => {
                let _ = tx.send(Err(DataFusionError::IoError(e))).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{CachePipeline, CachePipelineInput};
    use crate::cache_manager::{CacheBackend, CacheManager, CachePolicy, FlightGuard};
    use datafusion::arrow::array::Int64Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::error::DataFusionError;
    use datafusion::physical_plan::common::collect;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_cache_pipeline_l2_hit() {
        let cache_manager = Arc::new(CacheManager::new());
        let cache_backend: Arc<dyn CacheBackend + Send + Sync> = cache_manager.clone();
        let cache_key = "pipeline_l2_hit".to_string();
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int64, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int64Array::from(vec![1, 2, 3]))],
        )
        .unwrap();

        cache_backend
            .clone()
            .put_l2(cache_key.clone(), vec![batch.clone()], 1);

        let input = CachePipelineInput {
            cache_key,
            schema: schema.clone(),
            volatility_policy: CachePolicy::UseCache,
            flight_guard: None,
        };

        // **[2026-02-26]** 变更原因：闭包捕获 batch 导致仅实现 FnOnce。
        // **[2026-02-26]** 变更目的：满足 CachePipeline 回调的 Fn 约束。
        // **[2026-02-26]** 变更说明：通过 clone 保持多次调用安全。
        // **[2026-02-26]** 变更说明：不改变测试的 L2 命中语义。
        // **[2026-02-26]** 变更说明：仅调整测试回调的捕获方式。
        // **[2026-02-26]** 变更说明：避免编译器报错阻断重构。
        let called = Arc::new(AtomicUsize::new(0));
        let called_clone = called.clone();
        let batch_for_reader = batch.clone();
        let spawn_read_and_sidecar = Arc::new(
            move |_cache_manager: Arc<dyn CacheBackend + Send + Sync>,
                  tx: mpsc::Sender<Result<RecordBatch, DataFusionError>>,
                  _cache_key: String,
                  _volatility_policy: CachePolicy,
                  _flight_guard: Option<FlightGuard>| {
                let called = called_clone.clone();
                let batch_for_send = batch_for_reader.clone();
                tokio::spawn(async move {
                    called.fetch_add(1, Ordering::SeqCst);
                    let _ = tx.send(Ok(batch_for_send)).await;
                });
            },
        );
        let stream = CachePipeline::run(cache_backend, input, spawn_read_and_sidecar)
            .await
            .unwrap();
        let batches = collect(stream).await.unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 3);
        assert_eq!(called.load(Ordering::SeqCst), 0);
    }
}

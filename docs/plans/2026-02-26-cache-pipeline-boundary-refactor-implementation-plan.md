# 缓存管线与模块边界重构 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 sqlite/oracle 的缓存与单飞行逻辑抽取到 CachePipeline，明确模块边界且不改变行为。

**Architecture:** 新增 CachePipeline 统一处理 L1/L2/sidecar/singleflight，DataSource 仅提供数据读取策略与最小参数。

**Tech Stack:** Rust, DataFusion, Tokio

---

### Task 1: 引入 CachePipeline 模块与最小可编译接口

**Files:**
- Create: `d:\Rust\metadata\federated_query_engine\src\cache_pipeline.rs`
- Modify: `d:\Rust\metadata\federated_query_engine\src\main.rs`
- Test: `d:\Rust\metadata\federated_query_engine\src\cache_pipeline.rs` (cfg(test))

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_cache_pipeline_l2_hit() {
    let cache_manager = Arc::new(CacheManager::new());
    let key = "pipeline_l2_hit".to_string();
    let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int64, false)]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(Int64Array::from(vec![1, 2, 3]))],
    )
    .unwrap();

    cache_manager.clone().put_l2(key.clone(), vec![batch.clone()], 1);

    let input = CachePipelineInput {
        cache_key: key,
        schema: schema.clone(),
        volatility_policy: CachePolicy::UseCache,
        flight_guard: None,
    };

    let stream = CachePipeline::run(
        cache_manager,
        input,
        || async { Ok(vec![batch]) },
    )
    .await
    .unwrap();

    let batches: Vec<RecordBatch> =
        datafusion::physical_plan::common::collect(stream).await.unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 3);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine cache_pipeline::tests::test_cache_pipeline_l2_hit -v`
Expected: FAIL (CachePipeline/CachePipelineInput not found)

**Step 3: Write minimal implementation**

```rust
pub struct CachePipelineInput {
    pub cache_key: String,
    pub schema: SchemaRef,
    pub volatility_policy: CachePolicy,
    pub flight_guard: Option<FlightGuard>,
}

pub struct CachePipeline;

impl CachePipeline {
    pub async fn run<F, Fut>(
        cache_manager: Arc<dyn CacheBackend + Send + Sync>,
        input: CachePipelineInput,
        reader: F,
    ) -> Result<SendableRecordBatchStream>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Vec<RecordBatch>>> + Send,
    {
        if input.volatility_policy == CachePolicy::UseCache {
            if let Some(batches) = cache_manager.get_l2(&input.cache_key) {
                return Ok(Box::pin(RecordBatchStreamAdapter::new(
                    input.schema,
                    futures::stream::iter(batches.into_iter().map(Ok)),
                )));
            }
        }
        let batches = reader().await?;
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            input.schema,
            futures::stream::iter(batches.into_iter().map(Ok)),
        )))
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine cache_pipeline::tests::test_cache_pipeline_l2_hit -v`
Expected: PASS

**Step 5: Commit**

Skip commit unless user explicitly requests it.

---

### Task 2: 迁移 SQLite 执行路径到 CachePipeline

**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\datasources\sqlite.rs`
- Modify: `d:\Rust\metadata\federated_query_engine\src\cache_pipeline.rs`
- Test: `d:\Rust\metadata\federated_query_engine\src\datasources\sqlite.rs` (existing tests)

**Step 1: Write the failing test**

在现有 SQLite 测试中添加一个断言，验证 CachePipeline 处理 L2/L1 命中后仍返回正确行数。

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine sqlite -v`
Expected: FAIL (缓存逻辑尚未切换)

**Step 3: Write minimal implementation**

将 sqlite 的 L2/L1/singleflight/sidecar 逻辑集中到 CachePipeline：

```rust
let input = CachePipelineInput {
    cache_key,
    schema: self.schema.clone(),
    volatility_policy,
    flight_guard,
};

CachePipeline::run(cache_manager, input, || async move {
    read_sqlite_batches(db_path, table_name, schema, batch_size, where_clause).await
})
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine sqlite -v`
Expected: PASS

**Step 5: Commit**

Skip commit unless user explicitly requests it.

---

### Task 3: 迁移 Oracle 执行路径到 CachePipeline

**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\datasources\oracle.rs`
- Modify: `d:\Rust\metadata\federated_query_engine\src\cache_pipeline.rs`
- Test: `d:\Rust\metadata\federated_query_engine\src\datasources\oracle.rs` (新增最小单测或 feature 内测试)

**Step 1: Write the failing test**

在 oracle 模块内新增最小单测，验证 CachePipeline 在 L2 命中时不触发读取函数。

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine oracle -v`
Expected: FAIL (CachePipeline 未接入)

**Step 3: Write minimal implementation**

将 oracle 的 singleflight/sidecar/L1/L2 逻辑改为调用 CachePipeline，并在 reader 中保留读取流程：

```rust
CachePipeline::run(cache_manager, input, || async move {
    read_oracle_batches(user, pass, conn_str, table_name, schema, batch_size, fetch_strategy, where_clause).await
})
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine oracle -v`
Expected: PASS

**Step 5: Commit**

Skip commit unless user explicitly requests it.

---

### Task 4: 收敛边界与执行门禁

**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\main.rs`
- Modify: `d:\Rust\metadata\federated_query_engine\src\cache_manager.rs`

**Step 1: Write the failing test**

新增最小测试保证 cache_pipeline 的公开接口仅依赖 CacheBackend，不直接依赖 CacheManager。

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine cache_pipeline -v`
Expected: FAIL

**Step 3: Write minimal implementation**

清理 cache_pipeline 公开 API，使其仅依赖 CacheBackend trait；避免上层直接 import CacheManager。

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine cache_pipeline -v`
Expected: PASS

**Step 5: Commit**

Skip commit unless user explicitly requests it.

---

### Task 5: 全量验证

**Files:**
- Modify: None

**Step 1: Run format**

Run: `cargo fmt --all -- --check`
Expected: PASS

**Step 2: Run build**

Run: `cargo check`
Expected: PASS

**Step 3: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 4: Commit**

Skip commit unless user explicitly requests it.

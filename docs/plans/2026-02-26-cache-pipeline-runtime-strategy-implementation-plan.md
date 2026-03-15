# CachePipeline 运行时策略 Implementation Plan
 
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
 
**Goal:** 将 CachePipeline 同步入口限定在多线程 tokio runtime 中阻塞执行，并让相关测试显式使用 multi_thread。
 
**Architecture:** 在 `build_cache_pipeline_stream` 中检测当前 runtime flavor，仅允许 multi_thread 使用 `block_in_place + handle.block_on`；单线程 runtime 直接返回 `DataFusionError::Execution`。测试侧将调用该同步入口的 tokio 测试改为 multi_thread。
 
**Tech Stack:** Rust, tokio, datafusion
 
---
 
### Task 1: 约束同步入口的运行时类型
 
**Files:**
- Modify: `federated_query_engine/src/datasources/sqlite.rs:163-205`
 
**Step 1: 运行现有失败用例作为回归基线**
 
Run:
```
cargo test -p federated_query_engine sqlite::tests::test_sqlite_cache_pipeline_runner_not_called_on_l2_hit -v
```
Expected: FAIL with runtime flavor / block_in_place error
 
**Step 2: 写入最小实现，限制 multi_thread 才允许阻塞**
 
```rust
match tokio::runtime::Handle::try_current() {
    Ok(handle) => {
        if handle.runtime_flavor() != tokio::runtime::RuntimeFlavor::MultiThread {
            return Err(DataFusionError::Execution("...".to_string()));
        }
        tokio::task::block_in_place(|| handle.block_on(CachePipeline::run(...)))
    }
    Err(error) => {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(CachePipeline::run(...))
    }
}
```
 
**Step 3: 重新运行用例观察变化**
 
Run:
```
cargo test -p federated_query_engine sqlite::tests::test_sqlite_cache_pipeline_runner_not_called_on_l2_hit -v
```
Expected: FAIL with “需要 multi_thread runtime”错误（尚未改测试）
 
---
 
### Task 2: 将相关测试改为 multi_thread
 
**Files:**
- Modify: `federated_query_engine/src/datasources/sqlite.rs:860-1006`
 
**Step 1: 修改 tokio::test 注解**
 
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_cache_pipeline_runner_not_called_on_l2_hit() { ... }
```
 
**Step 2: 运行用例确认通过**
 
Run:
```
cargo test -p federated_query_engine sqlite::tests::test_sqlite_cache_pipeline_runner_not_called_on_l2_hit -v
```
Expected: PASS
 
---
 
### Task 3: 验证与约束执行
 
**Files:**
- None
 
**Step 1: 运行格式化检查**
 
Run:
```
cargo fmt --all -- --check
```
Expected: PASS
 
**Step 2: 运行类型检查**
 
Run:
```
cargo check
```
Expected: PASS
 
**Step 3: 运行 clippy**
 
Run:
```
cargo clippy --all-targets --all-features -D warnings
```
Expected: PASS
 
---
 
### Task 4: 提交（需用户授权）
 
**Files:**
- None
 
**Step 1: 若获授权再执行提交**
 
```
git add federated_query_engine/src/datasources/sqlite.rs docs/plans/2026-02-26-cache-pipeline-runtime-strategy-implementation-plan.md
git commit -m "fix: require multi-thread runtime for cache pipeline sync entry"
```

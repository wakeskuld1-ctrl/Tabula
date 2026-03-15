# Cache Manager Optimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 补齐缓存链路跨平台系统信息获取、L1写入驱逐触发、L2合并阈值与内存上限缓存，并用TDD验证。

**Architecture:** 在 `cache_manager.rs` 内新增私有字段与辅助方法，不改变对外接口；用 sysinfo 替换 PowerShell；L1 写入后触发节流驱逐检查；L2 合并加阈值；内存上限按时间缓存。

**Tech Stack:** Rust, DataFusion, sysinfo, tokio

---

### Task 1: 内存上限缓存与跨平台系统信息测试（红）

**Files:**
- Modify: `d:\Rust\metadata\.worktrees\cache-manager-optimization\federated_query_engine\src\cache_manager.rs` (tests module)

**Step 1: Write the failing test**

```rust
#[test]
fn test_memory_limit_cache_refresh_behavior() {
    let cache_manager = CacheManager::new();
    cache_manager.set_test_memory_limit(None);
    cache_manager.set_test_total_memory_bytes(Some(10 * 1024 * 1024));
    cache_manager.set_test_memory_limit_refresh_interval_ms(Some(1000));

    let first = cache_manager.get_memory_limit_bytes_with_now(10);
    let second = cache_manager.get_memory_limit_bytes_with_now(500);
    assert_eq!(first, second);

    let refreshed = cache_manager.get_memory_limit_bytes_with_now(2000);
    assert_eq!(refreshed, first);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_memory_limit_cache_refresh_behavior -p federated_query_engine`
Expected: FAIL (methods not found or behavior missing)

**Step 3: 记录变更（按需）**

如用户明确要求提交，再执行 git add/commit。

---

### Task 2: 跨平台系统信息与内存上限缓存实现（绿）

**Files:**
- Modify: `d:\Rust\metadata\.worktrees\cache-manager-optimization\federated_query_engine\src\cache_manager.rs`

**Step 1: Write minimal implementation**

```rust
fn get_total_memory_bytes(&self) -> usize { /* sysinfo + fallback */ }
fn get_memory_limit_bytes_with_now(&self, now_ms: u64) -> usize { /* cache + refresh */ }
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_memory_limit_cache_refresh_behavior -p federated_query_engine`
Expected: PASS

**Step 3: 记录变更（按需）**

如用户明确要求提交，再执行 git add/commit。

---

### Task 3: L1 写入触发驱逐节流测试与实现（红→绿）

**Files:**
- Modify: `d:\Rust\metadata\.worktrees\cache-manager-optimization\federated_query_engine\src\cache_manager.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_l1_eviction_check_throttled() {
    let cache_manager = CacheManager::new();
    cache_manager.set_test_l1_eviction_interval_ms(Some(1000));
    cache_manager.set_test_disk_usage(Some((100, 0)));

    let now = CacheManager::now_ms();
    cache_manager
        .l1_eviction_last_check_ms
        .store(now, Ordering::Relaxed);

    cache_manager.put_l1("k1".to_string(), PathBuf::from("cache/l1/a.parquet"), 1, 1);
    cache_manager.put_l1("k2".to_string(), PathBuf::from("cache/l1/b.parquet"), 1, 1);

    let last_check = cache_manager
        .l1_eviction_last_check_ms
        .load(Ordering::Relaxed);
    assert_eq!(last_check, now);

    let expired = now.saturating_sub(2000);
    cache_manager
        .l1_eviction_last_check_ms
        .store(expired, Ordering::Relaxed);
    cache_manager.put_l1("k3".to_string(), PathBuf::from("cache/l1/c.parquet"), 1, 1);
    let refreshed = cache_manager
        .l1_eviction_last_check_ms
        .load(Ordering::Relaxed);
    assert!(refreshed >= expired);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_l1_eviction_check_throttled -p federated_query_engine`
Expected: FAIL if throttle path missing; PASS once throttling is implemented

**Step 3: Write minimal implementation**

```rust
fn maybe_check_l1_disk_eviction(&self) { /* time gate + check */ }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_l1_eviction_check_throttled -p federated_query_engine`
Expected: PASS

**Step 5: 记录变更（按需）**

如用户明确要求提交，再执行 git add/commit。

---

### Task 4: L2 合并阈值测试与实现（红→绿）

**Files:**
- Modify: `d:\Rust\metadata\.worktrees\cache-manager-optimization\federated_query_engine\src\cache_manager.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_l2_compaction_threshold() {
    let cache_manager = Arc::new(CacheManager::new());
    cache_manager.set_test_l2_compaction_max_bytes(Some(1));
    cache_manager.set_test_memory_limit(Some(10 * 1024 * 1024));

    let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![1, 2, 3]))]).unwrap();
    cache_manager.put_l2("k".to_string(), vec![batch.clone(), batch], 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_l2_compaction_threshold -p federated_query_engine`
Expected: FAIL (threshold behavior missing)

**Step 3: Write minimal implementation**

```rust
let total_batch_bytes: usize = batches.iter().map(|b| b.get_array_memory_size()).sum();
if batches.len() > 1 && total_batch_bytes <= self.l2_compaction_max_bytes() { /* concat */ }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_l2_compaction_threshold -p federated_query_engine`
Expected: PASS

**Step 5: 记录变更（按需）**

如用户明确要求提交，再执行 git add/commit。

---

### Task 5: 全量验证

**Files:**
- Verify: `d:\Rust\metadata\.worktrees\cache-manager-optimization`

**Step 1: Run tests**

Run: `cargo test -p federated_query_engine`
Expected: PASS

**Step 2: Run lint**

Run: `cargo clippy -p federated_query_engine -- -D warnings`
Expected: PASS

**Step 3: Run typecheck**

Run: `cargo check -p federated_query_engine`
Expected: PASS

**Step 4: 记录变更（按需）**

如用户明确要求提交，再执行 git add/commit。

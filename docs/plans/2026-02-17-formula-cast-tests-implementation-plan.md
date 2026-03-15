# Formula Cast Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 添加 DataFusion/SQLite CAST 兼容性测试与 E2E 临时表覆盖，验证公式列路径稳定

**Architecture:** 后端在现有测试文件中新增两条集成测试（DataFusion 内存表与 SQLite 临时库），前端 E2E 自动创建临时表并验证公式列更新后 grid-data 拉取成功

**Tech Stack:** Rust (DataFusion, rusqlite), Node.js (Puppeteer), Axum API

---

### Task 1: 新增后端 DataFusion/SQLite 集成测试

**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\cache_e2e_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_cast_varchar_nullif_datafusion() {
    // 新增测试：空字符串与数值字符串的 CAST/NULLIF 兼容性
}

#[tokio::test]
async fn test_cast_varchar_nullif_sqlite() {
    // 新增测试：SQLite 侧的 CAST/NULLIF 兼容性
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine test_cast_varchar_nullif_datafusion`
Expected: FAIL (函数/逻辑尚未实现)

**Step 3: Write minimal implementation**

```rust
// DataFusion：SessionContext 注册内存表 + 执行 SQL，断言 NULL 与数值
// SQLite：创建临时 DB + 执行 SQL，断言 NULL 与数值
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine test_cast_varchar_nullif_datafusion`
Expected: PASS

**Step 5: Commit**

```bash
# 不执行提交（需用户明确授权）
```

---

### Task 2: 扩展 E2E 自动创建临时表并清理

**Files:**
- Modify: `d:\Rust\metadata\frontend\e2e_test.js`

**Step 1: Write the failing test**

```javascript
// E2E 中新增步骤：创建临时表 → 写入数据 → 更新公式列 → 再拉取 grid-data
```

**Step 2: Run test to verify it fails**

Run: `node e2e_test.js`
Expected: FAIL 或 SKIP（逻辑未实现）

**Step 3: Write minimal implementation**

```javascript
// 通过 /api/create_table /api/insert-column /api/insert-row /api/update-column-formula
// 完成临时表创建与公式列更新，并在结束时 /api/delete_table 清理
```

**Step 4: Run test to verify it passes**

Run: `node e2e_test.js`
Expected: PASS 或合理 SKIP（若后端不可用）

**Step 5: Commit**

```bash
# 不执行提交（需用户明确授权）
```

---

### Task 3: 全量验证

**Files:**
- None

**Step 1: Run format**

Run: `cargo fmt --all -- --check`
Expected: PASS

**Step 2: Run typecheck**

Run: `cargo check`
Expected: PASS

**Step 3: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 4: Run tests**

Run: `cargo test -p federated_query_engine test_cast_varchar_nullif_datafusion`
Expected: PASS

**Step 5: Commit**

```bash
# 不执行提交（需用户明确授权）
```

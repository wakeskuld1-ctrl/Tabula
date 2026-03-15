# Ensure Columns Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 新增 `/api/ensure_columns` 支持批量扩列（session 级）、幂等，并保证 `batch_update_cells` 可写新增列。

**Architecture:** 在 session_manager 中实现 ensure_columns 逻辑（检查 schema + insert_column），新增 API handler 与路由，补集成测试覆盖幂等与写入。

**Tech Stack:** Rust (axum, datafusion/arrow), existing SessionManager

---

### Task 1: 新增失败测试（TDD）

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: 写失败测试**
```rust
#[tokio::test]
async fn test_ensure_columns_idempotent_and_batch_update() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "ensure_cols",
        "id,amount\n1,10\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "columns": [
                { "name": "pivot_col_5", "type": "utf8" },
                { "name": "pivot_col_6", "type": "utf8" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["columns"].as_array().unwrap().iter().any(|v| v == "pivot_col_5"));

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "columns": [
                { "name": "pivot_col_5", "type": "utf8" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .post(format!("{}/api/batch_update_cells", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "updates": [
                { "row": 0, "col_name": "pivot_col_5", "new_value": "x" },
                { "row": 0, "col_name": "pivot_col_6", "new_value": "y" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}
```

**Step 2: 运行测试确认失败**
```
cargo test -p federated_query_engine test_ensure_columns_idempotent_and_batch_update -- --nocapture
```
Expected: FAIL（/api/ensure_columns 未实现）

---

### Task 2: 实现 ensure_columns（GREEN）

**Files:**
- Create: `federated_query_engine/src/api/ensure_columns_handler.rs`
- Modify: `federated_query_engine/src/api/mod.rs`
- Modify: `federated_query_engine/src/lib.rs`
- Modify: `federated_query_engine/src/session_manager/mod.rs`

**Step 1: SessionManager 新增 ensure_columns**
```rust
pub async fn ensure_columns(
    &self,
    table_name: &str,
    session_id: &str,
    columns: Vec<(String, DataType)>,
) -> Result<Vec<String>, UpdateCellError> {
    // 读取当前 schema
    // 对 columns 顺序遍历，若不存在则 insert_column(追加到末尾)
    // 返回最新列名列表
}
```

**Step 2: API handler**
```rust
#[derive(Deserialize)]
struct EnsureColumnsRequest {
  table_name: String,
  session_id: String,
  columns: Vec<EnsureColumnItem>,
}
#[derive(Deserialize)]
struct EnsureColumnItem { name: String, #[serde(rename = "type")] data_type: String }
```
- 解析 `data_type` -> Arrow `DataType`
- 调用 `SessionManager::ensure_columns`
- 返回 `{ status: "ok", columns: [...] }`

**Step 3: 路由**
- `POST /api/ensure_columns`

**Step 4: 运行测试确认通过**
```
cargo test -p federated_query_engine test_ensure_columns_idempotent_and_batch_update -- --nocapture
```
Expected: PASS

---

### Task 3: 编译检查

**Files:**
- None

**Step 1: 编译检查**
```
cargo test -p federated_query_engine --no-run
```
Expected: PASS

---

**Plan complete and saved to `docs/plans/2026-03-15-ensure-columns-implementation-plan.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**

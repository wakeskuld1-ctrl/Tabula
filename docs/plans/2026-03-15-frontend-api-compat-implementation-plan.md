# Frontend API Compatibility Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align backend handlers with frontend request/response expectations while keeping backward compatibility.

**Architecture:** Adjust only the HTTP handler layer to add `status` and optional `error` fields; preserve existing service/session logic and allow extra fields in responses.

**Tech Stack:** Rust, Axum, Serde, DataFusion, reqwest (tests)

---

### Task 1: Add failing integration tests for new response fields

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/tests/api_integration_test.rs`

**Step 1: Write the failing test**

Add assertions and new tests (keep existing flows intact):

```rust
// In test_positive_flow_create_update_save_and_read_consistency
assert_eq!(body["status"], "ok");

// After each /api/execute call in success flows
assert_eq!(body["status"], "ok");

#[tokio::test]
async fn test_execute_returns_status_on_error() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "execute_status_error",
        "id,name\n1,A\n",
    )
    .await;

    let res = client
        .post(format!("{}/api/execute", base_url))
        .json(&json!({
            "sql": format!("SELECT missing FROM \"{}\"", table_name)
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["error"].as_str().unwrap_or("").len() > 0);
}

#[tokio::test]
async fn test_update_style_range_error_includes_error_field() {
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": "",
            "range": { "start_col": 0, "start_row": 0, "end_col": 0, "end_row": 0 },
            "style": { "bold": true }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body.get("error").is_some());
}

#[tokio::test]
async fn test_ensure_columns_error_includes_error_field() {
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": "",
            "columns": [ { "name": "pivot_col_1", "type": "utf8" } ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body.get("error").is_some());
}
```

Also remove `/api/update_style_range` from `test_unimplemented_routes_return_404` because the route now exists.

**Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p federated_query_engine --test api_integration_test -- --nocapture
```

Expected: FAIL because `/api/execute` lacks `status` and error responses lack `error`.

**Step 3: Commit (tests only)**

```bash
git add D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/tests/api_integration_test.rs
git commit -m "test: assert status/error fields for frontend api"
```

---

### Task 2: Add status/error fields in handlers

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/execute_handler.rs`
- Modify: `D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/update_handler.rs`
- Modify: `D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/ensure_columns_handler.rs`

**Step 1: Write minimal implementation**

`execute_handler.rs` (wrap response with status using serde flatten):
```rust
#[derive(Serialize)]
struct ExecuteResponseWithStatus {
    status: String,
    #[serde(flatten)]
    data: ExecuteResponse,
}

pub(crate) async fn execute_sql(...) -> Json<ExecuteResponseWithStatus> {
    let response = execute_sql_service(&state, payload.sql).await;
    let status = if response.error.is_some() { "error" } else { "ok" };
    Json(ExecuteResponseWithStatus {
        status: status.to_string(),
        data: response,
    })
}
```

`update_handler.rs` (add `error` field on error/success):
```rust
Ok(msg) => Json(serde_json::json!({
    "status": "ok",
    "message": msg,
    "error": serde_json::Value::Null
}))

Err(e) => Json(serde_json::json!({
    "status": "error",
    "message": e,
    "error": e
}))
```

`ensure_columns_handler.rs` (include `error` in error responses, optional `message` on success):
```rust
Ok((effective_session_id, columns)) => AxumJson(serde_json::json!({
    "status": "ok",
    "session_id": effective_session_id,
    "columns": columns,
    "message": "columns ensured",
    "error": serde_json::Value::Null
}))

Err(e) => AxumJson(serde_json::json!({
    "status": "error",
    "message": e.message(),
    "code": e.code(),
    "details": e.details(),
    "error": e.message()
}))
```

Add date-stamped markdown comments **next to every changed block** to record reason/purpose per repo convention, and keep comment-to-code ratio at least 6:4 around modified sections.

**Step 2: Run tests**

Run:
```bash
cargo test -p federated_query_engine --test api_integration_test -- --nocapture
```
Expected: PASS.

**Step 3: Commit**

```bash
git add D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/execute_handler.rs
 git add D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/update_handler.rs
 git add D:/Rust/metadata/.worktrees/frontend-api-compat/federated_query_engine/src/api/ensure_columns_handler.rs
 git commit -m "feat: align frontend api response fields"
```

---

### Task 3: Final verification

**Step 1: Run targeted tests again**
```bash
cargo test -p federated_query_engine --test api_integration_test -- --nocapture
```
Expected: PASS.

**Step 2: Summarize changes and confirm next steps**
- Report updated endpoints and tests.
- Offer to proceed with branch integration using @finishing-a-development-branch if desired.

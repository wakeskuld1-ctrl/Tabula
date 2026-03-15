# Versions + Style Range APIs Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `/api/versions` and `/api/update_style_range` with optional `session_id`, keep newest-first ordering, and align frontend Time Machine requests.

**Architecture:** Extend SessionManager to resolve optional `session_id`, add new handlers and routes, then update frontend to pass `session_id` when available. Tests are written first and drive changes (TDD).

**Tech Stack:** Rust (axum, serde, lance), React/TypeScript

---

### Task 1: Add failing integration tests for new endpoints

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: Write failing tests for /api/update_style_range and /api/versions**

```rust
#[tokio::test]
async fn test_update_style_range_endpoint_ok() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_update_style_range",
        "id,name\n1,A\n2,B\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "range": { "start_row": 0, "start_col": 0, "end_row": 1, "end_col": 1 },
            "style": { "bold": true }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_versions_endpoint_supports_optional_session_id() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_versions",
        "id,name\n1,A\n2,B\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // No session_id
    let res = client
        .get(format!("{}/api/versions?table_name={}", base_url, table_name))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["versions"].is_array());

    // With session_id
    let res = client
        .get(format!(
            "{}/api/versions?table_name={}&session_id={}",
            base_url, table_name, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["versions"].is_array());
}
```

**Step 2: Update unimplemented route test**

Remove `/api/update_style_range` from `test_unimplemented_routes_return_404` (leave other endpoints).

**Step 3: Run tests to verify they fail**

Run:
```
cargo test -p federated_query_engine test_update_style_range_endpoint_ok -- --nocapture
cargo test -p federated_query_engine test_versions_endpoint_supports_optional_session_id -- --nocapture
```
Expected: FAIL with 404/405 or missing handler.

---

### Task 2: Implement backend handlers + SessionManager support

**Files:**
- Modify: `federated_query_engine/src/session_manager/mod.rs`
- Modify: `federated_query_engine/src/api/update_handler.rs`
- Modify: `federated_query_engine/src/api/session_handler.rs`
- Modify: `federated_query_engine/src/lib.rs`

**Step 1: Extend SessionManager to resolve optional session_id**

Add optional `session_id` support to `update_style_range` and `get_versions`:

```rust
pub async fn update_style_range(
    &self,
    table_name: &str,
    session_id: Option<&str>,
    range: MergeRange,
    style: CellStyle,
) -> Result<String, String> {
    // resolve session_id or active session
    // update metadata for that session
}

pub async fn get_versions(
    &self,
    table_name: &str,
    session_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    // resolve session_id or active session
    // open dataset and return versions sorted desc
}
```

**Step 2: Add update_style_range handler**

```rust
#[derive(Deserialize)]
pub struct UpdateStyleRangeRequest {
    pub table_name: String,
    pub session_id: Option<String>,
    pub range: crate::session_manager::MergeRange,
    pub style: crate::session_manager::CellStyle,
}

pub async fn update_style_range(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateStyleRangeRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .update_style_range(
            &payload.table_name,
            payload.session_id.as_deref(),
            payload.range,
            payload.style,
        )
        .await
    {
        Ok(msg) => Json(serde_json::json!({ "status": "ok", "message": msg })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e })),
    }
}
```

**Step 3: Add versions handler**

```rust
#[derive(Deserialize)]
pub struct VersionsQuery {
    pub table_name: String,
    pub session_id: Option<String>,
}

pub async fn get_versions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VersionsQuery>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .get_versions(&query.table_name, query.session_id.as_deref())
        .await
    {
        Ok(versions) => Json(serde_json::json!({ "status": "ok", "versions": versions })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e })),
    }
}
```

**Step 4: Wire routes**

```rust
.route("/api/update_style_range", post(api::update_handler::update_style_range))
.route("/api/versions", get(api::session_handler::get_versions))
```

**Step 5: Run tests and confirm pass**

Run:
```
cargo test -p federated_query_engine test_update_style_range_endpoint_ok -- --nocapture
cargo test -p federated_query_engine test_versions_endpoint_supports_optional_session_id -- --nocapture
```
Expected: PASS.

---

### Task 3: Update frontend Time Machine to pass session_id

**Files:**
- Modify: `frontend/src/components/TimeMachineDrawer.tsx`
- Modify: `frontend/src/App.tsx`

**Step 1: Add sessionId prop + query param**

```tsx
interface TimeMachineDrawerProps {
  tableName: string;
  sessionId?: string;
  onClose: () => void;
  onCheckout: (version: number) => void;
}
```

```tsx
const sessionParam = sessionId ? `&session_id=${encodeURIComponent(sessionId)}` : "";
const res = await fetch(`/api/versions?table_name=${encodeURIComponent(tableName)}${sessionParam}`);
```

**Step 2: Pass sessionId from App**

```tsx
<TimeMachineDrawer
  tableName={currentTable}
  sessionId={sessionId}
  onClose={...}
  onCheckout={...}
/>
```

**Step 3: (Optional) Frontend lint/build check**

Run:
```
cd frontend
npm test
```
Expected: PASS (if project has tests), otherwise skip.

---

### Task 4: Final verification + commit

**Step 1: Run relevant backend tests**

```
cargo test -p federated_query_engine api_integration_test -- --nocapture
```
Expected: PASS.

**Step 2: Commit**

```
git add federated_query_engine/src/session_manager/mod.rs \
        federated_query_engine/src/api/update_handler.rs \
        federated_query_engine/src/api/session_handler.rs \
        federated_query_engine/src/lib.rs \
        federated_query_engine/tests/api_integration_test.rs \
        frontend/src/components/TimeMachineDrawer.tsx \
        frontend/src/App.tsx

git commit -m "feat: add versions + style range endpoints"
```

---

**Plan complete and saved to `docs/plans/2026-03-15-versions-style-range.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**

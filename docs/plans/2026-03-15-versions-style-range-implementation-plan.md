# Versions + Style Range Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 先修复 `federated_query_engine` 基线编译错误，再实现 `/api/versions` 与 `/api/update_style_range`（支持可选 `session_id`，样式包含加粗/斜体/下划线/颜色/背景色）。

**Architecture:** 以 `SessionManager` 为会话与样式来源，新增 API handler 与路由；`session_id` 有值时按指定会话获取版本/样式范围，无值时沿用当前活动会话；基线编译修复先通过 `RegisterTableParams` 统一调用签名。

**Tech Stack:** Rust (axum, datafusion, lance, rusqlite), metadata_store

---

### Task 1: 复现基线编译失败（RED）

**Files:**
- None

**Step 1: 运行失败编译**

Run:
```
cargo test -p federated_query_engine --no-run
```
Expected: FAIL（旧的 `register_table` 调用签名仍在 `federated_query_engine/src/main.rs`）。

---

### Task 2: 修复遗留 register_table 调用（GREEN）

**Files:**
- Modify: `federated_query_engine/src/main.rs`

**Step 1: 定位旧签名调用并替换为 RegisterTableParams**

示例替换（按实际变量名调整）：
```rust
let params = RegisterTableParams {
    catalog: None,
    schema: None,
    table: table_name.as_str(),
    file_path: file_path.as_str(),
    source_type: source_type.as_str(),
    sheet_name: sheet_name.as_deref(),
    header_rows: None,
    header_mode: None,
};
metadata_manager.register_table(&ctx, params).await?;
```

**Step 2: 在修改处补充注释（原因+日期）**

---

### Task 3: 再次编译（GREEN）

**Files:**
- None

**Step 1: 运行编译检查**
```
cargo test -p federated_query_engine --no-run
```
Expected: PASS（至少可编译）。

---

### Task 4: /api/versions 失败测试（RED）

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: 添加失败测试**
```rust
#[tokio::test]
async fn test_versions_endpoint_with_and_without_session_id() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "versions",
        "a,b\n1,2\n",
    ).await;

    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .get(format!("{}/api/versions?table_name={}", base_url, table_name))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

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
}
```

**Step 2: 运行测试确认失败**
```
cargo test -p federated_query_engine test_versions_endpoint_with_and_without_session_id -- --nocapture
```
Expected: FAIL（/api/versions 未实现，返回 404 或 status != ok）。

---

### Task 5: 实现 /api/versions（GREEN）

**Files:**
- Create: `federated_query_engine/src/api/version_handler.rs`
- Modify: `federated_query_engine/src/api/mod.rs`
- Modify: `federated_query_engine/src/lib.rs`
- Modify: `federated_query_engine/src/session_manager/mod.rs`

**Step 1: 添加 handler 与路由**
```rust
// version_handler.rs
use axum::extract::{Query, State};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;

#[derive(Deserialize)]
pub struct VersionsQuery {
    pub table_name: String,
    pub session_id: Option<String>,
}

pub async fn get_versions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VersionsQuery>,
) -> axum::Json<serde_json::Value> {
    match state
        .session_manager
        .get_versions_with_session(&query.table_name, query.session_id.as_deref())
        .await
    {
        Ok(versions) => axum::Json(serde_json::json!({
            "status": "ok",
            "versions": versions
        })),
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}
```

**Step 2: SessionManager 增加 get_versions_with_session**
```rust
pub async fn get_versions_with_session(
    &self,
    table_name: &str,
    session_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let uri = match session_id {
        Some(id) => self.get_session_uri(id).await.ok_or("Session not found")?,
        None => self.get_active_session_uri(table_name)
            .await
            .ok_or("No active session found")?,
    };
    // 其余逻辑复用原 get_versions
}
```

**Step 3: 注释（原因+日期）**
- 在新增/修改处加入 Markdown 注释并写明原因/目的与日期。

**Step 4: 运行测试确认通过**
```
cargo test -p federated_query_engine test_versions_endpoint_with_and_without_session_id -- --nocapture
```
Expected: PASS

---

### Task 6: /api/update_style_range 失败测试（RED）

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: 添加失败测试**
```rust
#[tokio::test]
async fn test_update_style_range_with_optional_session_id() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "style_range",
        "a,b\n1,2\n",
    ).await;

    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": table_name,
            "range": { "start_row": 0, "start_col": 0, "end_row": 0, "end_col": 1 },
            "style": {
                "bold": true,
                "italic": true,
                "underline": true,
                "color": "#FF0000",
                "bg_color": "#00FF00"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "range": { "start_row": 0, "start_col": 0, "end_row": 0, "end_col": 1 },
            "style": { "bold": false }
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
cargo test -p federated_query_engine test_update_style_range_with_optional_session_id -- --nocapture
```
Expected: FAIL（/api/update_style_range 未实现）。

---

### Task 7: 实现 /api/update_style_range（GREEN）

**Files:**
- Modify: `federated_query_engine/src/api/update_handler.rs`
- Modify: `federated_query_engine/src/lib.rs`
- Modify: `federated_query_engine/src/session_manager/mod.rs`

**Step 1: handler 增加请求体与路由**
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
        .update_style_range_with_session(
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

**Step 2: SessionManager 增加 update_style_range_with_session**
```rust
pub async fn update_style_range_with_session(
    &self,
    table_name: &str,
    session_id: Option<&str>,
    range: MergeRange,
    style: CellStyle,
) -> Result<String, String> {
    let target_session_id = match session_id {
        Some(id) => id.to_string(),
        None => {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        }
    };
    // 复用现有 update_style_range 核心逻辑（改为使用 target_session_id）
}
```

**Step 3: 注释（原因+日期）**
- 在新增/修改处加入 Markdown 注释并写明原因/目的与日期。

**Step 4: 运行测试确认通过**
```
cargo test -p federated_query_engine test_update_style_range_with_optional_session_id -- --nocapture
```
Expected: PASS

---

**Plan complete and saved to `docs/plans/2026-03-15-versions-style-range-implementation-plan.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**

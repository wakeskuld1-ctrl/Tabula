# SessionId Align + Warnings Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 对齐前端 `/api/versions` 的 session_id 入参（空值传 null 并归一化），补回归测试并清理编译 warnings。

**Architecture:** 前端 TimeMachineDrawer 传入 sessionId，versions 请求带 session_id；后端归一化 "null"/空字符串为 None；补回归测试并清理未使用导入与 deprecated 调用。

**Tech Stack:** React/TypeScript, Rust (axum), cargo tests

---

### Task 1: 前端对齐 session_id（TDD 轻量回归）

**Files:**
- Modify: `frontend/src/components/TimeMachineDrawer.tsx`
- Modify: `frontend/src/App.tsx`

**Step 1: 写失败测试（若前端已有测试框架）**
- 如果 `frontend/tests` 中已有对应测试基础设施：
  - 新增测试验证 TimeMachineDrawer 请求 URL 包含 `session_id=null`。
- 若无合适框架：记录为手工回归，并跳过自动测试（由后端集成测试兜底）。

**Step 2: 实现前端参数对齐**
```tsx
interface TimeMachineDrawerProps {
  tableName: string;
  sessionId?: string;
  onClose: () => void;
  onCheckout: (version: number) => void;
}

const sessionQuery = sessionId ? sessionId : "null";
const res = await fetch(`/api/versions?table_name=${encodeURIComponent(tableName)}&session_id=${encodeURIComponent(sessionQuery)}`);
```

**Step 3: App 传入 sessionId**
```tsx
<TimeMachineDrawer tableName={currentTable} sessionId={sessionId} ... />
```

**Step 4: 手工回归/测试说明**
- 若无前端测试：记录手工回归步骤（打开时光机检查请求参数）。

---

### Task 2: 后端归一化 session_id（TDD）

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`
- Modify: `federated_query_engine/src/api/version_handler.rs`

**Step 1: 写失败测试（session_id=null）**
```rust
#[tokio::test]
async fn test_versions_endpoint_accepts_null_session_id() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "versions_null_sid",
        "id,amount\n1,10\n",
    )
    .await;
    let _ = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .get(format!(
            "{}/api/versions?table_name={}&session_id=null",
            base_url, table_name
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

**Step 2: 运行测试确认失败**
```
cargo test -p federated_query_engine test_versions_endpoint_accepts_null_session_id -- --nocapture
```
Expected: FAIL（session_id 未归一化导致 404/错误）。

**Step 3: 实现归一化**
```rust
let normalized = match query.session_id.as_deref() {
    Some("null") | Some("") => None,
    Some(v) => Some(v),
    None => None,
};
```

**Step 4: 运行测试确认通过**
```
cargo test -p federated_query_engine test_versions_endpoint_accepts_null_session_id -- --nocapture
```
Expected: PASS

---

### Task 3: 清理 warnings

**Files:**
- Modify: `federated_query_engine/src/services/register_service.rs`
- Modify: `federated_query_engine/src/session_manager/mod.rs`
- Modify: `federated_query_engine/src/lib.rs`
- Modify: `federated_query_engine/src/api/session_handler.rs`

**Step 1: 移除未使用 import / 变量**
- register_service.rs: 去掉 `RegisterTableParams`、`add_log` 未用导入
- session_manager/mod.rs: 删除未用 Timestamp* Array/Type 导入
- lib.rs: 删除未用 axum `Json/Multipart/State` 导入
- session_handler.rs: `state` 改为 `_state`

**Step 2: 替换 chrono deprecated 调用**
- 将 `.timestamp_millis()` 改为 `.and_utc().timestamp_millis()`
- 将 `.timestamp_micros()` 改为 `.and_utc().timestamp_micros()`

**Step 3: 编译检查**
```
cargo test -p federated_query_engine --no-run
```
Expected: PASS（warnings 降低或清零）

---

**Plan complete and saved to `docs/plans/2026-03-15-sessionid-align-warnings-implementation-plan.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**

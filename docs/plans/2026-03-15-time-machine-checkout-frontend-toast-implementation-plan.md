# Time Machine Checkout Frontend Toast Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 接入时光机回滚按钮调用 `/api/checkout_version` 并在失败时用现有 toast（debug-overlay）提示；补充 session/table mismatch 负向测试。

**Architecture:** 前端在 App 层集中发起回滚请求并控制 toast 展示，TimeMachineDrawer 仅触发回调；后端在集成测试中覆盖 session 与 table 不匹配的错误路径。

**Tech Stack:** React + TypeScript (frontend), Axum + Rust integration tests (backend)

---

### Task 1: 添加后端负向测试（session/table mismatch）

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: Write the failing test**

```rust
// **[2026-03-15]** Reason: TDD for checkout mismatch session/table.
// **[2026-03-15]** Purpose: ensure backend rejects cross-table session rollback.
#[tokio::test]
async fn test_checkout_version_session_table_mismatch() {
    let (client, base_url) = spawn_test_server().await;
    let table_a = register_csv_table(&client, &base_url, "checkout_mismatch_a", "id,name\n1,Alice\n").await;
    let table_b = register_csv_table(&client, &base_url, "checkout_mismatch_b", "id,name\n1,Bob\n").await;

    let session_id = create_session_and_get_id(&client, &base_url, &table_a).await;

    // Trigger versions to ensure checkout target exists
    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": table_a,
            "session_id": session_id,
            "row": 0,
            "col_name": "name",
            "new_value": "Zed"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let versions = wait_for_versions(&client, &base_url, &table_a, Some(&session_id)).await;
    let min_version = versions
        .iter()
        .filter_map(|v| v["version"].as_u64())
        .min()
        .unwrap_or(0);

    let res = client
        .post(format!(
            "{}/api/checkout_version?table_name={}&version={}&session_id={}",
            base_url, table_b, min_version, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap_or("").contains("Session table mismatch"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine test_checkout_version_session_table_mismatch -- --nocapture`
Expected: FAIL with `status` being `ok` or error message not matching.

**Step 3: Write minimal implementation**

If backend already rejects mismatch, no code change needed. Otherwise add guard to `SessionManager::checkout_version_with_session` to validate `session.table_name == table_name` and return `Err("Session table mismatch")`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine test_checkout_version_session_table_mismatch -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add federated_query_engine/tests/api_integration_test.rs

git commit -m "test: cover checkout version session/table mismatch"
```

### Task 2: 前端接入回滚接口 + toast 错误提示

**Files:**
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/utils/GridAPI.ts`
- Modify: `frontend/src/App.css`

**Step 1: Write the failing test**

Add a small unit test around a new URL builder function (e.g. `buildCheckoutVersionUrl`) in `frontend/src/utils/GridAPI.ts`.

```ts
import { buildCheckoutVersionUrl } from "../GridAPI";

test("buildCheckoutVersionUrl omits empty session_id", () => {
  expect(buildCheckoutVersionUrl("orders", 2, "")).toBe("/api/checkout_version?table_name=orders&version=2");
});
```

**Step 2: Run test to verify it fails**

Run: `npx tsc tests/buildCheckoutVersionUrl.test.ts src/utils/GridAPI.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck`
Then: `node .tmp-test/tests/buildCheckoutVersionUrl.test.js`
Expected: FAIL (function not defined).

**Step 3: Write minimal implementation**

- 在 `GridAPI.ts` 新增 `buildCheckoutVersionUrl` 与 `checkoutVersion`。
- 在 `App.tsx` 的 `onCheckout` 里调用 `checkoutVersion`，失败时用 `debug-overlay` 显示 toast。
- 添加 toast 自动消失（3s）逻辑，避免常驻。
- 成功后刷新网格：`gridRef.current?.refresh()` + `fetchTableData(currentTable)`。

**Step 4: Run test to verify it passes**

Run: `npx tsc tests/buildCheckoutVersionUrl.test.ts src/utils/GridAPI.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck`
Then: `node .tmp-test/tests/buildCheckoutVersionUrl.test.js`
Expected: PASS.

**Step 5: Commit**

```bash
git add frontend/tests/buildCheckoutVersionUrl.test.ts frontend/src/utils/GridAPI.ts frontend/src/App.tsx frontend/src/App.css

git commit -m "feat: wire time machine checkout with toast"
```

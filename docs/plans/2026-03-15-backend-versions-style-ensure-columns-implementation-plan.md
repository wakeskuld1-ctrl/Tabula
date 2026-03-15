# Backend APIs: versions, update_style_range, ensure_columns Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose `/api/versions`, `/api/update_style_range`, and `/api/ensure_columns` in the main repository with consistent `session_id` rules and aligned routing for `lib.rs` + `main.rs`.

**Architecture:** Add API handlers under `src/api`, normalize `session_id` in handlers, call `SessionManager` methods, register routes in `lib.rs`, and make `main.rs` use `create_app()` so both entrypoints share the router.

**Tech Stack:** Rust, axum, serde, datafusion, lance, tokio.

---

### Task 1: Add failing integration tests for versions/style range

**Files:**
- Modify: `D:\Rust\metadata\federated_query_engine\tests\api_integration_test.rs`

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn test_versions_endpoint_with_and_without_session_id() {
    // setup app + table, call /api/versions with session_id and without it
    // assert status ok and response shape
}

#[tokio::test]
async fn test_update_style_range_applies_style() {
    // setup app + table, call /api/update_style_range, then read back style
}
```

**Step 2: Run tests to confirm failure**

Run: `cargo test -p federated_query_engine test_versions_endpoint_with_and_without_session_id -- --nocapture`  
Expected: FAIL with 404/405 (route missing).

**Step 3: Commit**

```bash
git add D:\Rust\metadata\federated_query_engine\tests\api_integration_test.rs
git commit -m "test: add versions/update_style_range integration tests"
```

---

### Task 2: Implement versions/update_style_range handlers + routing

**Files:**
- Create: `D:\Rust\metadata\federated_query_engine\src\api\versions_handler.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\api\update_handler.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\api\mod.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\lib.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\main.rs`

**Step 1: Implement handler logic**

```rust
// normalize session_id: empty/"null" => None
// call SessionManager::get_versions or update_style_range
```

**Step 2: Register routes**

```rust
// lib.rs create_app():
// .route("/api/versions", get(api::versions_handler::get_versions))
// .route("/api/update_style_range", post(api::update_handler::update_style_range))
```

**Step 3: Align main.rs**

```rust
#[tokio::main]
async fn main() {
    federated_query_engine::run().await;
}
```

**Step 4: Run tests**

Run: `cargo test -p federated_query_engine test_versions_endpoint_with_and_without_session_id -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add D:\Rust\metadata\federated_query_engine\src\api D:\Rust\metadata\federated_query_engine\src\lib.rs D:\Rust\metadata\federated_query_engine\src\main.rs
git commit -m "feat: add versions and update_style_range endpoints"
```

---

### Task 3: Add ensure_columns (tests + handler + SessionManager)

**Files:**
- Modify: `D:\Rust\metadata\federated_query_engine\tests\api_integration_test.rs`
- Create: `D:\Rust\metadata\federated_query_engine\src\api\ensure_columns_handler.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\api\mod.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\lib.rs`
- Modify: `D:\Rust\metadata\federated_query_engine\src\session_manager\mod.rs`

**Step 1: Write failing test**

```rust
#[tokio::test]
async fn test_ensure_columns_idempotent_and_batch_update() {
    // call /api/ensure_columns twice with same columns
    // then call /api/batch_update_cells to write into new columns
}
```

**Step 2: Run test to confirm failure**

Run: `cargo test -p federated_query_engine test_ensure_columns_idempotent_and_batch_update -- --nocapture`  
Expected: FAIL with 404/405 (route missing).

**Step 3: Implement ensure_columns**

```rust
// SessionManager::ensure_columns:
// - load session schema
// - append missing columns in request order
// - idempotent: skip existing
// - return (effective_session_id, columns list)
```

**Step 4: Add handler + route**

```rust
// POST /api/ensure_columns
// parse { table_name, session_id, columns[] { name, type } }
// normalize session_id and call SessionManager::ensure_columns
```

**Step 5: Run test to confirm pass**

Run: `cargo test -p federated_query_engine test_ensure_columns_idempotent_and_batch_update -- --nocapture`  
Expected: PASS.

**Step 6: Commit**

```bash
git add D:\Rust\metadata\federated_query_engine\src D:\Rust\metadata\federated_query_engine\tests\api_integration_test.rs
git commit -m "feat: add ensure_columns endpoint with idempotent expansion"
```

---

### Task 4: Final verification + task journal

**Files:**
- Modify: `D:\Rust\metadata\.trae\CHANGELOG_TASK.md`

**Step 1: Run focused verification**

Run: `cargo test -p federated_query_engine test_ensure_columns_idempotent_and_batch_update -- --nocapture`  
Expected: PASS.

**Step 2: Update task journal**

Add a new entry describing the changes and tests run.


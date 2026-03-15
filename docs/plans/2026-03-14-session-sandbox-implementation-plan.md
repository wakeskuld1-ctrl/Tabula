# Session Sandbox + Read-Only Default Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the top selector choose a table, and the bottom tabs list table-scoped sandboxes (sessions) with a read-only default; fix merge background color and prevent invalid session_id usage.

**Architecture:** Add backend APIs for listing/switching sessions; wire frontend to fetch sessions per table, name new sandboxes as SheetN, and switch active sessions; enforce read-only default in GlideGrid; remove hard-coded merge white background.

**Tech Stack:** Rust (axum), TypeScript/React, Glide Data Grid.

---

### Task 1: Add failing tests for sessions APIs (TDD)

**Files:**
- Modify: `federated_query_engine/tests/api_integration_test.rs`

**Step 1: Write failing tests**

Add tests that expect:
- `GET /api/sessions?table_name=...` returns status ok and includes created sessions
- `POST /api/switch_session` switches active session

**Step 2: Run tests (expect RED)**

Run:
```
cargo test -p tabula-server --test api_integration_test
```
Expected: new tests fail (endpoint missing / 404)

**Step 3: Commit**

```
git add federated_query_engine/tests/api_integration_test.rs
git commit -m "test: add sessions list/switch API tests"
```

---

### Task 2: Implement backend sessions list/switch APIs

**Files:**
- Modify: `federated_query_engine/src/session_manager/mod.rs`
- Modify: `federated_query_engine/src/api/session_handler.rs`
- Modify: `federated_query_engine/src/lib.rs`

**Step 1: Add active session getter**

Add to SessionManager:
```rust
pub async fn get_active_session_id(&self, table_name: &str) -> Option<String> {
    let active = self.active_table_sessions.lock().await;
    active.get(table_name).cloned()
}
```

**Step 2: Add list/switch handlers**

`GET /api/sessions` (Query: table_name) -> returns sessions list + active_session_id  
`POST /api/switch_session` (table_name, session_id) -> switches active session

**Step 3: Register routes**

Add routes in `create_app()`:
- `.route("/api/sessions", get(api::session_handler::list_sessions))`
- `.route("/api/switch_session", post(api::session_handler::switch_session))`

**Step 4: Run tests (expect GREEN)**

Run:
```
cargo test -p tabula-server --test api_integration_test
```
Expected: all tests pass

**Step 5: Commit**

```
git add federated_query_engine/src/session_manager/mod.rs federated_query_engine/src/api/session_handler.rs federated_query_engine/src/lib.rs
git commit -m "feat: add sessions list/switch APIs"
```

---

### Task 3: Wire frontend table → sessions and SheetBar

**Files:**
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/components/layout/SheetBar.tsx` (if needed for labels)

**Step 1: Add sessions state + fetch**

Add:
- `sessions` array with session_id/name/is_default/created_at
- `activeSessionId` (sessionId)
- `isReadOnly` flag

Fetch sessions on table change:
- call `/api/sessions?table_name=...`
- set sessions list (default first, then others by created_at desc)
- auto-select default session and call `/api/switch_session`

**Step 2: Update SheetBar data source**

Pass session display names instead of table names:
- default session → label `默认/只读`
- others → `name` (SheetN)

When user clicks a tab:
- call `/api/switch_session`
- update `sessionId`
- set `isReadOnly` if default

**Step 3: Create sandbox as SheetN**

When creating:
- compute next SheetN from existing names
- call `/api/create_session` with `from_session_id` (not base_session_id)
- refresh sessions list
- switch to the new session

**Step 4: Commit**

```
git add frontend/src/App.tsx frontend/src/components/layout/SheetBar.tsx
git commit -m "feat: drive sheet tabs from sessions"
```

---

### Task 4: Enforce read-only default + fix merge background

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`

**Step 1: Add readOnly prop**

Add `readOnly?: boolean` to props and handle in:
- `getCellContent` (force readonly)
- edit entry points (`onCellsEdited`, `updateCell`, `paste`, `updateSelectionStyle`, `mergeSelection`, etc.)

**Step 2: Normalize empty session id**

Ensure `session_id` is omitted if empty string to avoid “会话不存在”.

**Step 3: Fix merge background**

Replace hard-coded `bgCell = "#ffffff"` with `customTheme.bgCell`.

**Step 4: Manual check**

Verify:
- Default session is read-only
- Merge keeps dark background
- Batch update works with active session

**Step 5: Commit**

```
git add frontend/src/components/GlideGrid.tsx
git commit -m "fix: read-only default and merge styling"
```

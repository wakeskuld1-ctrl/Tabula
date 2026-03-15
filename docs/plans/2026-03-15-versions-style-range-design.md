# Versions + Style Range API Design

**Date:** 2026-03-15

## Goal
Add missing backend routes for `/api/versions` (Time Machine) and `/api/update_style_range`, with optional `session_id` support and newest-first version ordering.

## Scope
- Backend: add routes + handlers; extend SessionManager to resolve optional `session_id`.
- Frontend: pass `session_id` to `/api/versions` when available.
- Tests: add/adjust integration coverage for new routes.

## Non-Goals
- No changes to existing update style semantics beyond range support.
- No UI/UX redesign for the Time Machine drawer.

## API Contract
### POST /api/update_style_range
Request:
```json
{
  "table_name": "...",
  "session_id": "...", // optional
  "range": { "start_row": 0, "start_col": 0, "end_row": 1, "end_col": 2 },
  "style": { "bold": true, "bg_color": "#fff" }
}
```
Response:
```json
{ "status": "ok", "message": "Style range updated" }
```
Error:
```json
{ "status": "error", "message": "Session not found" }
```

### GET /api/versions
Request:
```
/api/versions?table_name=...&session_id=... // session_id optional
```
Response:
```json
{
  "status": "ok",
  "versions": [
    { "version": 3, "timestamp": 1700000000, "metadata": {} }
  ]
}
```

## Session Resolution Rules
- If `session_id` is provided and exists, use it.
- If `session_id` is not provided, use the current active session for the table.
- If `session_id` is invalid, return an error.

## Version Ordering
- Sort by `version` descending (newest first).

## Error Handling
- Mirror existing patterns (`status: error`, `message: ...`).
- For invalid session, return a clear error message.

## Frontend Alignment
- `TimeMachineDrawer` passes `session_id` when available.
- If no `session_id`, it omits the param (backend uses active session).

## Tests
- Update the unimplemented-route test to remove the route now implemented.
- Add integration tests for:
  - `/api/update_style_range` returns `ok` on valid input.
  - `/api/versions` works with and without `session_id`.
  - Versions are ordered newest first.

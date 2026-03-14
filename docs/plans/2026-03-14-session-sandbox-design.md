# Design: Table-Scoped Sandboxes (Sessions) + Read-Only Default

Date: 2026-03-14  
Status: Approved

## Goal
Align UI behavior with Excel-style sheets:
- Top selector chooses **table**
- Bottom tabs show **sandboxes (sessions) for the selected table**
- Default session is **read-only**
- New sandboxes are named **Sheet1 / Sheet2 / ...**, ordered newest → oldest

Also fix merge-cell background to match the dark theme (no forced white).

## Scope
**Backend**
1) Add API to list sessions for a table  
2) Add API to switch active session for a table

**Frontend**
1) Replace bottom `SheetBar` data source from tables → sessions  
2) Create session names as SheetN  
3) Read-only mode for default session  
4) Normalize empty `session_id` so batch update doesn’t send invalid session  
5) Merge cell background uses theme color (not white)

## Architecture
Backend modules:
- `federated_query_engine/src/api/session_handler.rs` (add list/switch handlers)
- `federated_query_engine/src/lib.rs` (register routes)
- `federated_query_engine/src/session_manager/mod.rs` (expose active session id)
- `federated_query_engine/tests/api_integration_test.rs` (TDD tests)

Frontend modules:
- `frontend/src/App.tsx` (sessions list, table → sessions wiring)
- `frontend/src/components/GlideGrid.tsx` (readOnly behavior, merge bg)

## API Contracts
### GET /api/sessions
Request: `?table_name=users`  
Response:
```json
{
  "status": "ok",
  "table_name": "users",
  "active_session_id": "uuid",
  "sessions": [
    {
      "session_id": "uuid",
      "name": "Sheet1",
      "is_default": false,
      "created_at": 1710420000,
      "from_session_id": "uuid"
    }
  ]
}
```

### POST /api/switch_session
Request:
```json
{ "table_name": "users", "session_id": "uuid" }
```
Response:
```json
{ "status": "ok", "session_id": "uuid" }
```

## UI Behavior
1) **Top selector = table** (unchanged)
2) **Bottom tabs = sessions**
   - Default session shown as **“默认/只读”**
   - Other sessions use their `name` (SheetN)
   - Ordering: default first, then others by created_at desc (new → old)
3) **Create sandbox**
   - `session_name = SheetN` (smallest missing or next max + 1)
   - `from_session_id = current session` (if any)
4) **Switch sandbox**
   - Calls `/api/switch_session`
   - Updates `sessionId` + `readOnly` state
5) **Read-only default**
   - Disable editing, paste, merge, style updates, batch updates

## Merge Styling
In `GlideGrid.tsx`, remove hard-coded white background for merge master cells.
Use theme background (`customTheme.bgCell`) so merge area stays dark.

## Error Handling
1) If `/api/sessions` fails: show debug message, keep UI stable
2) If `/api/switch_session` fails: keep current session, show error
3) If create session fails: show error and do not change selection

## Testing
Add TDD tests:
- `/api/sessions` returns ok and includes the new session
- `/api/switch_session` switches active session for a table
- Frontend read-only behavior verified manually

# Frontend API Compatibility Design

> **Scope**: Align backend request/response shapes with the frontend's current calling conventions while preserving backward compatibility and allowing extra fields in responses.

## Goals
- Match frontend-expected response fields for the listed endpoints.
- Preserve existing request field aliases and extra response fields.
- Keep changes limited to HTTP handler layer where possible.

## Non-Goals
- Changing core data/session logic in `SessionManager` or services.
- Introducing new endpoints beyond the provided list.
- Enforcing strict response schemas that would reject extra fields.

## API Specification

### POST `/api/batch_update_cells`
**Request:**
- Accepts `table_name`, optional `session_id`, and `updates` items with `row`, `col`, `val` (existing aliases remain supported).

**Response (success):**
```json
{ "status": "ok", "session_id": "optional", "message": "optional" }
```

**Response (error):**
```json
{ "status": "error", "message": "...", "error": "..." }
```

### GET `/api/grid-data`
**Request (query params):**
`table_name`, `page`, `page_size`, optional `session_id`, `filters`, `sort`.

**Response (success):**
```json
{
  "status": "ok",
  "data": [["..."]],
  "columns": ["col_0", "col_1"],
  "column_types": ["utf8", "utf8"],
  "total_rows": 100,
  "metadata": {},
  "formula_columns": []
}
```

### POST `/api/execute`
**Request:**
```json
{ "sql": "SELECT ..." }
```

**Response (success):**
```json
{ "status": "ok", "columns": ["col1"], "rows": [[1, "a"]] }
```

**Response (error):**
```json
{ "status": "error", "columns": [], "rows": [], "error": "..." }
```

### GET `/api/versions`
**Request:**
`table_name`, optional `session_id` (accepts `null`/empty -> active session)

**Response (success):**
```json
{ "status": "ok", "versions": [{ "version": 1, "timestamp": 1710000000, "metadata": {} }] }
```

### POST `/api/update_style_range`
**Request:**
```json
{
  "table_name": "xxx",
  "session_id": "optional",
  "range": { "start_col": 0, "start_row": 0, "end_col": 2, "end_row": 5 },
  "style": { "bold": true }
}
```

**Response (success):**
```json
{ "status": "ok", "message": "optional", "error": "optional" }
```

### POST `/api/ensure_columns`
**Request:**
```json
{
  "table_name": "xxx",
  "session_id": "optional",
  "columns": [ { "name": "pivot_col_1", "type": "utf8" } ]
}
```

**Response (success):**
```json
{ "status": "ok", "message": "optional", "error": "optional" }
```

### POST `/api/update_merge`
**Request:**
```json
{
  "table_name": "xxx",
  "range": { "start_col": 0, "start_row": 0, "end_col": 3, "end_row": 0 }
}
```

**Response (success):**
```json
{ "status": "ok", "message": "Merged | Unmerged" }
```

## Behavior
- Keep legacy request field aliases (`row`, `col`, `val`) and existing response fields.
- Add `status` to `/api/execute` responses for uniformity.
- Normalize `session_id` values of `null` or empty string to `None`.
- Allow extra response fields (frontend ignores unknown fields).

## Implementation Notes
- Update only HTTP handlers in `federated_query_engine/src/api/*_handler.rs`.
- Keep service-layer return types unchanged where possible.
- Update integration tests to assert new `status` field and error payloads.
- Add change-reason comments with date on modified code lines.

## Testing (TDD)
- Extend `federated_query_engine/tests/api_integration_test.rs`:
  - `/api/execute` returns `status` on success and error.
  - `/api/update_style_range` error response includes `error`.
  - `/api/ensure_columns` error response includes `error`.
  - `/api/batch_update_cells` retains legacy behavior while allowing extra fields.

## Risks
- Inconsistent error fields across handlers if not normalized.
- Frontend could rely on newly added fields; ensure old clients still parse correctly.

## Success Criteria
- Frontend calls succeed with expected fields present.
- Existing integrations remain functional with no breaking changes.
- Tests confirm `status` and error fields where required.

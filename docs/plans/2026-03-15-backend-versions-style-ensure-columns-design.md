# Backend APIs: versions, update_style_range, ensure_columns (main repo)

## Goal
Expose three backend APIs in the main repository and keep the runtime entrypoints aligned so the running binary always serves the new routes.

## Scope
- GET `/api/versions`
- POST `/api/update_style_range`
- POST `/api/ensure_columns`

## Constraints and Rules
- `session_id` is optional:
  - If present, use that session.
  - If absent or the literal string `"null"` (or empty), use the active session.
- `ensure_columns` expands **session-level** schema only.
- Column expansion is **idempotent** and **order-preserving** (append in request order).
- `batch_update_cells` must accept writes to newly expanded columns.

## Architecture
- Add new API handlers in `federated_query_engine/src/api/`:
  - `versions_handler.rs`
  - `ensure_columns_handler.rs`
  - (update style range remains in `update_handler.rs` or a new handler file)
- Register routes in `federated_query_engine/src/lib.rs` (`create_app()`).
- Update `federated_query_engine/src/main.rs` to use `create_app()` so the binary and library share the same router.

## Data Flow
1. HTTP request hits handler.
2. Handler normalizes `session_id` (empty / "null" -> None).
3. Handler calls `SessionManager`:
   - `get_versions` for versions list
   - `update_style_range` for styles
   - `ensure_columns` for session schema expansion
4. Response returns status + data (versions list / columns list / session_id).

## Error Handling
- 400: invalid params (missing table_name, malformed payload, unsupported type).
- 404: table or session not found.
- 500: internal errors (Lance/Arrow/DataFusion failures).

## Testing Strategy
Integration tests in `federated_query_engine/tests/api_integration_test.rs`:
- `/api/versions` with and without `session_id`.
- `/api/update_style_range` applies bold/italic/underline/color/bg_color.
- `/api/ensure_columns` idempotency + follow-up `batch_update_cells` write.

## Rollout and Compatibility
- Keep existing endpoints unchanged.
- Ensure both `lib.rs` and `main.rs` use the same router to avoid 404/405 in production.


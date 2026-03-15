# Backend Main Bootstrap Unification Design

**Date:** 2026-03-15

## Goal
Unify the `federated_query_engine` bootstrap logic so `main.rs` uses the same initialization path as `lib.rs` while preserving **exact current runtime behavior** (data source registration, metadata cleanup, cache maintenance, and routing).

## Non-Goals
- No API response schema changes beyond the already agreed status/error alignment.
- No behavior changes in default dataset registration or metadata cleanup.
- No new features or refactors outside bootstrap unification.

## Current State
- `lib.rs` owns the complete router and `AppState`.
- `main.rs` still has its own bootstrap + a smaller router, causing 405s for new endpoints.
- Startup behavior in `main.rs` includes:
  - Cache maintenance task startup.
  - Metadata store cleanup for missing files.
  - Default dataset registration (CSV/Excel/SQLite introspection).
  - Specific registration refresh paths.

## Proposed Design (Option C)
Move **all** bootstrap logic from `main.rs` into `lib.rs` and expose a single entry that:
1. Initializes the `SessionContext`, `MetadataManager`, and `SessionManager`.
2. Starts cache maintenance and session auto-flush.
3. Performs persisted table recovery + metadata cleanup.
4. Registers default datasets (orders, exchange_rates, users.xlsx, sqlite introspection).
5. Builds the full router with all API routes.
6. Starts the server.

`main.rs` becomes a thin wrapper that calls the `lib` entry.

## Components & Responsibilities
- `lib.rs`
  - `create_app_with_bootstrap()` (or rename existing `create_app`) returns `(Router, AppState)` or just `Router` with initialized state.
  - `run()` uses the unified builder.
- `main.rs`
  - calls `tabula_server::run()` (or the new unified entry), no extra bootstrap.

## Data Flow
1. Determine data paths based on CWD.
2. Build `MetadataManager` and `SessionManager`.
3. Start cache maintenance.
4. Load persisted tables and re-register.
5. Register default datasets if missing.
6. Build router with all handlers.
7. Bind and serve.

## Error Handling
- Preserve existing behavior: log + continue for recoverable failures.
- Fatal errors remain only for initialization of critical services.

## Compatibility
- Routes and behavior should match the previous `main.rs` behavior but now include all endpoints.
- Ensure `/api/execute` keeps `status` field in both success and error response.

## Testing
- Run `cargo test -p federated_query_engine -- --nocapture` (may need longer timeout).
- Run `cargo test -p federated_query_engine --test api_integration_test -- --nocapture`.
- Manual smoke: start backend and hit `/api/ensure_columns`, `/api/update_style_range`, `/api/execute`.

## Risks & Mitigations
- Risk: missing default dataset registration after refactor.
  - Mitigation: preserve the same registration code paths and add a quick manual smoke check.
- Risk: double-registration or inconsistent metadata refresh.
  - Mitigation: keep existing `registered_names` set logic unchanged.

# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Project Overview

This is Tabula, a query engine built with Rust and DataFusion that enables unified SQL queries across multiple data sources (CSV, Excel, SQLite, Oracle, Parquet). The project uses a Rust Workspace structure and includes a React/TypeScript frontend for interactive spreadsheet-like data editing.

## Architecture

### Workspace Structure
- `federated_query_engine/`: Core Axum web server with query logic, caching, and data source implementations
- `metadata_store/`: Shared metadata storage library (local dependency crate)
- `frontend/`: React + TypeScript UI with Glide Data Grid and Univer integration
- `data/`: Source files (Excel/CSV，需自行添加)
- `cache/`: Runtime-generated Parquet cache files

### Core Components

**Backend (federated_query_engine/src/)**
- `main.rs`: Entry point, Axum router, and AppState definition
- `cache_manager.rs`: Multi-level cache implementation (L0/L1/L2) with:
  - Volatility tracking for frequently-updated tables
  - Request coalescing (singleflight pattern)
  - TTI-based eviction (probation vs protected)
  - Metrics registry for performance monitoring
- `session_manager/`: Multi-session support with Lance-backed storage for editing
  - In-memory editing with auto-flush
  - Fork/extend sessions
  - Cell-level updates with style and merge metadata
- `metadata_manager.rs`: Table metadata persistence
- `query_rewriter.rs`: SQL query optimization and rewriting
- `datasources/`: Data source connectors
  - `csv.rs`, `excel.rs`, `parquet.rs`, `sqlite.rs`, `oracle.rs`
  - `sql_dialect.rs`: Custom SQL dialect handling
- `api/`: HTTP handlers (execute, grid, health, plan, register, upload)
- `services/`: Business logic layer

**Frontend (frontend/src/)**
- `App.tsx`: Main application with multi-session table editing
- `components/`: UI components including GlideGrid integration
- React + Vite + TypeScript stack
- Glide Data Grid for high-performance spreadsheet UI
- Univer SDK integration for advanced spreadsheet features
- `frontend/README.md` 提到网格组件候选为 Luckysheet / FortuneSheet（Proposed）

### Cache Architecture

Three-level caching system:
- **L0**: Direct source query (bypassed for volatile tables)
- **L1**: Parquet file cache on disk
- **L2**: In-memory RecordBatch cache with LRU eviction

Volatility detection: Tables with 3+ updates in 10 seconds enter bypass mode for 30 seconds.

## Development Commands

### Build & Run
```bash
# Build the entire workspace
cargo build

# Run the backend server (listens on 0.0.0.0:3000)
cargo run --bin tabula-server

# Run with Oracle support (requires Oracle Instant Client)
cargo run --features oracle

# Build frontend
cd frontend
npm install
npm run dev  # Development server on port 5174
npm run build  # Production build
```

### Testing
```bash
# Run all tests
cargo test

# Run a single test by name
cargo test <test_name>

# Run specific test modules
cargo test cache_stress_test
cargo test cache_e2e_test

# Frontend tests (from frontend/)
npm run dev  # Manual testing via browser
```

### Code Quality
```bash
# Format code (must run before commits)
cargo fmt

# Lint and check for issues
cargo clippy

# Check compilation without building
cargo check
```

### Development Workflow
```powershell
# Kill existing processes and restart both backend and frontend (Windows)
./restart_all.ps1
```

## Common Development Tasks

### Adding a New Data Source
1. Create new file in `federated_query_engine/src/datasources/`
2. Implement the `TableProvider` trait from DataFusion
3. Register the new file type in `main.rs` route handlers
4. Add any new dependencies to `federated_query_engine/Cargo.toml`

### Modifying Cache Logic
- Core logic is in `federated_query_engine/src/cache_manager.rs`
- Key constants at top of file: `VOLATILITY_WINDOW_MS`, `VOLATILITY_THRESHOLD`, `PROTECTED_TTL_MS`
- After changes, run `cargo test cache_stress_test` to verify concurrency safety
- Check metrics via `/metrics` endpoint

### Adding API Endpoints
1. Create handler in `federated_query_engine/src/api/`
2. Create service logic in `federated_query_engine/src/services/`
3. Register route in `main.rs` router
4. Update AppState if new shared state is needed

### Session Management
- Sessions are stored as Lance datasets in `cache/sessions/{table_name}/{session_id}/`
- Default session created automatically on table registration
- Fork operations copy parent session data and metadata
- Auto-flush triggers on: time interval (5s), pending writes (20), or dirty duration (15s)

## Important Implementation Details

### Multi-Session Flow
1. Frontend selects table → calls `/api/sessions?table_name=X`
2. Backend returns list of sessions from SessionManager
3. Frontend selects session → calls `/api/grid-data?session_id=Y`
4. Backend switches active session pointer in DataFusion context
5. Edits are tracked in-memory and flushed periodically to Lance

### Cell Updates
- Updates go through `SessionManager::update_cell()` at `session_manager/mod.rs`
- Supports value updates, style changes, and formula persistence
- Metadata (styles, merges) stored in SheetMetadata structure
- Both data and metadata persisted to Lance on flush

### Query Execution Path
1. Request → API handler (`api/execute_handler.rs`)
2. Cache policy decision (`cache_manager.rs:should_cache()`)
3. If cached → L2 memory check → L1 disk check
4. If miss → Execute via DataFusion → Store in L1/L2
5. Return results to frontend

## File References

Key configuration files:
- `Cargo.toml`: Workspace definition
- `federated_query_engine/Cargo.toml`: Backend dependencies (DataFusion 50.2.0, Arrow 56.2.0, Lance 1.0.4)
- `frontend/package.json`: Frontend dependencies (React 18, Glide Data Grid, Univer)

Documentation:
- `architecture_flow.md`: Sequence diagrams for old vs new session flow
- `SAVE_UPDATE_LOGIC.md`: Cell update and persistence logic
- `EXCEL_UI_IMPLEMENTATION.md`: UI implementation details

## Platform Notes

### Windows Development
- Uses Visual Studio Build Tools for C++ compilation (required for ring, zstd dependencies)
- 使用 rustup 安装 Rust 工具链（stable-x86_64-pc-windows-msvc）
- 如需运行 `generate_report.py`，需安装 Python 3.x
- File paths: Use forward slashes in Bash commands
- Default shell: Git Bash
- Port management: Use `restart_all.ps1` to kill processes on ports 3000 (backend) and 5174 (frontend)

### Debugging
- VS Code recommended with rust-analyzer, CodeLLDB, Even Better TOML extensions
- Press F5 to start debugging (auto-generates launch.json)
- Backend logs include timestamps via `add_log()` in `main.rs`
- Frontend: Browser DevTools, check Network tab for API calls

## Testing Strategy

### Backend Tests
- Unit tests: Inline in source files with `#[cfg(test)]`
- Cache tests: `cache_e2e_test.rs`, `cache_stress_test.rs`
- Focus areas: Concurrency, cache correctness, volatility detection

### Frontend Tests
- Manual testing via development server
- E2E scripts: `verify_*.js` and `verify_*.ps1` in frontend/
- Test coverage: Multi-session switching, cell editing, numeric updates

## Dependencies

Critical dependencies and versions:
- DataFusion 50.2.0 (core query engine)
- Arrow 56.2.0 (columnar data format)
- Lance 1.0.4 (versioned dataset storage)
- Axum 0.8.8 (web framework)
- Tokio 1.40 (async runtime)

Frontend stack:
- React 18.2
- TypeScript 5.2
- Vite 5.2
- Glide Data Grid 6.0.3
- Univer 0.4.2

# Task Plan: Fix Table Name Case Sensitivity Issue

## Goal
Fix the frontend error `Error during planning: table 'datafusion.public.Pi…e_Project_YMP_需求_export_20251210153349' not found` which occurs after uploading a file with mixed-case name, despite backend logs showing successful query execution.

## Analysis
- **Symptom**: Backend executes SQL successfully (with rewritten lowercase name), but frontend reports error with original mixed-case name.
- **Hypothesis**:
    1. Table is registered in DataFusion with a normalized (lowercase) name.
    2. Backend query execution handles this normalization (rewriting SQL).
    3. However, some subsequent operation (maybe `get_metadata` or a second query triggered by frontend) uses the original mixed-case name *without* normalization or quoting, causing DataFusion to fail to find the table.
    4. Or, the frontend sends a request that bypasses the normalization logic.

## Plan
- [ ] **Step 1: Investigate `upload_service.rs`**
    - Check how tables are registered. Are names normalized?
- [ ] **Step 2: Investigate `grid_service.rs` and `grid_handler.rs`**
    - Check `fetch_grid_data` and `prepare_grid_session_metadata`.
    - Look for where `register_table` is called and what name is used.
    - Look for where queries are constructed.
- [ ] **Step 3: Reproduce/Verify**
    - I cannot easily reproduce with a full frontend, but I can inspect the code to confirm the case handling logic.
    - Create a test case if possible.
- [ ] **Step 4: Fix**
    - Ensure consistent table name handling (either always normalize to lowercase, or always quote and preserve case).
    - Likely need to ensure `register_table` uses the same name string as the queries.

## Findings
- Backend log: `Rewritten SQL: SELECT * FROM "pingcode_project_..."` implies there is a SQL rewriter that lowercases table names.
- Backend log: `[GridData] Executing: SELECT * FROM "PingCode_Project_..."` shows the incoming query *before* rewrite?
- Frontend error comes from `GlideGrid`.

## Next Steps
- Search for "Rewritten SQL" to find the rewriting logic.
- Search for "register_table" to see how it's registered.

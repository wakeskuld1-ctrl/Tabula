# E2E Simulation & Verification Plan

## 1. Objective
Simulate user interactions from the frontend to verify the backend logic, focusing on Caching, Execution Plan, Cost Estimation, Logs, Secondary Fetch (Cache Hit), and Cross-Datasource Querying.

## 2. Test Environment
- **Backend**: Rust `federated_query_engine` running on `localhost:3000` (or configured port).
- **Frontend Simulation**: Rust integration test (`tests/e2e_simulation.rs`) using `reqwest` to call API endpoints.

## 3. Scenarios & Verification Points

### Scenario A: YashanDB Connection & Metadata
1. **Action**: POST `/api/connect/yashandb` (or equivalent debug/save endpoint).
2. **Expectation**: Return list of tables.
3. **Log Check**: "Successfully connected with driver...".

### Scenario B: Caching Logic (The Core Loop)
1. **Action**: Register `tpcc.BMSQL_DISTRICT` (or similar small table).
2. **Action**: Execute Query (Scan) `SELECT * FROM tpcc.BMSQL_DISTRICT LIMIT 10`.
3. **Verification**:
   - **First Run**:
     - Response status: "ok".
     - Logs: "Cache Miss: serving ... from remote", "Sidecar: Cached ... successfully" (async).
     - Check `cache/yashandb/` for `.parquet` file existence.
   - **Wait**: Allow sidecar to finish (poll file size or logs).
   - **Second Run**:
     - Execute same query.
     - Logs: "Cache Hit: Serving ... from local parquet".
     - Response time should be significantly lower.

### Scenario C: Execution Plan & Cost
1. **Action**: POST `/api/plan` with SQL `SELECT * FROM tpcc.BMSQL_DISTRICT`.
2. **Verification**:
   - Response JSON contains:
     - `physical_plan_text`: Should show `ParquetExec` (if cached) or `YashanExec` (if not).
     - `cost_est`: Should be > 0 (if stats available).
     - `estimated_rows`: Should match `NUM_ROWS` from stats.

### Scenario D: Cross-Datasource Query (Federated)
1. **Action**: Register `tpcc.BMSQL_WAREHOUSE` (Oracle/YashanDB) and `orders` (CSV).
2. **Action**: Execute Join Query `SELECT * FROM tpcc.BMSQL_WAREHOUSE w JOIN orders o ON w.w_id = o.o_w_id LIMIT 5`.
3. **Verification**:
   - Response contains joined data.
   - Logs show `HashJoinExec` in plan.
   - **Pushdown Check**: If joining two tables from SAME source, check if it pushes down.

### Scenario E: Logs
1. **Action**: GET `/api/logs`.
2. **Verification**: Returns recent logs including "Cache Hit/Miss", "Execution Time".

## 4. Implementation Steps
1. **Create Test Script**: `tests/e2e_simulation.rs`.
2. **Run Test**: Execute against running backend.
3. **Analyze Results**: Identify gaps (e.g., Cost not showing, Cache not hitting).
4. **Fix & Refine**: Modify code based on findings.

## 5. Specific Fixes (Anticipated)
- **Cost Display**: Ensure `PlanResponse` correctly maps DataFusion statistics.
- **Cache Logic**: Verify atomic write prevents partial reads.
- **Frontend API**: Ensure `handleAutoScan` and other frontend wrappers call these APIs correctly (verified via simulation).

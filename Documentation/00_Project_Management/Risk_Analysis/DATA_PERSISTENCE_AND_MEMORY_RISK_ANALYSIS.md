# Architectural Risk Analysis: Data Persistence & Memory Management
**Date:** 2026-02-01
**Status:** Approved for Implementation

## Executive Summary
This document analyzes the architectural risks associated with the current "Direct Mode" (direct creation of Parquet/Lance files) versus the "Managed Mode" (SQLite Metadata + Buffer Manager). It highlights the critical "Split-Brain" risk and "Unbounded Memory" risk that threaten system stability as we introduce `create_table` and cross-sheet capabilities.

## 1. The "Triple Store" Problem (Split-Brain Risk)

### Current Architecture
Currently, the system maintains state in three disconnected locations:
1.  **SQLite (`metadata.db`)**: Registry of known tables.
2.  **JSON (`sessions.json`)**: Registry of user sessions, cell styles, and merge info.
3.  **Lance/Parquet Files**: The actual tabular data on disk.

### The Risk
When a user creates a table or edits a cell, the system must update multiple stores.
*   **Scenario:** User creates a new table "Q1_Sales".
    *   Step 1: Write empty Lance file to disk. (Success)
    *   Step 2: Update `metadata.db` to register "Q1_Sales". (Success)
    *   Step 3: Update `sessions.json` to create a default session. (**Fail** - e.g., disk full, process crash)
*   **Result:** "Orphaned Data". The table exists in SQLite but has no session, or exists on disk but isn't in SQLite. The UI will crash or show empty states.

### Impact on `create_table` API
If we implement `create_table` by just writing a Parquet file, we bypass the session management layer.
*   **Consequence:** The new table is "read-only" effectively because `SessionManager` doesn't know about it.
*   **Fix:** We must use **Atomic Transactions**. The creation of the physical file, the SQLite entry, and the initial Session record must happen as a single logical unit.

## 2. Memory Management Risks (The "RAM Explosion")

### Current Architecture
The `SessionManager` loads the **entire dataset** into a `Vec<RecordBatch>` in RAM when a session is active.
```rust
// session_manager/mod.rs
pub struct SessionInfo {
    // ...
    pub current_data: Option<Arc<RwLock<Vec<RecordBatch>>>>, // All data in memory
}
```

### The Risk
*   **Scenario:** A user opens 3 sheets: "Sales_2024" (500MB), "Logs_Jan" (1GB), and "Users" (200MB).
*   **Result:** The backend process attempts to allocate ~1.7GB + overhead.
*   **Cross-Sheet Formulas:** To calculate `=SUM(Sheet1!A:A) + SUM(Sheet2!A:A)`, *both* sheets must be loaded into memory simultaneously.
*   **OOM Crash:** The Node.js frontend or Rust backend will likely be killed by the OS (OOM Killer).

### Mitigation: LRU Buffer Manager
We cannot keep full datasets in memory. We need a Database-style Buffer Manager:
1.  **Page-Based Loading:** Divide Lance files into "Pages" (Row Groups).
2.  **LRU Cache:** Keep only the most recently used 50-100MB of data in RAM.
3.  **Spilling:** If the user edits a huge range, spill dirty pages to a temporary WAL (Write-Ahead Log) or swap file, not just heap memory.

## 3. Cross-Sheet Dependency Risks

### The "Stale Read" Problem
When Sheet A references Sheet B:
*   If Sheet B is edited by User 2 (or another session), and Sheet A is looking at a cached version of Sheet B, the calculation is wrong.
*   **Risk:** Financial errors due to stale data.

### Architecture Requirement
*   **Single Source of Truth:** All formula calculations must go through a unified data access layer (the proposed `BufferManager`) that ensures it reads the latest committed version of data.
*   **Dependency Graph:** We need to track `Sheet1 -> depends_on -> Sheet2`. When Sheet 2 updates, we must invalidate Sheet 1's cache.

## 4. Recommendation

1.  **Immediate (Done/In-Progress):**
    *   Migrate `sessions.json` to SQLite to unify metadata.
    *   Wrap `create_table` in a Saga/Transaction pattern.

2.  **Short-Term (Next Sprint):**
    *   Implement **LRU Buffer Manager** to replace `Vec<RecordBatch>`.
    *   Do not allow `current_data` to grow unboundedly.

3.  **Long-Term:**
    *   Implement a true dependency graph for cross-sheet reactivity.

## 5. Conclusion
Directly creating Parquet files is simple but dangerous. It leads to data corruption (orphans) and scalability limits (OOM). The move to a SQLite-backed Session Manager and a future Buffer Manager is non-negotiable for a production-grade Excel alternative.

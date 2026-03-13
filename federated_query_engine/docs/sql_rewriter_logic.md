# SQL Rewriter Logic & Architecture

## Overview
The SQL Rewriter is a core component of the Federated Query Engine responsible for translating logical table names (e.g., `tpcc.bmsql_warehouse`) into physical table names (e.g., `oracle_tpcc_bmsql_warehouse`) routed to specific data sources (Oracle, YashanDB, etc.).

## Evolution: From Regex to AST + CBO
Previously, the rewriter used simple string replacement or Regex, which was fragile and prone to errors with complex queries (aliases, subqueries, quoted identifiers).
The new implementation uses **DataFusion's AST (Abstract Syntax Tree)** parser and a **Cost-Based Optimizer (CBO)** approach.

### Key Improvements (Why code was changed)
- **Precision**: AST parsing ensures we only replace table names, not column aliases or string literals.
- **Flexibility**: Handles schema-qualified names (`schema.table`) and unqualified names (`table`) uniformly.
- **Intelligence**: CBO selects the best data source when a table exists in multiple locations (e.g., cached Parquet vs. remote Oracle).

## Core Logic Flow

### 1. Table Extraction
The rewriter parses the SQL and extracts all logical table names (e.g., `tpcc.bmsql_warehouse`, `tpcc.bmsql_history`).

### 2. Candidate Selection (CBO)
For each logical table, it searches the Metadata Manager for matching physical tables.
- **Match Strategy**:
  - Exact match: `sheet_name == simple_name`
  - Scoped suffix match: `table_name` ends with `_{simple_name}`
  - Case-insensitive match.

### 3. Aggressive Inference (New Feature)
If a table (e.g., `tpcc.bmsql_history`) is **NOT found** in metadata (not yet imported), the CBO attempts to **infer** its physical name based on other tables in the same query.
- **Scenario**: `warehouse` is found as `oracle_tpcc_bmsql_warehouse`.
- **Inference**: The engine detects the prefix `oracle_tpcc_` and assumes `history` follows the same pattern -> `oracle_tpcc_bmsql_history`.
- **Benefit**: Enables "Single Source Pushdown" even for tables not explicitly registered, preventing rewrite failures for complex joins.

### 4. Source Selection
The CBO scores potential sources based on:
- **Coverage**: How many query tables does this source contain? (Maximize coverage to allow pushdown).
- **Cost**: Sum of row counts (Minimize data transfer).

### 5. AST Rewriting
The chosen mapping (Logical -> Physical) is applied to the AST.
- **Schema Stripping**: If static routing fails, schema prefixes (`tpcc.`) are stripped as a final fallback to allow local table resolution.

## Current Issue Diagnosis (Terminal#1003-1018)

### Symptom
Query: `... JOIN tpcc.bmsql_history ...`
Result: `history` table is not rewritten or fails execution.

### Root Cause
1. **Successful Rewrite**: The Aggressive Inference correctly identifies `oracle_tpcc_bmsql_history`.
2. **Execution Failure**: DataFusion fails to create a Logical Plan because `oracle_tpcc_bmsql_history` is **not registered** in the catalog.
   - The table exists in Oracle, but was likely never imported/synced in the frontend.
   - DataFusion requires a schema (columns/types) to validate the query, even for pushdown.

### Solution Plan
1. **Dynamic Registration**: Modify `main.rs` to detect inferred tables.
2. **Metadata Fetch**: On detection, automatically fetch metadata (columns) for `oracle_tpcc_bmsql_history` from the source (Oracle) and register it temporarily.
3. **Execution**: Once registered, DataFusion can generate the plan and push the query to Oracle.

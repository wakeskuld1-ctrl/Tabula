# Metadata Sync & Reconciliation Test Plan

## 1. Overview
This document outlines the test strategy for verifying the "Closed Loop" metadata synchronization mechanism, ensuring that the Federated Query Engine correctly handles table additions, deletions, and schema changes from remote data sources.

## 2. Test Cases

| ID | Test Case | Pre-Condition | Action | Expected Result |
| :--- | :--- | :--- | :--- | :--- |
| **TC-01** | **Auto-Discovery (Add)** | Table `T1` exists in Source but not in Metadata. | Trigger Sync Task. | `T1` is registered in DataFusion and persisted to Metadata. |
| **TC-02** | **De-Registration (Delete)** | Table `T1` exists in Metadata but deleted from Source. | Trigger Sync Task. | `T1` is unregistered from DataFusion and removed from Metadata. |
| **TC-03** | **Schema Drift (Update)** | Table `T1` has 10 cols in Metadata, Source changed to 12 cols. | Trigger Sync Task. | `T1` is re-registered with new Schema (12 cols). |
| **TC-04** | **No-Op (Stable)** | Table `T1` is consistent in both. | Trigger Sync Task. | No changes (logs show "Skipped"). |
| **TC-05** | **Multi-Source Isolation** | Source A has `T1`, Source B has `T2`. | Sync Source A. | `T2` (Source B) is unaffected. |

## 3. Python Test Script (Simulation)
The following Python script simulates the lifecycle of a table to verify the API and Metadata logic.

```python
import requests
import json
import time

BASE_URL = "http://localhost:3000/api"

def test_metadata_lifecycle():
    # 1. Setup: Ensure clean state
    print("[1] Cleaning up...")
    requests.delete(f"{BASE_URL}/tables/yashan_user_mock_table")

    # 2. Simulate "Discovery" (Manually Register for Test)
    print("[2] Simulating Discovery (Registering Table)...")
    payload = {
        "catalog": "datafusion",
        "schema": "public",
        "table": "yashan_user_mock_table",
        "source_type": "yashandb",
        "config": json.dumps({"host": "192.168.1.1", "port": 1688}), # Mock Config
        "file_path": "mock" 
    }
    # Note: In real scenario, background task calls register_table internally.
    # Here we verify the API endpoint behaves correctly for manual ops.
    res = requests.post(f"{BASE_URL}/register_table", json=payload)
    assert res.status_code == 200, f"Registration failed: {res.text}"
    
    # Verify Existence
    tables = requests.get(f"{BASE_URL}/tables").json()
    assert any(t['name'] == 'yashan_user_mock_table' for t in tables), "Table not found after register"
    print(" -> Table Registered Successfully")

    # 3. Simulate "Deletion" (Unregister)
    print("[3] Simulating Deletion (Unregistering Table)...")
    res = requests.delete(f"{BASE_URL}/tables/yashan_user_mock_table")
    assert res.status_code == 200, f"Deletion failed: {res.text}"
    
    # Verify Removal
    tables = requests.get(f"{BASE_URL}/tables").json()
    assert not any(t['name'] == 'yashan_user_mock_table' for t in tables), "Table still exists after delete"
    print(" -> Table Deleted Successfully")

if __name__ == "__main__":
    try:
        test_metadata_lifecycle()
        print("\n[PASS] All Metadata Lifecycle Tests Passed")
    except Exception as e:
        print(f"\n[FAIL] Test Failed: {e}")
```

## 4. Rust Integration Test (Pseudocode)
For the actual background task, we can add a test in `src/main.rs` (under `#[cfg(test)]`):

```rust
#[tokio::test]
async fn test_reconciliation_logic() {
    // Mock MetadataManager and Context
    let (ctx, mm) = setup_env();
    
    // 1. Seed Metadata with "Stale Table"
    mm.register_table(..., "yashan_old_table", ...).await.unwrap();
    
    // 2. Mock Source Introspection (Returns empty list)
    let source_tables = vec![]; 
    
    // 3. Run Sync Logic
    sync_tables(&ctx, &mm, &source_tables).await;
    
    // 4. Assert "yashan_old_table" is gone
    assert!(!ctx.table_exist("yashan_old_table").unwrap());
}
```

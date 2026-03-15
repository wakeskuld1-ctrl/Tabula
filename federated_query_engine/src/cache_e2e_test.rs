#[cfg(test)]
mod tests {
    use crate::cache_manager::CacheManager;
    use crate::datasources::sqlite::FetchStrategy;
    use crate::datasources::sqlite::SqliteExec;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::common::collect;
    use datafusion::physical_plan::ExecutionPlan; // Added trait
    use rusqlite::Connection;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    async fn create_test_db(path: &str) {
        let _ = std::fs::remove_file(path);
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS test_table (id INTEGER, val TEXT)",
            [],
        )
        .unwrap();
        // Insert enough data to be noticeable but not huge
        // 1000 rows
        for i in 0..1000 {
            conn.execute(
                "INSERT INTO test_table (id, val) VALUES (?1, ?2)",
                (i, format!("val_{}", i)),
            )
            .unwrap();
        }
    }

    #[allow(dead_code)]
    async fn get_table_mtime(path: &str) -> u64 {
        let metadata = std::fs::metadata(path).unwrap();
        metadata
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[tokio::test]
    async fn test_e2e_cache_lifecycle() {
        // --- Setup ---
        let db_path = "e2e_test.db";
        create_test_db(db_path).await;

        // Reset Cache State
        CacheManager::clear_l1();
        CacheManager::clear_l2();

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("val", DataType::Utf8, true),
        ]));

        let exec = SqliteExec::new(
            db_path.to_string(),
            "test_table".to_string(),
            schema.clone(),
            None,
            1024,
            FetchStrategy::Cursor,
            None,
            datafusion::common::Statistics::new_unknown(&schema),
            None,
        );

        println!("\n=== Phase 1: Cold Start (L0 -> L1 -> L2) ===");
        let start = std::time::Instant::now();
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let batches = collect(stream).await.unwrap();
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(rows, 1000);
        println!("Query 1 finished in {:?}. Rows: {}", start.elapsed(), rows);

        // Wait for Sidecar to populate L1 and L2
        println!("Waiting for Sidecar...");
        sleep(Duration::from_secs(2)).await;

        let status = CacheManager::get_cache_status();
        assert!(
            status
                .iter()
                .any(|s| s.contains("L2 Cache (Memory) Count: 1")),
            "L2 should have 1 entry"
        );
        assert!(
            status
                .iter()
                .any(|s| s.contains("L1 Cache (Disk) Count: 1")),
            "L1 should have 1 entry"
        );
        println!("Cache Status Post-Phase 1:\n{:#?}", status);

        println!("\n=== Phase 2: L2 Hit ===");
        // Should be instant and from memory
        let start = std::time::Instant::now();
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let _ = collect(stream).await.unwrap();
        let elapsed = start.elapsed();
        println!("Query 2 (L2 Hit) finished in {:?}", elapsed);
        // assert!(elapsed.as_millis() < 50, "L2 Hit should be very fast");

        println!("\n=== Phase 3: L2 Eviction & L1 Hit ===");
        // 1. Set Memory Limit to 0 (Force Eviction)
        CacheManager::set_test_memory_limit(Some(1)); // 1 Byte limit

        // 2. Trigger Eviction Logic
        CacheManager::put_l2("dummy_key".to_string(), vec![], 10);

        // Wait for async eviction
        sleep(Duration::from_millis(1000)).await;

        let status = CacheManager::get_cache_status();
        println!("Cache Status Post-Eviction:\n{:#?}", status);

        // Verify L1 Hit
        // Expectation: L2 might be empty or contain only dummy
        let start = std::time::Instant::now();
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let _ = collect(stream).await.unwrap();
        println!("Query 3 (L1 Hit) finished in {:?}", start.elapsed());

        println!("\n=== Phase 4: Consistency (Source Change) ===");
        // 1. Modify Source DB
        {
            let conn = Connection::open(db_path).unwrap();
            conn.execute(
                "INSERT INTO test_table (id, val) VALUES (9999, 'new_row')",
                [],
            )
            .unwrap();
        }
        // Update mtime (sometimes fast IO doesn't update mtime immediately visible to granularity)
        // Wait a bit or touch file? Sqlite update should change mtime.
        sleep(Duration::from_millis(100)).await;

        // 2. Run Query
        let stream = exec
            .execute(0, Arc::new(datafusion::execution::TaskContext::default()))
            .unwrap();
        let batches = collect(stream).await.unwrap();
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(rows, 1001, "Should reflect new data");

        // This should have generated a NEW key.
        let status = CacheManager::get_cache_status();
        println!("Cache Status Post-Update:\n{:#?}", status);
        // Should see a new L1 entry eventually.

        println!("\n=== Phase 5: L1 Eviction (Disk Pressure) ===");
        // 1. Set Disk Usage to 90% used (Total=100, Free=10)
        CacheManager::set_test_disk_usage(Some((100, 10)));

        // 2. Trigger check by adding a new L1 item (we just did in Phase 4)
        // Wait for Sidecar from Phase 4 to finish
        sleep(Duration::from_secs(2)).await;

        // Now Phase 4 sidecar called put_l1 -> check_l1_disk_eviction -> should evict
        let status = CacheManager::get_cache_status();
        println!("Cache Status Post-L1-Eviction:\n{:#?}", status);

        // Check if old files are gone.
        // We expect some eviction.

        // Cleanup
        let _ = std::fs::remove_file(db_path);
        CacheManager::clear_l1();
        CacheManager::clear_l2();
    }
}

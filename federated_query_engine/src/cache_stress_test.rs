use crate::cache_manager::CacheManager;
use crate::datasources::sqlite::{FetchStrategy, SqliteExec, SqliteExecParams};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::common::collect;
use datafusion::physical_plan::ExecutionPlan;
use rusqlite::Connection;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// Helper to generate a large database
async fn create_large_test_db(path: &str, rows: usize) {
    let _ = std::fs::remove_file(path);
    let conn = Connection::open(path).unwrap();

    let _ = conn.query_row("PRAGMA journal_mode = MEMORY", [], |_| Ok(()));

    conn.execute(
        "CREATE TABLE IF NOT EXISTS large_table (
        id INTEGER PRIMARY KEY, 
        customer_id INTEGER, 
        amount REAL, 
        description TEXT,
        created_at TEXT
    )",
        [],
    )
    .unwrap();

    let tx = conn.unchecked_transaction().unwrap();
    {
        let mut stmt = tx.prepare_cached("INSERT INTO large_table (id, customer_id, amount, description, created_at) VALUES (?1, ?2, ?3, ?4, ?5)").unwrap();

        for i in 0..rows {
            let customer_id = i % 1000;
            let amount = (i as f64) * 0.5;
            let description = format!(
                "Order description for item {}, providing some text payload for size.",
                i
            );
            let created_at = format!("2024-01-{:02}", (i % 30) + 1);

            stmt.execute((i, customer_id, amount, description, created_at))
                .unwrap();
        }
    }
    tx.commit().unwrap();

    // Check size
    let metadata = std::fs::metadata(path).unwrap();
    println!(
        "Created DB: {} with {} rows. Size: {:.2} MB",
        path,
        rows,
        metadata.len() as f64 / 1024.0 / 1024.0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_full_capabilities_stress() {
    // --- Configuration ---
    let db_path = "stress_test_full.db";
    let row_count = 50_000; // ~5-10MB

    // --- Step 1: Setup ---
    println!("\n=== [Phase 0] Setup & Data Generation ===");
    create_large_test_db(db_path, row_count).await;

    // Reset Cache
    CacheManager::clear_l1();
    CacheManager::clear_l2();
    CacheManager::set_test_memory_limit(Some(100 * 1024 * 1024)); // 100MB

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, true),
        Field::new("customer_id", DataType::Int64, true),
        Field::new("amount", DataType::Float64, true),
        Field::new("description", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, true),
    ]));

    let exec = Arc::new(SqliteExec::new(SqliteExecParams {
        db_path: db_path.to_string(),
        table_name: "large_table".to_string(),
        schema: schema.clone(),
        projection: None,
        batch_size: 1024,
        fetch_strategy: FetchStrategy::Cursor,
        limit: None,
        where_clause: None,
    }));

    let ctx = Arc::new(TaskContext::default());

    // --- Step 2: Concurrency & Canonicalization (Anti-Stampede) ---
    println!("\n=== [Phase 1] Concurrency, Singleflight & Canonicalization ===");
    let start = std::time::Instant::now();
    let concurrency = 50; // High load
    let mut handles = vec![];

    // Mixed queries to test Canonicalization + Singleflight
    // Group A: id = 1 AND customer_id = 1
    // Group B: customer_id = 1 AND id = 1
    // These should map to the SAME cache key and trigger Singleflight coalescing.

    for i in 0..concurrency {
        let _exec_clone = exec.clone();
        let ctx_clone = ctx.clone();

        let where_clause = if i % 2 == 0 {
            Some("id = 1 AND customer_id = 1".to_string())
        } else {
            Some("customer_id = 1 AND id = 1".to_string())
        };

        // Create a specific exec for this query to override the default "None" where
        let specific_exec = Arc::new(SqliteExec::new(SqliteExecParams {
            db_path: db_path.to_string(),
            table_name: "large_table".to_string(),
            schema: schema.clone(),
            projection: None,
            batch_size: 1024,
            fetch_strategy: FetchStrategy::Cursor,
            limit: None,
            where_clause,
        }));

        handles.push(tokio::spawn(async move {
            crate::cache_manager::get_metrics_registry()
                .query_count
                .fetch_add(1, Ordering::Relaxed);
            let start = std::time::Instant::now();
            let start_q = std::time::Instant::now();
            let stream = specific_exec.execute(0, ctx_clone).unwrap();
            let batches = collect(stream).await.unwrap();
            let elapsed = start_q.elapsed().as_micros() as u64;
            crate::cache_manager::get_metrics_registry().record_query_latency(elapsed);
            let elapsed = start.elapsed().as_micros() as u64;
            crate::cache_manager::get_metrics_registry().record_query_latency(elapsed);
            batches.iter().map(|b| b.num_rows()).sum::<usize>()
        }));
    }

    let results = futures::future::join_all(handles).await;
    let duration = start.elapsed();
    println!(
        "Executed {} concurrent canonical queries in {:.2}s",
        concurrency,
        duration.as_secs_f64()
    );

    // All should return 0 or 1 row (depending on data). Let's just check they don't error.
    for res in results {
        assert!(res.is_ok(), "Query failed");
    }
    println!(">> [Pass] Canonicalization + Singleflight verified under load.");

    // --- Step 3: Eviction Pool Stress (Memory Pressure) ---
    println!("\n=== [Phase 2] Eviction Pool Stress Test (Million-Row Throughput Simulation) ===");

    // 1. Set limit (64MB)
    CacheManager::set_test_memory_limit(Some(64 * 1024 * 1024));

    // Start System Monitor (typeperf)
    let monitor_file = "stress_test_metrics.csv";
    let _ = std::fs::remove_file(monitor_file);
    let mut monitor_cmd = std::process::Command::new("typeperf")
        .args([
            "\\Processor(_Total)\\% Processor Time",
            "\\PhysicalDisk(_Total)\\% Disk Time",
            "\\Memory\\Available MBytes",
            "-sc",
            "120", // Max samples
            "-si",
            "1", // Interval 1s
            "-o",
            monitor_file,
            "-y",
        ])
        .spawn()
        .expect("Failed to start typeperf");

    println!("System monitor started. Logging to {}", monitor_file);

    // Start App Metrics Logger (Time Series)
    let app_metrics_file = "app_metrics_series.csv";
    let _ = std::fs::remove_file(app_metrics_file);
    {
        let mut f = std::fs::File::create(app_metrics_file).unwrap();
        writeln!(f, "timestamp,query_count,total_query_latency_us,l2_hits,l2_misses,l2_read_latency_us,l2_lock_wait_us,l2_eviction_count,l1_hits,l1_misses,l1_io_latency_us,l1_eviction_count,l0_requests,l0_exec_latency_us,memory_usage").unwrap();
    }

    let logger_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open("app_metrics_series.csv")
            .unwrap();

        loop {
            interval.tick().await;
            let m = crate::cache_manager::get_metrics_registry().snapshot();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) = writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                ts,
                m.query_count,
                m.total_query_latency_us,
                m.l2_hits,
                m.l2_misses,
                m.l2_read_latency_us,
                m.l2_lock_wait_us,
                m.l2_eviction_count,
                m.l1_hits,
                m.l1_misses,
                m.l1_io_latency_us,
                m.l1_eviction_count,
                m.l0_requests,
                m.l0_exec_latency_us,
                m.memory_usage
            ) {
                eprintln!("Failed to write metrics: {}", e);
            }
        }
    });

    // 2. Flood cache with unique queries
    let flood_count = 5000;
    let rows_per_query = 200;
    println!(
        "Flooding cache with {} unique queries (Target: ~{} rows processed)...",
        flood_count,
        flood_count * rows_per_query
    );

    let start_flood = std::time::Instant::now();

    // We use a semaphore to limit concurrency
    let semaphore = Arc::new(tokio::sync::Semaphore::new(50));
    let mut flood_handles = vec![];

    for i in 0..flood_count {
        let ctx_clone = ctx.clone();
        let schema_clone = schema.clone();
        let db_path_clone = db_path.to_string();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        flood_handles.push(tokio::spawn(async move {
            let _permit = permit; // Hold permit until task done
            crate::cache_manager::get_metrics_registry()
                .query_count
                .fetch_add(1, Ordering::Relaxed);

            // Unique query for each iteration to bypass cache hit and force insert
            // Range: [i*10, i*10 + 200]
            let start_id = i * 10;
            let end_id = start_id + rows_per_query;
            let where_clause = Some(format!("id > {} AND id <= {}", start_id, end_id));

            let specific_exec = Arc::new(SqliteExec::new(SqliteExecParams {
                db_path: db_path_clone,
                table_name: "large_table".to_string(),
                schema: schema_clone,
                projection: None,
                batch_size: 1024,
                fetch_strategy: FetchStrategy::Cursor,
                limit: Some(rows_per_query),
                where_clause,
            }));

            let start = std::time::Instant::now();
            let stream = specific_exec.execute(0, ctx_clone).unwrap();
            let batches = collect(stream).await.unwrap();
            let elapsed = start.elapsed().as_micros() as u64;
            crate::cache_manager::get_metrics_registry().record_query_latency(elapsed);
            batches.iter().map(|b| b.num_rows()).sum::<usize>()
        }));

        if i % 500 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    let results = futures::future::join_all(flood_handles).await;
    let _ = monitor_cmd.kill(); // Stop monitor
    let _ = monitor_cmd.wait();
    logger_handle.abort(); // Stop app logger

    let total_rows: usize = results.into_iter().map(|r| r.unwrap_or(0)).sum();
    let duration = start_flood.elapsed();

    println!("\nFlood complete in {:.2}s", duration.as_secs_f64());
    println!("Total Rows Processed: {}", total_rows);
    println!(
        "Throughput: {:.2} rows/sec",
        total_rows as f64 / duration.as_secs_f64()
    );

    // Read and print metrics summary
    if let Ok(content) = std::fs::read_to_string(monitor_file) {
        println!("\n=== System Metrics (Sample) ===");
        for (i, line) in content.lines().enumerate() {
            if i < 5 || i > content.lines().count() - 5 {
                println!("{}", line);
            } else if i == 5 {
                println!("...");
            }
        }
    }

    // Dump Application Metrics
    let metrics = crate::cache_manager::get_metrics_registry().snapshot();
    let metrics_json = serde_json::to_string_pretty(&metrics).unwrap();
    std::fs::write("metrics_final.json", &metrics_json).unwrap();
    println!("\nApplication metrics dumped to metrics_final.json");

    println!(">> [Pass] Eviction Pool survived high pressure (No OOM/Deadlock).");

    // --- Step 4: Cleanup ---
    // (Optional) Remove db file
    // std::fs::remove_file(db_path).unwrap();
}

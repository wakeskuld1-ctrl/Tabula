#[cfg(test)]
mod cache_e2e_test;
mod cache_manager;
#[cfg(test)]
mod cache_stress_test;
mod datasources;
mod metadata_manager;
mod query_rewriter;

use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Json, Router,
};
use cache_manager::CacheManager;
use datafusion::prelude::*;
use datasources::{
    csv::CsvDataSource, excel::ExcelDataSource, parquet::ParquetDataSource,
    sqlite::SqliteDataSource, DataSource,
};
use metadata_manager::MetadataManager;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

struct AppState {
    ctx: SessionContext,
    metadata_manager: Arc<MetadataManager>,
    logs: Arc<RwLock<VecDeque<String>>>,
}

// Helper to add log
fn add_log(logs: &Arc<RwLock<VecDeque<String>>>, msg: String) {
    let mut guard = logs.write().unwrap();
    // Add timestamp
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    guard.push_back(format!("[{}] {}", timestamp, msg));
    if guard.len() > 100 {
        guard.pop_front();
    }
    // Also print to stdout
    println!("{}", msg);
}

// Helper for API responses
async fn list_tables(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // List from Metadata Manager to show rich metadata
    if let Ok(tables) = state.metadata_manager.list_tables() {
        let json_tables: Vec<serde_json::Value> = tables
            .iter()
            .map(|t| {
                serde_json::json!({
                    "table_name": t.table_name,
                    "file_path": t.file_path,
                    "source_type": t.source_type,
                    "sheet_name": t.sheet_name,
                    "schema_json": t.schema_json,
                    "indexes_json": t.indexes_json
                })
            })
            .collect();
        Json(serde_json::json!({ "status": "ok", "tables": json_tables }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "Failed to list tables" }))
    }
}

async fn get_logs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let logs = state.logs.read().unwrap();
    let log_list: Vec<String> = logs.iter().cloned().collect();
    Json(serde_json::json!({ "status": "ok", "logs": log_list }))
}

#[tokio::main]
async fn main() {
    // Initialize DataFusion Context
    let ctx = SessionContext::new();

    // Determine paths based on CWD
    let (data_path_str, public_path_str, db_path_str) =
        if std::path::Path::new("federated_query_engine").exists() {
            // Running from workspace root
            (
                "federated_query_engine/data",
                "federated_query_engine/public",
                "federated_query_engine/metadata.db",
            )
        } else {
            // Running from crate root
            ("data", "public", "metadata.db")
        };

    // Initialize Metadata Manager
    let metadata_manager =
        MetadataManager::new(db_path_str).expect("Failed to initialize metadata manager");
    let metadata_manager = Arc::new(metadata_manager);

    // Register Data Sources from Metadata Store
    let data_dir = std::path::Path::new(data_path_str);
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).unwrap();
    }

    // Start Cache Maintenance Task (TTI Eviction)
    CacheManager::start_maintenance_task();

    // Load persisted tables and clean up invalid metadata
    let mut registered_names = std::collections::HashSet::new();
    if let Ok(tables) = metadata_manager.list_tables() {
        for table in tables {
            // Consistency Check: Ensure file exists
            if !std::path::Path::new(&table.file_path).exists() {
                println!(
                    "Warning: File for table '{}' not found at '{}'. Cleaning up metadata.",
                    table.table_name, table.file_path
                );
                if let Err(e) = metadata_manager.unregister_table(
                    &table.catalog_name,
                    &table.schema_name,
                    &table.table_name,
                ) {
                    eprintln!(
                        "Failed to unregister invalid table '{}': {}",
                        table.table_name, e
                    );
                }
                continue;
            }

            // Currently assuming all tables are in default catalog/schema for DataFusion registration
            // TODO: Support catalog/schema in DataSource trait

            let source: Option<Box<dyn DataSource>> = if table.source_type == "csv" {
                match CacheManager::ensure_parquet_cache(&table.file_path, "csv", None).await {
                    Ok(p) => Some(Box::new(ParquetDataSource::new(
                        table.table_name.clone(),
                        p,
                    ))),
                    Err(e) => {
                        eprintln!(
                            "Transcoding failed for {}, using original CSV: {}",
                            table.table_name, e
                        );
                        Some(Box::new(CsvDataSource::new(
                            table.table_name.clone(),
                            table.file_path.clone(),
                        )))
                    }
                }
            } else if table.source_type == "excel" {
                // Now we should have the correct sheet name from metadata
                let sheet = table
                    .sheet_name
                    .clone()
                    .unwrap_or_else(|| "Sheet1".to_string());
                match CacheManager::ensure_parquet_cache(
                    &table.file_path,
                    "excel",
                    Some(sheet.clone()),
                )
                .await
                {
                    Ok(p) => Some(Box::new(ParquetDataSource::new(
                        table.table_name.clone(),
                        p,
                    ))),
                    Err(e) => {
                        eprintln!(
                            "Transcoding failed for {}, using original Excel: {}",
                            table.table_name, e
                        );
                        Some(Box::new(ExcelDataSource::new(
                            table.table_name.clone(),
                            table.file_path.clone(),
                            sheet,
                        )))
                    }
                }
            } else if table.source_type == "parquet" {
                Some(Box::new(ParquetDataSource::new(
                    table.table_name.clone(),
                    table.file_path.clone(),
                )))
            } else if table.source_type == "sqlite" {
                // Assuming format: catalog.schema.table for internal metadata, but DataSource needs specific args
                // For sqlite, file_path is the db path
                // Use sheet_name as internal table name if present, otherwise fallback to table_name
                let internal_name = table.sheet_name.clone().unwrap_or(table.table_name.clone());
                Some(Box::new(SqliteDataSource::new(
                    table.table_name.clone(),
                    table.file_path.clone(),
                    internal_name,
                )))
            } else {
                None
            };

            if let Some(source) = source {
                if source.register(&ctx).await.is_ok() {
                    println!("Registered persisted data source: {}", source.name());
                    // Force refresh metadata (capture schema/indexes)
                    if let Err(e) = metadata_manager
                        .register_table(
                            &ctx,
                            &table.catalog_name,
                            &table.schema_name,
                            &table.table_name,
                            &table.file_path,
                            &table.source_type,
                            table.sheet_name.clone(),
                        )
                        .await
                    {
                        eprintln!("Failed to refresh metadata for {}: {}", table.table_name, e);
                    }
                    registered_names.insert(source.name().to_string());
                } else {
                    eprintln!("Failed to register persisted table: {}", table.table_name);
                }
            }
        }
    }

    // Default Data Sources (if not already registered)

    // 1. Orders CSV
    if !registered_names.contains("orders") {
        let orders_path = data_dir.join("orders.csv").to_str().unwrap().to_string();
        if std::path::Path::new(&orders_path).exists() {
            let source = CsvDataSource::new("orders".to_string(), orders_path.clone());
            if source.register(&ctx).await.is_ok() {
                println!("Registered data source: {}", source.name());
                let _ = metadata_manager
                    .register_table(
                        &ctx,
                        "datafusion",
                        "public",
                        "orders",
                        &orders_path,
                        "csv",
                        None,
                    )
                    .await;
            }
        }
    }

    // 2. Exchange Rates CSV
    if !registered_names.contains("exchange_rates") {
        let rates_path = data_dir
            .join("exchange_rates.csv")
            .to_str()
            .unwrap()
            .to_string();
        if std::path::Path::new(&rates_path).exists() {
            let source = CsvDataSource::new("exchange_rates".to_string(), rates_path.clone());
            if source.register(&ctx).await.is_ok() {
                println!("Registered data source: {}", source.name());
                let _ = metadata_manager
                    .register_table(
                        &ctx,
                        "datafusion",
                        "public",
                        "exchange_rates",
                        &rates_path,
                        "csv",
                        None,
                    )
                    .await;
            }
        }
    }

    // 3. Users Excel
    let excel_path = data_dir.join("users.xlsx");
    if !excel_path.exists() {
        generate_test_excel(&excel_path);
    }

    if excel_path.exists() && !registered_names.contains("users") {
        let path_str = excel_path.to_str().unwrap().to_string();
        let source =
            ExcelDataSource::new("users".to_string(), path_str.clone(), "Sheet1".to_string());
        if source.register(&ctx).await.is_ok() {
            println!("Registered data source: {}", source.name());
            let _ = metadata_manager
                .register_table(
                    &ctx,
                    "datafusion",
                    "public",
                    "users",
                    &path_str,
                    "excel",
                    Some("Sheet1".to_string()),
                )
                .await;
        }
    }

    // 4. SQLite Metadata Store (Self-Introspection)
    // Register metadata.db itself as a queryable source
    let meta_db_path = db_path_str;
    if std::path::Path::new(meta_db_path).exists() {
        if let Ok(tables) = SqliteDataSource::list_tables(meta_db_path) {
            for table_name in tables {
                let register_name = if table_name == "tables_metadata" {
                    "sys_metadata".to_string()
                } else {
                    table_name.clone()
                };

                // Avoid double registration if already persisted
                if !registered_names.contains(&register_name) {
                    let source = SqliteDataSource::new(
                        register_name.clone(),
                        meta_db_path.to_string(),
                        table_name.clone(),
                    );
                    if source.register(&ctx).await.is_ok() {
                        println!(
                            "Registered SQLite source: {} -> {}",
                            register_name, table_name
                        );
                        let _ = metadata_manager
                            .register_table(
                                &ctx,
                                "datafusion",
                                "public",
                                &register_name,
                                meta_db_path,
                                "sqlite",
                                Some(table_name),
                            )
                            .await;
                    }
                }
            }
        }
    }

    // Shared State
    let logs = Arc::new(RwLock::new(VecDeque::new()));
    let state = Arc::new(AppState {
        ctx,
        metadata_manager,
        logs,
    });

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build Router
    let app = Router::new()
        .route("/api/upload", post(upload_file))
        .route("/api/register_table", post(register_table_endpoint))
        .route("/api/execute", post(execute_sql))
        .route("/api/tables", get(list_tables))
        .route("/api/logs", get(get_logs))
        .route("/api/connect_sqlite", post(connect_sqlite))
        .route("/api/plan", post(get_plan))
        .route("/api/metrics", get(get_metrics))
        .route("/api/health", get(health))
        .fallback_service(ServeDir::new(public_path_str).append_index_html_on_directories(true))
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit
        .with_state(state);

    // Run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    println!("Backend server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Basic handler
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": "0.1.0" }))
}

#[derive(Deserialize)]
struct ConnectSqliteRequest {
    path: String,
}

async fn connect_sqlite(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ConnectSqliteRequest>,
) -> Json<serde_json::Value> {
    let path = payload.path;
    match SqliteDataSource::list_tables(&path) {
        Ok(tables) => {
            let mut registered = Vec::new();
            for table_name in tables {
                // Register each table
                let source =
                    SqliteDataSource::new(table_name.clone(), path.clone(), table_name.clone());
                if source.register(&state.ctx).await.is_ok() {
                    // Persist metadata
                    let _ = state
                        .metadata_manager
                        .register_table(
                            &state.ctx,
                            "datafusion",
                            "public",
                            &table_name,
                            &path,
                            "sqlite",
                            None,
                        )
                        .await;
                    registered.push(table_name);
                }
            }

            if registered.is_empty() {
                Json(
                    serde_json::json!({ "status": "warning", "message": "Connected but no tables found or registered", "tables": [] }),
                )
            } else {
                Json(serde_json::json!({ "status": "ok", "tables": registered }))
            }
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

// Request/Response models for Plan
#[derive(Deserialize)]
struct PlanRequest {
    sql: String,
    #[allow(dead_code)]
    #[serde(default)]
    dry_run: bool,
    #[allow(dead_code)]
    #[serde(default)]
    runtime_filter: bool,
}

#[derive(Serialize)]
struct PlanResponse {
    plan_json: serde_json::Value, // The tree structure for frontend
    physical_plan_text: String,   // Text representation
    cost_est: f64,
    estimated_rows: Option<usize>,
    estimated_bytes: Option<usize>,
    warnings: Vec<String>,
}

#[derive(Deserialize)]
struct ExecuteRequest {
    sql: String,
}

#[derive(Serialize)]
struct ExecuteResponse {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    execution_time_ms: u64,
    error: Option<String>,
}

async fn execute_sql(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    let sql = payload.sql;
    let start = std::time::Instant::now();

    add_log(&state.logs, format!("Executing SQL: {}", sql));

    // Rewrite Query
    let final_sql = match query_rewriter::rewrite_query(&state.ctx, &sql).await {
        Ok(rewritten) => {
            if rewritten != sql {
                add_log(&state.logs, format!("Rewritten SQL: {}", rewritten));
            }
            rewritten
        }
        Err(e) => {
            add_log(&state.logs, format!("Rewrite Error: {}", e));
            sql.clone()
        }
    };

    // Create DataFrame
    let df_result = state.ctx.sql(&final_sql).await;

    // Self-Healing Logic (Retry if "No field named" error)
    let df_result = match df_result {
        Ok(df) => Ok(df),
        Err(e) => {
            let err_msg = e.to_string();
            // Check for specific error patterns
            // Error: SchemaError(FieldNotFound { field: Box("users"), valid_fields: [...] })
            // or "No field named 'users'"
            if err_msg.contains("No field named") {
                let mut unknown_field = None;
                // Try double quotes
                if let Some(start) = err_msg.find("No field named \"") {
                    let rest = &err_msg[start + 16..];
                    if let Some(end) = rest.find('"') {
                        unknown_field = Some(rest[..end].to_string());
                    }
                }
                // Try single quotes if not found
                if unknown_field.is_none() {
                    if let Some(start) = err_msg.find("No field named '") {
                        let rest = &err_msg[start + 16..];
                        if let Some(end) = rest.find('\'') {
                            unknown_field = Some(rest[..end].to_string());
                        }
                    }
                }

                if let Some(field) = unknown_field {
                    add_log(
                        &state.logs,
                        format!("Detected missing field '{}', attempting fix...", field),
                    );
                    match query_rewriter::fix_query(&state.ctx, &final_sql, &field).await {
                        Ok(fixed_sql) => {
                            add_log(
                                &state.logs,
                                format!("Retrying with fixed SQL: {}", fixed_sql),
                            );
                            state.ctx.sql(&fixed_sql).await
                        }
                        Err(_) => Err(e), // Return original error if fix fails
                    }
                } else {
                    Err(e)
                }
            } else {
                Err(e)
            }
        }
    };

    match df_result {
        Ok(df) => {
            // Collect
            match df.collect().await {
                Ok(batches) => {
                    let duration = start.elapsed();

                    if batches.is_empty() {
                        return Json(ExecuteResponse {
                            columns: vec![],
                            rows: vec![],
                            execution_time_ms: duration.as_millis() as u64,
                            error: None,
                        });
                    }

                    // Format Output
                    let schema = batches[0].schema();
                    let columns: Vec<String> =
                        schema.fields().iter().map(|f| f.name().clone()).collect();

                    let mut rows = Vec::new();
                    for batch in batches {
                        let num_rows = batch.num_rows();
                        let num_cols = batch.num_columns();
                        for i in 0..num_rows {
                            let mut row_vec = Vec::new();
                            for j in 0..num_cols {
                                let col = batch.column(j);
                                // Simple string conversion
                                let val_str = arrow::util::display::array_value_to_string(col, i)
                                    .unwrap_or("".to_string());
                                row_vec.push(val_str);
                            }
                            rows.push(row_vec);
                        }
                    }

                    add_log(
                        &state.logs,
                        format!(
                            "Query executed successfully. {} rows returned in {}ms",
                            rows.len(),
                            duration.as_millis()
                        ),
                    );

                    Json(ExecuteResponse {
                        columns,
                        rows,
                        execution_time_ms: duration.as_millis() as u64,
                        error: None,
                    })
                }
                Err(e) => {
                    add_log(&state.logs, format!("Execution Error: {}", e));
                    Json(ExecuteResponse {
                        columns: vec![],
                        rows: vec![],
                        execution_time_ms: start.elapsed().as_millis() as u64,
                        error: Some(e.to_string()),
                    })
                }
            }
        }
        Err(e) => {
            add_log(&state.logs, format!("Planning Error: {}", e));
            Json(ExecuteResponse {
                columns: vec![],
                rows: vec![],
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
            })
        }
    }
}

async fn upload_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<serde_json::Value> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        let data_dir = if std::path::Path::new("federated_query_engine/data").exists() {
            std::path::Path::new("federated_query_engine/data")
        } else {
            std::path::Path::new("data")
        };

        let path = data_dir.join(&file_name);

        // Write file
        std::fs::write(&path, data).unwrap();
        let path_str = path.to_str().unwrap().to_string();

        add_log(&state.logs, format!("File uploaded: {}", file_name));

        // Generate Table Name
        // Remove extension
        let raw_name = std::path::Path::new(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        // Sanitize table name: replace . - ( ) space with _
        let table_name = raw_name
            .replace('.', "_")
            .replace('-', "_")
            .replace(' ', "_")
            .replace('(', "_")
            .replace(')', "_");

        let extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check for multiple sheets if Excel
        let mut single_sheet_name = None;

        if extension == "xlsx" {
            // We need to peek at the sheets
            // This requires ExcelDataSource to expose a helper or we use it here temporarily
            // Assuming we can use existing get_sheet_names helper from datasources::excel
            if let Ok(sheets) = ExcelDataSource::get_sheet_names(&path_str) {
                if sheets.len() > 1 {
                    return Json(serde_json::json!({
                        "status": "select_sheet",
                        "message": "Multiple sheets detected. Please select one.",
                        "file_path": path_str,
                        "sanitized_name": table_name,
                        "sheets": sheets
                    }));
                } else if sheets.len() == 1 {
                    single_sheet_name = Some(sheets[0].clone());
                } else {
                    return Json(
                        serde_json::json!({ "status": "error", "message": "Excel file contains no sheets" }),
                    );
                }
            }
        }

        let (source, final_path, final_source_type): (Option<Box<dyn DataSource>>, String, String) =
            if extension == "csv" {
                match CacheManager::ensure_parquet_cache(&path_str, "csv", None).await {
                    Ok(p) => (
                        Some(Box::new(ParquetDataSource::new(
                            table_name.clone(),
                            p.clone(),
                        ))),
                        p,
                        "parquet".to_string(),
                    ),
                    Err(e) => {
                        eprintln!(
                            "Transcoding failed for {}, falling back to CSV: {}",
                            file_name, e
                        );
                        (
                            Some(Box::new(CsvDataSource::new(
                                table_name.clone(),
                                path_str.clone(),
                            ))),
                            path_str.clone(),
                            "csv".to_string(),
                        )
                    }
                }
            } else if extension == "xlsx" {
                let sheet = single_sheet_name
                    .clone()
                    .unwrap_or_else(|| "Sheet1".to_string());
                match CacheManager::ensure_parquet_cache(&path_str, "excel", Some(sheet.clone()))
                    .await
                {
                    Ok(p) => (
                        Some(Box::new(ParquetDataSource::new(
                            table_name.clone(),
                            p.clone(),
                        ))),
                        p,
                        "parquet".to_string(),
                    ),
                    Err(e) => {
                        eprintln!(
                            "Transcoding failed for {}, falling back to Excel: {}",
                            file_name, e
                        );
                        (
                            Some(Box::new(ExcelDataSource::new(
                                table_name.clone(),
                                path_str.clone(),
                                sheet,
                            ))),
                            path_str.clone(),
                            "excel".to_string(),
                        )
                    }
                }
            } else {
                (None, path_str.clone(), extension.clone())
            };

        if let Some(source) = source {
            if source.register(&state.ctx).await.is_ok() {
                println!("Auto-registered uploaded data source: {}", source.name());

                let sheet_name = if extension == "xlsx" {
                    Some(if let Some(s) = &single_sheet_name {
                        s.clone()
                    } else {
                        "Sheet1".to_string()
                    })
                } else {
                    None
                };

                if let Err(e) = state
                    .metadata_manager
                    .register_table(
                        &state.ctx,
                        "datafusion",
                        "public",
                        &table_name,
                        &final_path,
                        &final_source_type,
                        sheet_name,
                    )
                    .await
                {
                    eprintln!("Failed to persist metadata for table {}: {}", table_name, e);
                } else {
                    println!("Persisted metadata for table: {}", table_name);
                }

                return Json(serde_json::json!({
                    "status": "ok",
                    "message": format!("Uploaded and optimized {} as table '{}' (Format: {})", file_name, table_name, final_source_type),
                    "table": table_name
                }));
            } else {
                return Json(
                    serde_json::json!({ "status": "error", "message": format!("Uploaded {} but failed to register. Ensure sheet exists.", file_name) }),
                );
            }
        }

        return Json(
            serde_json::json!({ "status": "ok", "message": format!("Uploaded {} (not registered, unsupported type)", file_name) }),
        );
    }
    Json(serde_json::json!({ "status": "error", "message": "No file" }))
}

#[derive(Deserialize)]
struct RegisterTableRequest {
    file_path: String,
    table_name: String,
    sheet_name: Option<String>,
    source_type: String,
}

async fn register_table_endpoint(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterTableRequest>,
) -> Json<serde_json::Value> {
    let source: Option<Box<dyn DataSource>> = if payload.source_type == "excel" {
        let sheet = payload
            .sheet_name
            .clone()
            .unwrap_or_else(|| "Sheet1".to_string());
        match CacheManager::ensure_parquet_cache(&payload.file_path, "excel", Some(sheet.clone()))
            .await
        {
            Ok(p) => Some(Box::new(ParquetDataSource::new(
                payload.table_name.clone(),
                p,
            ))),
            Err(e) => {
                eprintln!(
                    "Transcoding failed for {}, using original Excel: {}",
                    payload.table_name, e
                );
                Some(Box::new(ExcelDataSource::new(
                    payload.table_name.clone(),
                    payload.file_path.clone(),
                    sheet,
                )))
            }
        }
    } else if payload.source_type == "csv" {
        match CacheManager::ensure_parquet_cache(&payload.file_path, "csv", None).await {
            Ok(p) => Some(Box::new(ParquetDataSource::new(
                payload.table_name.clone(),
                p,
            ))),
            Err(e) => {
                eprintln!(
                    "Transcoding failed for {}, using original CSV: {}",
                    payload.table_name, e
                );
                Some(Box::new(CsvDataSource::new(
                    payload.table_name.clone(),
                    payload.file_path.clone(),
                )))
            }
        }
    } else if payload.source_type == "sqlite" {
        let internal_name = payload
            .sheet_name
            .clone()
            .unwrap_or(payload.table_name.clone());
        Some(Box::new(SqliteDataSource::new(
            payload.table_name.clone(),
            payload.file_path.clone(),
            internal_name,
        )))
    } else {
        None
    };

    if let Some(source) = source {
        if source.register(&state.ctx).await.is_ok() {
            add_log(
                &state.logs,
                format!("Manually registered data source: {}", source.name()),
            );

            if let Err(e) = state
                .metadata_manager
                .register_table(
                    &state.ctx,
                    "datafusion",
                    "public",
                    &payload.table_name,
                    &payload.file_path,
                    &payload.source_type,
                    payload.sheet_name,
                )
                .await
            {
                eprintln!(
                    "Failed to persist metadata for table {}: {}",
                    payload.table_name, e
                );
            }

            return Json(
                serde_json::json!({ "status": "ok", "message": "Registered successfully" }),
            );
        }
    }

    Json(serde_json::json!({ "status": "error", "message": "Registration failed" }))
}

// Handler for generating execution plan
async fn get_plan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PlanRequest>,
) -> Json<PlanResponse> {
    let sql = payload.sql;

    // Attempt to rewrite query (same logic as execute_sql)
    let final_sql = match query_rewriter::rewrite_query(&state.ctx, &sql).await {
        Ok(rewritten) => rewritten,
        Err(_) => sql.clone(),
    };

    // 1. Create Logical Plan
    let mut plan_result = state.ctx.state().create_logical_plan(&final_sql).await;

    // Self-Healing for Logical Plan (same logic as execute_sql)
    if let Err(e) = &plan_result {
        let err_msg = e.to_string();
        if err_msg.contains("No field named") {
            let mut unknown_field = None;
            // Try double quotes
            if let Some(start) = err_msg.find("No field named \"") {
                let rest = &err_msg[start + 16..];
                if let Some(end) = rest.find('"') {
                    unknown_field = Some(rest[..end].to_string());
                }
            }
            // Try single quotes if not found
            if unknown_field.is_none() {
                if let Some(start) = err_msg.find("No field named '") {
                    let rest = &err_msg[start + 16..];
                    if let Some(end) = rest.find('\'') {
                        unknown_field = Some(rest[..end].to_string());
                    }
                }
            }

            if let Some(field) = unknown_field {
                if let Ok(fixed_sql) =
                    query_rewriter::fix_query(&state.ctx, &final_sql, &field).await
                {
                    plan_result = state.ctx.state().create_logical_plan(&fixed_sql).await;
                }
            }
        }
    }

    match plan_result {
        Ok(logical_plan) => {
            // 2. Create Physical Plan
            let physical_plan_result = state.ctx.state().create_physical_plan(&logical_plan).await;

            match physical_plan_result {
                Ok(physical_plan) => {
                    // 3. Format plan for display
                    let plan_text = datafusion::physical_plan::displayable(physical_plan.as_ref())
                        .indent(true)
                        .to_string();

                    // Extract Statistics
                    let mut estimated_rows = None;
                    let mut estimated_bytes = None;

                    if let Ok(stats) = physical_plan.statistics() {
                        match stats.num_rows {
                            datafusion::common::stats::Precision::Exact(n) => {
                                estimated_rows = Some(n)
                            }
                            datafusion::common::stats::Precision::Inexact(n) => {
                                estimated_rows = Some(n)
                            }
                            _ => {}
                        }
                        match stats.total_byte_size {
                            datafusion::common::stats::Precision::Exact(n) => {
                                estimated_bytes = Some(n)
                            }
                            datafusion::common::stats::Precision::Inexact(n) => {
                                estimated_bytes = Some(n)
                            }
                            _ => {}
                        }
                    }

                    // 4. Construct JSON representation (Simplified for now)
                    let plan_json = serde_json::json!({
                        "name": "PhysicalPlan",
                        "children": [
                            { "name": format!("{}", physical_plan.schema()) }
                        ]
                    });

                    Json(PlanResponse {
                        plan_json,
                        physical_plan_text: plan_text,
                        cost_est: 0.0,
                        estimated_rows,
                        estimated_bytes,
                        warnings: vec![],
                    })
                }
                Err(e) => Json(PlanResponse {
                    plan_json: serde_json::json!({}),
                    physical_plan_text: format!("Physical Plan Error: {}", e),
                    cost_est: 0.0,
                    estimated_rows: None,
                    estimated_bytes: None,
                    warnings: vec![e.to_string()],
                }),
            }
        }
        Err(e) => Json(PlanResponse {
            plan_json: serde_json::json!({}),
            physical_plan_text: format!("Logical Plan Error: {}", e),
            cost_est: 0.0,
            estimated_rows: None,
            estimated_bytes: None,
            warnings: vec![e.to_string()],
        }),
    }
}

// Metrics Handler
async fn get_metrics() -> Json<cache_manager::MetricsSnapshot> {
    Json(cache_manager::get_metrics_registry().snapshot())
}

// Helper to generate test Excel file (if missing)
fn generate_test_excel(_path: &std::path::Path) {
    // This is just a placeholder to avoid crashes if file is missing.
    // Real implementation would use rust_xlsxwriter or similar.
    println!("Warning: users.xlsx missing. Please provide it.");
}

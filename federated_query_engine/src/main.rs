mod config;
#[cfg(test)]
mod cache_e2e_test;
mod cache_manager;
#[cfg(test)]
mod cache_stress_test;
mod datasources;
mod logger;
mod metadata_manager;
mod optimizer;
#[cfg(test)]
mod link_consistency_test;
#[cfg(test)]
mod pool_reliability_test;
mod query_rewriter;
mod resources;
#[cfg(test)]
mod sql_parser_tpcc_tests;
pub mod utils;

use crate::config::AppConfig;
use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use datafusion::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
// use datafusion::execution::context::SessionState;
use datafusion::optimizer::Optimizer;
use datafusion::execution::session_state::SessionStateBuilder;
use datasources::{
    csv::CsvDataSource, excel::ExcelDataSource, parquet::ParquetDataSource, DataSource,
};
// use datasources::sqlite::SqliteDataSource;
use cache_manager::CacheManager;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::execution::disk_manager::{DiskManager, DiskManagerMode};
use datafusion::execution::memory_pool::FairSpillPool;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::logical_expr::LogicalPlan;
use futures::StreamExt;
use metadata_manager::MetadataManager;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

struct AppState {
    ctx: SessionContext,
    metadata_manager: Arc<MetadataManager>,
}

/// 构建 DataFusion SessionContext
///
/// **实现方案**:
/// 1. 初始化 `Optimizer` 并添加自定义规则 `OraclePushDown`。
/// 2. 构建 `SessionState`，注入配置、运行时环境和优化器规则。
/// 3. 返回 `SessionContext`。
///
/// **关键问题点**:
/// - 优化器规则顺序：`OraclePushDown` 需要在默认规则之前或之后生效，当前策略是追加到默认规则列表。
fn build_session_context(
    session_config: SessionConfig,
    runtime_env: Arc<datafusion::execution::runtime_env::RuntimeEnv>,
) -> SessionContext {
    let mut rules = Optimizer::default().rules;
    rules.push(Arc::new(optimizer::oracle_pushdown::OraclePushDown::new()));

    let state = SessionStateBuilder::new_with_default_features()
        .with_config(session_config)
        .with_runtime_env(runtime_env)
        .with_optimizer_rules(rules)
        .build();
    
    SessionContext::new_with_state(state)
}

fn build_routing_rules(schema_name: &str, table_name: &str) -> Vec<String> {
    let mut rules = Vec::new();
    let table = table_name.trim();
    if table.contains('.') {
        rules.push(table.to_string());
        let (_, simple) = query_rewriter::IdentifierNormalizer::parse(table);
        rules.push(simple);
        return rules;
    }
    let schema = schema_name.trim();
    if !schema.is_empty() {
        rules.push(format!("{}.{}", schema, table));
    }
    rules.push(table.to_string());
    rules
}

fn build_scoped_name(prefix: &str, schema_name: &str, table_name: &str, config: &str) -> String {
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    let hash_hex = format!("{:x}", hasher.finish());
    format!("{}_{}_{}_{}", prefix, schema_name, hash_hex, table_name).to_lowercase()
}

fn resolve_schema_and_table(
    schema_opt: Option<String>,
    table_name: &str,
    default_schema: &str,
) -> (String, String, bool) {
    let (parsed_schema, simple_table) = query_rewriter::IdentifierNormalizer::parse(table_name);
    if let Some(s) = schema_opt {
        let schema = s.trim().to_string();
        if !schema.is_empty() {
            return (schema, simple_table, true);
        }
    }
    if let Some(schema) = parsed_schema {
        if !schema.trim().is_empty() {
            return (schema, simple_table, true);
        }
    }
    (default_schema.to_string(), simple_table, false)
}

fn extract_missing_table_name(err_msg: &str) -> Option<String> {
    for (prefix, suffix) in [("table '", "'"), ("table \"", "\""), ("table `", "`")] {
        if let Some(start) = err_msg.find(prefix) {
            let rest = &err_msg[start + prefix.len()..];
            if let Some(end) = rest.find(suffix) {
                let name = rest[..end].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

#[cfg(feature = "oracle")]
async fn auto_register_missing_oracle_table(
    state: &Arc<AppState>,
    missing_table: &str,
) -> std::result::Result<Option<String>, String> {
    let (schema_opt, simple_table) = query_rewriter::IdentifierNormalizer::parse(missing_table);
    let schema_name = schema_opt.unwrap_or_else(|| "public".to_string());

    // 1. Try to find credentials from existing TABLES
    let all_tables = state.metadata_manager.list_tables().unwrap_or_default();
    let table_candidate = all_tables
        .iter()
        .find(|t| {
            t.source_type == "oracle"
                && t.schema_name.eq_ignore_ascii_case(&schema_name)
                && serde_json::from_str::<serde_json::Value>(&t.file_path).is_ok()
        })
        .or_else(|| {
            all_tables.iter().find(|t| {
                t.source_type == "oracle"
                    && serde_json::from_str::<serde_json::Value>(&t.file_path).is_ok()
            })
        });

    let config_val: serde_json::Value = if let Some(seed_meta) = table_candidate {
        serde_json::from_str(&seed_meta.file_path).map_err(|e| e.to_string())?
    } else {
        // 2. Fallback: Try to find credentials from saved CONNECTIONS
        let connections = state.metadata_manager.list_connections().unwrap_or_default();
        let conn_candidate = connections.iter().find(|c| c.source_type == "oracle");
        
        if let Some(conn) = conn_candidate {
            serde_json::from_str(&conn.config).map_err(|e| e.to_string())?
        } else {
            return Ok(None);
        }
    };

    let user = config_val["user"].as_str().unwrap_or("").trim().to_string();
    let pass = config_val["pass"].as_str().unwrap_or("").trim().to_string();
    let host = config_val["host"].as_str().unwrap_or("").trim().to_string();
    let port = config_val["port"].as_u64().unwrap_or(1521) as u16;
    let service = config_val["service"].as_str().unwrap_or("").trim().to_string();
    
    if user.is_empty() || host.is_empty() || service.is_empty() {
        return Ok(None);
    }
    
    // Use consistent config JSON string for hashing
    let config_str = serde_json::json!({
        "user": user,
        "pass": pass,
        "host": host,
        "port": port,
        "service": service
    }).to_string();

    let scoped_name = build_scoped_name("oracle", &schema_name, &simple_table, &config_str);
    let sql_table_name = if !schema_name.is_empty() && schema_name != "public" && schema_name != "default" {
        format!("{}.{}", schema_name, simple_table)
    } else {
        simple_table.clone()
    };
    let source = datasources::oracle::OracleDataSource::new(
        scoped_name.clone(),
        user,
        pass,
        host,
        port,
        service,
        sql_table_name,
    )
    .map_err(|e| e.to_string())?;

    let _ = source.register_with_name(&state.ctx, &scoped_name).await;
    if !schema_name.is_empty() && schema_name != "public" && schema_name != "default" {
        let _ = source
            .register_with_schema(&state.ctx, &schema_name, &scoped_name)
            .await;
        let _ = source
            .register_with_schema(&state.ctx, &schema_name, &simple_table)
            .await;
    }

    let exists = all_tables
        .iter()
        .any(|t| t.table_name.eq_ignore_ascii_case(&scoped_name));
    if !exists {
        let stats_json = match source.get_table_stats() {
            Ok(Some((num_rows, avg_len))) => Some(
                serde_json::json!({
                    "num_rows": num_rows,
                    "avg_row_len": avg_len
                })
                .to_string(),
            ),
            _ => None,
        };
        let _ = state
            .metadata_manager
            .register_table(
                &state.ctx,
                "datafusion",
                &schema_name,
                &scoped_name,
                &config_str,
                "oracle",
                Some(simple_table.clone()),
                stats_json,
            )
            .await;
    }

    let logical_name = if !schema_name.is_empty() && schema_name != "public" && schema_name != "default" {
        format!("{}.{}", schema_name, simple_table)
    } else {
        simple_table.clone()
    };
    for rule in build_routing_rules(&schema_name, &logical_name) {
        query_rewriter::set_routing_rule(rule, scoped_name.clone());
    }
    query_rewriter::set_routing_rule(missing_table.to_string(), scoped_name.clone());
    Ok(Some(scoped_name))
}

#[cfg(not(feature = "oracle"))]
async fn auto_register_missing_oracle_table(
    _state: &Arc<AppState>,
    _missing_table: &str,
) -> std::result::Result<Option<String>, String> {
    Ok(None)
}

/// 确保 Oracle 客户端库在 PATH 环境变量中
///
/// **实现方案**:
/// 1. 检查 `ORACLE_CLIENT_DIR` 环境变量。
/// 2. 检查当前目录及 `federated_query_engine` 子目录是否存在 `oci.dll`。
/// 3. 检查常见的默认安装路径。
/// 4. 如果找到有效路径且尚未在 PATH 中，则追加到 PATH。
///
/// **关键问题点**:
/// - Windows 平台依赖 `oci.dll`，必须在进程启动早期设置 PATH。
fn ensure_oracle_client_path() {
    let mut candidates = Vec::new();
    if let Ok(v) = std::env::var("ORACLE_CLIENT_DIR") {
        let val = v.trim();
        if !val.is_empty() {
            candidates.push(val.to_string());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let in_root = cwd.join("oci.dll");
        if in_root.exists() {
            candidates.push(cwd.to_string_lossy().to_string());
        }
        let in_sub = cwd.join("federated_query_engine").join("oci.dll");
        if in_sub.exists() {
            candidates.push(
                cwd.join("federated_query_engine")
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    let fallback = r"C:\Users\tangguokai\Downloads\instantclient_19_29";
    if std::path::Path::new(fallback).join("oci.dll").exists() {
        candidates.push(fallback.to_string());
    }
    let path_var = match std::env::var("PATH") {
        Ok(v) => v,
        Err(_) => {
            crate::logger::log("PATH环境变量不存在，无法设置Oracle客户端目录");
            return;
        }
    };
    let lower_path = path_var.to_lowercase();
    for candidate in candidates {
        let trimmed = candidate
            .trim()
            .trim_end_matches('\\')
            .trim_end_matches('/')
            .to_string();
        if trimmed.is_empty() {
            continue;
        }
        if lower_path.contains(&trimmed.to_lowercase()) {
            crate::logger::log(&format!("Oracle客户端目录已在PATH中: {}", trimmed));
            return;
        }
        let new_path = format!("{};{}", trimmed, path_var);
        std::env::set_var("PATH", new_path);
        crate::logger::log(&format!("已将Oracle客户端目录加入PATH: {}", trimmed));
        return;
    }
    crate::logger::log("未找到可用的Oracle客户端目录");
}

// Helper for API responses
async fn list_tables(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // List from Metadata Manager to show rich metadata
    if let Ok(tables) = state.metadata_manager.list_tables() {
        let json_tables: Vec<serde_json::Value> = tables
            .iter()
            .map(|t| {
                serde_json::json!({
                    "catalog_name": t.catalog_name,
                    "schema_name": t.schema_name,
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

async fn get_logs(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let log_list = logger::get_logs();
    Json(serde_json::json!({ "status": "ok", "logs": log_list }))
}

async fn get_sidecar_logs(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let log_list = logger::get_sidecar_logs();
    Json(serde_json::json!({ "status": "ok", "logs": log_list }))
}

// --- Connection API Handlers ---

#[derive(Deserialize)]
struct SaveConnectionRequest {
    id: String,
    name: String,
    source_type: String,
    config: serde_json::Value,
}

#[cfg(feature = "oracle")]
async fn prefetch_oracle_tables_for_connection(
    state: &Arc<AppState>,
    config: &serde_json::Value,
) -> std::result::Result<usize, String> {
    let user = config["user"].as_str().unwrap_or("").trim().to_string();
    let pass = config["pass"].as_str().unwrap_or("").trim().to_string();
    let host = config["host"].as_str().unwrap_or("").trim().to_string();
    let port = config["port"].as_u64().unwrap_or(1521) as u16;
    let service = config["service"].as_str().unwrap_or("").trim().to_string();
    if user.is_empty() || host.is_empty() || service.is_empty() {
        return Ok(0);
    }

    let config_str = serde_json::json!({
        "user": user,
        "pass": pass,
        "host": host,
        "port": port,
        "service": service
    })
    .to_string();

    let tables = datasources::oracle::OracleDataSource::test_connection(
        config["user"].as_str().unwrap_or("").trim(),
        config["pass"].as_str().unwrap_or("").trim(),
        config["host"].as_str().unwrap_or("").trim(),
        port,
        config["service"].as_str().unwrap_or("").trim(),
    )
    .map_err(|e| e.to_string())?;

    let mut registered = 0usize;
    for (schema_name, table_name, num_rows, avg_row_len) in tables {
        let schema_val = schema_name.trim().to_string();
        let simple_table = table_name.trim().to_string();
        if schema_val.is_empty() || simple_table.is_empty() {
            continue;
        }
        let scoped_name = build_scoped_name("oracle", &schema_val, &simple_table, &config_str);
        let sql_table_name = format!("{}.{}", schema_val, simple_table);

        let source = match datasources::oracle::OracleDataSource::new(
            scoped_name.clone(),
            user.clone(),
            pass.clone(),
            host.clone(),
            port,
            service.clone(),
            sql_table_name,
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let _ = source.register_with_name(&state.ctx, &scoped_name).await;
        let _ = source
            .register_with_schema(&state.ctx, &schema_val, &scoped_name)
            .await;
        let _ = source
            .register_with_schema(&state.ctx, &schema_val, &simple_table)
            .await;

        let stats_json = if num_rows.is_some() || avg_row_len.is_some() {
            Some(
                serde_json::json!({
                    "num_rows": num_rows,
                    "avg_row_len": avg_row_len
                })
                .to_string(),
            )
        } else {
            None
        };

        let _ = state
            .metadata_manager
            .register_table(
                &state.ctx,
                "datafusion",
                &schema_val,
                &scoped_name,
                &config_str,
                "oracle",
                Some(simple_table.clone()),
                stats_json,
            )
            .await;

        for rule in build_routing_rules(&schema_val, &simple_table) {
            query_rewriter::set_routing_rule(rule, scoped_name.clone());
        }
        query_rewriter::set_routing_rule(
            format!("{}.{}", schema_val, simple_table),
            scoped_name.clone(),
        );
        registered += 1;
    }

    Ok(registered)
}

#[cfg(not(feature = "oracle"))]
async fn prefetch_oracle_tables_for_connection(
    _state: &Arc<AppState>,
    _config: &serde_json::Value,
) -> std::result::Result<usize, String> {
    Ok(0)
}

async fn save_connection(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SaveConnectionRequest>,
) -> Json<serde_json::Value> {
    crate::app_log!(
        "Saving connection: {} ({})",
        payload.name,
        payload.source_type
    );
    if let Err(e) = state.metadata_manager.save_connection(
        &payload.id,
        &payload.name,
        &payload.source_type,
        &payload.config.to_string(),
    ) {
        crate::app_log!("Failed to save connection: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    let mut prefetch_registered = 0usize;
    if payload.source_type.eq_ignore_ascii_case("oracle") {
        match prefetch_oracle_tables_for_connection(&state, &payload.config).await {
            Ok(count) => {
                prefetch_registered = count;
                crate::app_log!(
                    "Oracle connection prefetch finished: {} tables registered",
                    count
                );
            }
            Err(e) => {
                crate::app_log!("Oracle connection prefetch failed: {}", e);
            }
        }
    }
    Json(serde_json::json!({ "status": "ok", "prefetch_registered": prefetch_registered }))
}

async fn list_connections(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    crate::app_log!("Listing connections...");
    match state.metadata_manager.list_connections() {
        Ok(conns) => {
            crate::app_log!("Found {} connections", conns.len());
            let json_conns: Vec<serde_json::Value> = conns.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "source_type": c.source_type,
                    "config": serde_json::from_str::<serde_json::Value>(&c.config).unwrap_or(serde_json::json!({}))
                })
            }).collect();
            Json(serde_json::json!({ "status": "ok", "connections": json_conns }))
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn delete_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.metadata_manager.delete_connection(&id) {
        Ok(_) => Json(serde_json::json!({ "status": "ok" })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
struct UnregisterTableParams {
    catalog: Option<String>,
    schema: Option<String>,
}

/// 注销表 API 处理函数
///
/// **实现方案**:
/// 1. 列出所有表。
/// 2. 过滤出符合条件的表（名称、Schema、Catalog）。
/// 3. 调用 DataFusion `deregister_table`。
/// 4. 调用 `metadata_manager.unregister_table` 持久化删除。
///
/// **关键问题点**:
/// - 幂等性：支持重复调用。
/// - 级联删除：目前仅删除元数据，不删除底层文件（如果是文件源）。
async fn unregister_table_handler(
    State(state): State<Arc<AppState>>,
    Path(table_name): Path<String>,
    Query(params): Query<UnregisterTableParams>,
) -> Json<serde_json::Value> {
    match state.metadata_manager.list_tables() {
        Ok(tables) => {
            let mut deleted = 0;
            for t in tables {
                if t.table_name == table_name {
                    // Filter by schema if provided
                    if let Some(s) = &params.schema {
                        if &t.schema_name != s {
                            continue;
                        }
                    }
                    // Filter by catalog if provided
                    if let Some(c) = &params.catalog {
                        if &t.catalog_name != c {
                            continue;
                        }
                    }

                    // Deregister from DataFusion
                    let _ = state.ctx.deregister_table(&table_name);

                    if state
                        .metadata_manager
                        .unregister_table(&t.catalog_name, &t.schema_name, &t.table_name)
                        .is_ok()
                    {
                        deleted += 1;
                        crate::app_log!("Unregistered table: {}", table_name);
                    }
                }
            }
            if deleted > 0 {
                Json(
                    serde_json::json!({ "status": "ok", "message": format!("Unregistered {} instances of {}", deleted, table_name) }),
                )
            } else {
                Json(serde_json::json!({ "status": "warning", "message": "Table not found" }))
            }
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn list_connection_tables(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // 1. Get Connection Config
    let conn_meta = match state.metadata_manager.get_connection(&id) {
        Ok(Some(c)) => c,
        Ok(Option::None) => {
            return Json(
                serde_json::json!({ "status": "error", "message": "Connection not found" }),
            )
        }
        Err(e) => return Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    };

    let config: serde_json::Value =
        serde_json::from_str(&conn_meta.config).unwrap_or(serde_json::json!({}));

    // 2. Fetch Tables based on Type
    match conn_meta.source_type.as_str() {
        "yashandb" => {
            let user = config["user"].as_str().unwrap_or("").trim().to_string();
            let pass = config["pass"].as_str().unwrap_or("").trim().to_string();
            let host = config["host"].as_str().unwrap_or("").trim().to_string();
            let port = config["port"].as_u64().unwrap_or(1688) as u16;
            let service = config["service"].as_str().unwrap_or("").trim().to_string();
            let sql_query = config["sql_query"].as_str().map(|s| s.to_string());

            // Check cache first
            let cache_dir = std::path::Path::new("metadata_cache");
            let cache_path = cache_dir.join(format!("{}.parquet", id));

            if cache_path.exists() {
                crate::logger::log(&format!("Cache hit for connection tables: {}", id));
                if let Ok(file) = std::fs::File::open(&cache_path) {
                    if let Ok(builder) = datafusion::parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file) {
                         if let Ok(mut reader) = builder.build() {
                             let mut all_tables = Vec::new();
                             while let Some(Ok(batch)) = reader.next() {
                                 let owners = batch.column(0).as_any().downcast_ref::<datafusion::arrow::array::StringArray>().unwrap();
                                 let tables = batch.column(1).as_any().downcast_ref::<datafusion::arrow::array::StringArray>().unwrap();
                                 for i in 0..batch.num_rows() {
                                     all_tables.push(serde_json::json!({
                                         "schema": owners.value(i),
                                         "table_name": tables.value(i)
                                     }));
                                 }
                             }
                             return Json(serde_json::json!({ "status": "ok", "tables": all_tables }));
                         }
                     }
                }
            }

            match datasources::yashandb::YashanDataSource::test_connection(
                &user, &pass, &host, port, &service, sql_query.clone(), None, None,
            ) {
                Ok(tables) => {
                    // Sidecar: Fetch full metadata and save to Parquet
                    let user_clone = user.clone();
                    let pass_clone = pass.clone();
                    let host_clone = host.clone();
                    let service_clone = service.clone();
                    let conn_id = id.clone();
                    let sql_query_clone = sql_query.clone();

                    tokio::task::spawn_blocking(move || {
                        // Use CacheManager's flight logic to avoid duplicate sidecars
                        let flight_key = format!("sidecar:{}", conn_id);
                        let handle = tokio::runtime::Handle::current();

                        // We need to run async code in blocking thread, or spawn async task.
                        // But CacheManager::join_or_start_flight is async.
                        // Let's just spawn an async task from here.
                        handle.spawn(async move {
                             match CacheManager::join_or_start_flight(flight_key) {
                                 crate::cache_manager::FlightResult::IsLeader(flight_guard) => {
                                     crate::logger::log(&format!("Starting sidecar fetch for {}", conn_id));
                                     // Now run blocking fetch
                                    type SidecarFetchResult = Result<
                                        Result<
                                            Vec<(String, String, Option<i64>, Option<i64>)>,
                                            datafusion::error::DataFusionError,
                                        >,
                                        tokio::task::JoinError,
                                    >;
                                    let res: SidecarFetchResult = tokio::task::spawn_blocking(move || {
                                        datasources::yashandb::YashanDataSource::test_connection(&user_clone, &pass_clone, &host_clone, port, &service_clone, sql_query_clone, None, None)
                                    }).await;

                                    match res {
                                        Ok(Ok(full_tables)) => {
                                            // Convert and Save
                                            let owners: Vec<String> = full_tables
                                                .iter()
                                                .map(|(o, _, _, _): &(String, String, Option<i64>, Option<i64>)| o.clone())
                                                .collect();
                                            let table_names: Vec<String> = full_tables
                                                .iter()
                                                .map(|(_, t, _, _): &(String, String, Option<i64>, Option<i64>)| t.clone())
                                                .collect();
                                            let schema = std::sync::Arc::new(datafusion::arrow::datatypes::Schema::new(vec![
                                                 datafusion::arrow::datatypes::Field::new("owner", datafusion::arrow::datatypes::DataType::Utf8, false),
                                                 datafusion::arrow::datatypes::Field::new("table_name", datafusion::arrow::datatypes::DataType::Utf8, false),
                                             ]));
                                             let batch = datafusion::arrow::record_batch::RecordBatch::try_new(
                                                 schema.clone(),
                                                 vec![
                                                     std::sync::Arc::new(datafusion::arrow::array::StringArray::from(owners)),
                                                     std::sync::Arc::new(datafusion::arrow::array::StringArray::from(table_names)),
                                                 ],
                                             );
                                             if let Ok(batch) = batch {
                                                  let cache_dir = std::path::Path::new("metadata_cache");
                                                  if !cache_dir.exists() {
                                                      let _ = std::fs::create_dir_all(cache_dir);
                                                  }
                                                  let file_path = cache_dir.join(format!("{}.parquet", conn_id));
                                                  if let Ok(file) = std::fs::File::create(file_path) {
                                                      let props = datafusion::parquet::file::properties::WriterProperties::builder().build();
                                                      if let Ok(mut writer) = datafusion::parquet::arrow::ArrowWriter::try_new(file, schema, Some(props)) {
                                                          let _ = writer.write(&batch);
                                                          let _ = writer.close();
                                                          crate::logger::log(&format!("Sidecar fetch completed for {}. Saved to metadata_cache.", conn_id));
                                                      }
                                                  }
                                             }
                                         },
                                         Ok(Err(e)) => crate::logger::log(&format!("Sidecar fetch failed logic: {}", e)),
                                         Err(e) => crate::logger::log(&format!("Sidecar fetch task panicked: {}", e)),
                                     }
                                     // Drop guard
                                     drop(flight_guard);
                                 },
                                 crate::cache_manager::FlightResult::IsFollower(_) => {
                                     crate::logger::log(&format!("Sidecar for {} already running, skipping.", conn_id));
                                 }
                             }
                         });
                    });

                    let table_objs: Vec<serde_json::Value> = tables.into_iter().map(|(s, t, _, _): (String, String, Option<i64>, Option<i64>)| {
                        serde_json::json!({ "schema": s, "table_name": t })
                    }).collect();
                    Json::<serde_json::Value>(
                        serde_json::json!({ "status": "ok", "tables": table_objs }),
                    )
                }
                Err(e) => Json::<serde_json::Value>(
                    serde_json::json!({ "status": "error", "message": e.to_string() }),
                ),
            }
        }
        "oracle" => {
            #[cfg(feature = "oracle")]
            {
                let user = config["user"].as_str().unwrap_or("").to_string();
                let pass = config["pass"].as_str().unwrap_or("").to_string();
                let host = config["host"].as_str().unwrap_or("").to_string();
                let port = config["port"].as_u64().unwrap_or(1521) as u16;
                let service = config["service"].as_str().unwrap_or("").to_string();

                match datasources::oracle::OracleDataSource::test_connection(
                    &user, &pass, &host, port, &service,
                ) {
                    Ok(tables) => {
                        let table_objs: Vec<serde_json::Value> = tables.into_iter().map(|(s, t, _, _): (String, String, Option<i64>, Option<i64>)| {
                            serde_json::json!({ "schema": s, "table_name": t })
                        }).collect();
                        Json::<serde_json::Value>(
                            serde_json::json!({ "status": "ok", "tables": table_objs }),
                        )
                    }
                    Err(e) => Json::<serde_json::Value>(
                        serde_json::json!({ "status": "error", "message": e.to_string() }),
                    ),
                }
            }
            #[cfg(not(feature = "oracle"))]
            Json::<serde_json::Value>(
                serde_json::json!({ "status": "error", "message": "Oracle not supported" }),
            )
        }
        "sqlite" => {
            // let path = config["path"].as_str().unwrap_or("").to_string();
            // match datasources::sqlite::SqliteDataSource::list_tables(&path) { ... }
            Json::<serde_json::Value>(
                serde_json::json!({ "status": "error", "message": "Sqlite not supported" }),
            )
        }
        _ => Json::<serde_json::Value>(
            serde_json::json!({ "status": "error", "message": "Unsupported connection type" }),
        ),
    }
}

fn main() {
    ensure_oracle_client_path();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main());
}

/// 异步入口点
///
/// **实现方案**:
/// 1. **环境清理**: 删除不完整的缓存文件。
/// 2. **运行时配置**: 设置 DataFusion 内存限制 (系统 70%) 和磁盘溢写目录。
/// 3. **会话初始化**: 创建 DataFusion `SessionContext`，注册优化器规则。
/// 4. **元数据初始化**: 加载 `metadata.db`，恢复已注册的数据源。
/// 5. **Web 服务启动**: 绑定端口，启动 Axum HTTP 服务。
///
/// **关键问题点**:
/// - 启动顺序：先环境检查，再初始化核心组件，最后启动网络服务。
/// - 持久化恢复：自动重新注册 `yashandb`、`oracle` 等外部数据源。
async fn async_main() {
    // --- Startup Cleanup ---
    // Remove incomplete .tmp files from cache directory
    let cache_dir = std::path::Path::new("cache/yashandb");
    if cache_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Check if file ends with .tmp
                if path.extension().is_some_and(|ext| ext == "tmp") {
                    if let Err(e) = std::fs::remove_file(&path) {
                        crate::app_log!("Failed to remove incomplete cache file {:?}: {}", path, e);
                    } else {
                        crate::app_log!("Removed incomplete cache file: {:?}", path);
                    }
                }
            }
        }
    }

    // --- 配置运行时环境（磁盘落盘 & 内存限制） ---
    // 1. 内存限制：系统内存的 70%
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::everything()));
    sys.refresh_memory();
    let total_memory = sys.total_memory();
    // sys.total_memory() 返回字节数
    let memory_limit = (total_memory as f64 * 0.70) as usize;

    crate::logger::log(&format!(
        "配置 DataFusion 内存限制: {} MB (总内存: {} MB)",
        memory_limit / 1024 / 1024,
        total_memory / 1024 / 1024
    ));

    // 2. 磁盘落盘：使用 'execution_spill' 目录
    let spill_dir = std::path::Path::new("execution_spill");
    if !spill_dir.exists() {
        let _ = std::fs::create_dir_all(spill_dir);
    }

    // 3. 会话配置：使用 "Partitioned Hash Join with Spilling" (Grace Hash Join)
    // 这是 DataFusion/Spark/Presto 的"黄金平衡点"：
    // - 默认使用 Hash Join (最快)
    // - 开启并行 Join (Repartition)
    // - 当内存不足时自动溢写磁盘 (Spilling)
    let mut session_config = SessionConfig::new()
        .with_batch_size(8192)
        .with_repartition_joins(true); // 开启并行 Join (Partitioned)

    // 确保不强制使用 SortMergeJoin，让优化器默认选择 Hash Join
    // DataFusion 的 HashJoinExec 实现了自动落盘机制 (Hybrid Hash Join)
    let _ = session_config
        .options_mut()
        .set("datafusion.optimizer.prefer_sort_merge_join", "false");
    let _ = session_config
        .options_mut()
        .set("datafusion.sql_parser.enable_ident_normalization", "true");

    // 使用自定义配置初始化 DataFusion 上下文
    let runtime_env =
        RuntimeEnvBuilder::new()
            .with_disk_manager_builder(DiskManager::builder().with_mode(
                DiskManagerMode::Directories(vec![std::path::PathBuf::from(spill_dir)]),
            ))
            .with_memory_pool(Arc::new(FairSpillPool::new(memory_limit)))
            .build()
            .expect("无法创建 RuntimeEnv");
    let ctx = build_session_context(session_config, Arc::new(runtime_env));

    // Initialize Config & Logger
    let config = AppConfig::global();
    crate::logger::init_logger(config.runtime_dir.join("logs").to_str().unwrap());
    
    crate::logger::log("Starting Federated Query Engine...");
    crate::logger::log(&format!("Runtime Directory: {:?}", config.runtime_dir));

    // Determine paths based on Config
    let data_path_str = if std::path::Path::new("federated_query_engine").exists() {
        "federated_query_engine/data"
    } else {
        "data"
    };
    let public_path_str = if std::path::Path::new("federated_query_engine").exists() {
        "federated_query_engine/public"
    } else {
        "public"
    };

    // Initialize Metadata Manager with unified path
    let metadata_manager = MetadataManager::new(config.metadata_path.to_str().unwrap())
        .expect("Failed to initialize metadata manager");
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
            // Consistency Check: Ensure file exists (only for file-based sources)
            let is_file_source =
                ["csv", "excel", "parquet", "sqlite"].contains(&table.source_type.as_str());
            if is_file_source && !std::path::Path::new(&table.file_path).exists() {
                crate::app_log!(
                    "Warning: File for table '{}' not found at '{}'. Cleaning up metadata.",
                    table.table_name,
                    table.file_path
                );
                if let Err(e) = metadata_manager.unregister_table(
                    &table.catalog_name,
                    &table.schema_name,
                    &table.table_name,
                ) {
                    crate::app_log!(
                        "Failed to unregister invalid table '{}': {}",
                        table.table_name,
                        e
                    );
                }
                continue;
            }

            // Currently assuming all tables are in default catalog/schema for DataFusion registration
            // TODO: Support catalog/schema in DataSource trait

            let source: Option<Box<dyn DataSource>> = if table.source_type == "csv" {
                match CacheManager::ensure_parquet_cache(
                    &table.file_path,
                    "csv",
                    Option::<String>::None,
                )
                .await
                {
                    Ok(p) => Some(Box::new(ParquetDataSource::new(
                        table.table_name.clone(),
                        p,
                    ))),
                    Err(e) => {
                        crate::app_log!(
                            "Transcoding failed for {}, using original CSV: {}",
                            table.table_name,
                            e
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
                        crate::app_log!(
                            "Transcoding failed for {}, using original Excel: {}",
                            table.table_name,
                            e
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
                // let internal_name = table.sheet_name.clone().unwrap_or(table.table_name.clone());
                // Some(Box::new(SqliteDataSource::new(table.table_name.clone(), table.file_path.clone(), internal_name)))
                crate::app_log!("Sqlite source disabled");
                None
            } else if table.source_type == "oracle" {
                #[cfg(feature = "oracle")]
                {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&table.file_path)
                    {
                        let user = config["user"].as_str().unwrap_or("").to_string();
                        let pass = config["pass"].as_str().unwrap_or("").to_string();
                        let host = config["host"].as_str().unwrap_or("").to_string();
                        let port = config["port"].as_u64().unwrap_or(1521) as u16;
                        let service = config["service"].as_str().unwrap_or("").to_string();
                        let sql_table =
                            table.sheet_name.clone().unwrap_or(table.table_name.clone());

                        match datasources::oracle::OracleDataSource::new(
                            table.table_name.clone(),
                            user,
                            pass,
                            host,
                            port,
                            service,
                            sql_table,
                        ) {
                            Ok(ds) => {
                                // Register to specific schema if needed (Persistence Fix)
                                if !table.schema_name.is_empty()
                                    && table.schema_name != "public"
                                    && table.schema_name != "default"
                                {
                                    if let Err(e) = ds
                                        .register_with_schema(&ctx, &table.schema_name, &table.table_name)
                                        .await
                                    {
                                        crate::app_log!(
                                            "Failed to register persisted Oracle table in schema {}: {}",
                                            table.schema_name,
                                            e
                                        );
                                    } else {
                                        crate::app_log!(
                                            "Registered persisted Oracle table in schema {}: {}",
                                            table.schema_name,
                                            table.table_name
                                        );
                                    }
                                }
                                Some(Box::new(ds))
                            }
                            Err(e) => {
                                crate::app_log!(
                                    "Failed to init Oracle source {}: {}",
                                    table.table_name,
                                    e
                                );
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                #[cfg(not(feature = "oracle"))]
                None
            } else if table.source_type == "yashandb" {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&table.file_path) {
                    let user = config["user"].as_str().unwrap_or("").to_string();
                    let pass = config["pass"].as_str().unwrap_or("").to_string();
                    let host = config["host"].as_str().unwrap_or("").to_string();
                    let port = config["port"].as_u64().unwrap_or(1688) as u16;
                    let service = config["service"].as_str().unwrap_or("").to_string();
                    let sql_query = config["sql_query"].as_str().map(|s| s.to_string());
                    let schema = if table.schema_name.is_empty() || table.schema_name == "default" {
                        None
                    } else {
                        Some(table.schema_name.clone())
                    };

                    match datasources::yashandb::YashanDataSource::new(
                        table.table_name.clone(),
                        schema,
                        user,
                        pass,
                        host,
                        port,
                        service,
                        sql_query,
                    ) {
                        Ok(mut source_obj) => {
                            // Set remote table name from sheet_name if available
                            if let Some(remote_name) = &table.sheet_name {
                                source_obj = source_obj.with_remote_table_name(remote_name.clone());
                            }

                            // Restore stats
                            if let Some(stats_json) = &table.stats_json {
                                if let Ok(json) =
                                    serde_json::from_str::<serde_json::Value>(stats_json)
                                {
                                    let num_rows = json["num_rows"].as_i64();
                                    let avg_len = json["avg_row_len"].as_i64();
                                    if let (Some(n), Some(a)) = (num_rows, avg_len) {
                                        source_obj.stats = Some((n, a));
                                    }
                                }
                            }

                            Some(Box::new(source_obj))
                        }
                        Err(e) => {
                            crate::app_log!(
                                "Failed to init YashanDB source '{}': {}. Skipping.",
                                table.table_name,
                                e
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(source) = source {
                if source.register(&ctx).await.is_ok() {
                    crate::app_log!("Registered persisted data source: {}", source.name());
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
                            table.stats_json.clone(),
                        )
                        .await
                    {
                        crate::app_log!(
                            "Failed to refresh metadata for {}: {}",
                            table.table_name,
                            e
                        );
                    }

                    // RESTORE ROUTING RULE
                    if table.source_type == "yashandb" || table.source_type == "oracle" {
                        if let Some(real_name) = &table.sheet_name {
                            let rules = build_routing_rules(&table.schema_name, real_name);
                            for rule in rules {
                                query_rewriter::set_routing_rule(
                                    rule.clone(),
                                    table.table_name.clone(),
                                );
                                crate::app_log!("已恢复路由规则: {} -> {}", rule, table.table_name);
                            }
                        }
                    }

                    registered_names.insert(source.name().to_string());
                } else {
                    crate::app_log!("Failed to register persisted table: {}", table.table_name);
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
                crate::app_log!("Registered data source: {}", source.name());
                let _ = metadata_manager
                    .register_table(
                        &ctx,
                        "datafusion",
                        "public",
                        "orders",
                        &orders_path,
                        "csv",
                        None,
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
                crate::app_log!("Registered data source: {}", source.name());
                let _ = metadata_manager
                    .register_table(
                        &ctx,
                        "datafusion",
                        "public",
                        "exchange_rates",
                        &rates_path,
                        "csv",
                        None,
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
            crate::app_log!("Registered data source: {}", source.name());
            let _ = metadata_manager
                .register_table(
                    &ctx,
                    "datafusion",
                    "public",
                    "users",
                    &path_str,
                    "excel",
                    Some("Sheet1".to_string()),
                    None,
                )
                .await;
        }
    }

    // 4. SQLite Metadata Store (Self-Introspection)
    // Register metadata.db itself as a queryable source
    let meta_db_path = config.metadata_path.to_str().unwrap();
    if std::path::Path::new(meta_db_path).exists() {
        /*
        if let Ok(tables) = SqliteDataSource::list_tables(meta_db_path) {
            for (_schema, table_name) in tables {
                let register_name = if table_name == "tables_metadata" { "sys_metadata".to_string() } else { table_name.clone() };

                // Avoid double registration if already persisted
                if !registered_names.contains(&register_name) {
                     let source = SqliteDataSource::new(register_name.clone(), meta_db_path.to_string(), table_name.clone());
                     if source.register(&ctx).await.is_ok() {
                         crate::app_log!("Registered SQLite source: {} -> {}", register_name, table_name);
                         let _ = metadata_manager.register_table(&ctx, "datafusion", "public", &register_name, meta_db_path, "sqlite", Some(table_name), None).await;
                     }
                }
            }
        }
        */
        crate::app_log!("SQLite introspection disabled");
    }

    // Shared State
    let state = Arc::new(AppState {
        ctx,
        metadata_manager,
    });

    let app = build_app(state.clone(), public_path_str);

    // Run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    // tracing::info!("listening on {}", addr);
    crate::logger::log(&format!("Backend server running at http://{}", addr));
    crate::logger::log("Version: Sidecar Logic with Type Probe v2");
    crate::logger::log(
        "变更备注 2026-02-27: 移除旧元数据自动发现脚本与 auto_link_scan API，避免旧元数据干扰，需手动注册",
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn build_app(state: Arc<AppState>, public_path_str: &str) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/upload", post(upload_file))
        .route("/api/register_table", post(register_table_endpoint))
        .route("/api/execute", post(execute_sql))
        .route("/api/link_tables", post(link_identical_tables))
        .route("/api/tables", get(list_tables))
        .route("/api/logs", get(get_logs))
        .route("/api/sidecar_logs", get(get_sidecar_logs))
        .route("/api/connect_sqlite", post(connect_sqlite))
        .route("/api/plan", post(get_plan))
        .route("/api/metrics", get(get_metrics))
        .route("/api/health", get(health))
        .route(
            "/api/connections",
            get(list_connections).post(save_connection),
        )
        .route("/api/connections/{id}", delete(delete_connection))
        .route("/api/connections/{id}/tables", get(list_connection_tables))
        .route("/api/tables/{name}", delete(unregister_table_handler))
        .route("/api/debug/oracle", post(debug_oracle))
        .route("/api/datasources/oracle/register", post(register_oracle))
        .route("/api/debug/yashandb", post(debug_yashandb))
        .route(
            "/api/datasources/yashandb/register",
            post(register_yashandb),
        )
        .route("/api/router/config", post(update_routing_config))
        .fallback_service(ServeDir::new(public_path_str).append_index_html_on_directories(true))
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": "0.1.0" }))
}

#[derive(Deserialize)]
struct ConnectSqliteRequest {
    #[allow(dead_code)]
    path: String,
}

async fn connect_sqlite(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<ConnectSqliteRequest>,
) -> Json<serde_json::Value> {
    // let path = payload.path;
    // match SqliteDataSource::list_tables(&path) { ... }
    Json(serde_json::json!({ "status": "error", "message": "Sqlite support temporarily disabled" }))
}

#[derive(Deserialize)]
struct OracleConnectRequest {
    user: String,
    pass: String,
    host: String,
    port: u16,
    service: String,
}

#[derive(Deserialize)]
struct OracleRegisterRequest {
    user: String,
    pass: String,
    host: String,
    port: u16,
    service: String,
    table_name: String,
    alias: Option<String>,
    schema: Option<String>,
}

#[cfg(feature = "oracle")]
async fn debug_oracle(Json(payload): Json<OracleConnectRequest>) -> Json<serde_json::Value> {
    match datasources::oracle::OracleDataSource::test_connection(
        payload.user.trim(),
        payload.pass.trim(),
        payload.host.trim(),
        payload.port,
        payload.service.trim(),
    ) {
        Ok(tables) => Json(serde_json::json!({ "status": "ok", "tables": tables })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

#[cfg(not(feature = "oracle"))]
async fn debug_oracle(Json(_): Json<OracleConnectRequest>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "error", "message": "Oracle feature not enabled" }))
}

#[cfg(feature = "oracle")]
async fn register_oracle(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OracleRegisterRequest>,
) -> Json<serde_json::Value> {
    let (schema_val, simple_table, schema_known) =
        resolve_schema_and_table(payload.schema.clone(), &payload.table_name, "public");
    let table_alias = payload.alias.clone().unwrap_or(simple_table.clone());
    let sql_table_name = if schema_known {
        format!("{}.{}", schema_val, simple_table)
    } else {
        simple_table.clone()
    };
    let config = serde_json::json!({
        "user": payload.user.trim(),
        "pass": payload.pass.trim(),
        "host": payload.host.trim(),
        "port": payload.port,
        "service": payload.service.trim()
    });
    let config_str = config.to_string();

    // 1. Create DataSource
    let source = match datasources::oracle::OracleDataSource::new(
        table_alias.clone().trim().to_string(),
        payload.user.trim().to_string(),
        payload.pass.trim().to_string(),
        payload.host.trim().to_string(),
        payload.port,
        payload.service.trim().to_string(),
        sql_table_name,
    ) {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    };

    // 2. Register
    // Always register scoped name: oracle_schema_table (flat name to avoid catalog issues)
    let scoped_name = build_scoped_name("oracle", &schema_val, &simple_table, &config_str);
    if let Err(e) = source.register_with_name(&state.ctx, &scoped_name).await {
        crate::app_log!(
            "Failed to register scoped Oracle table {}: {}",
            scoped_name,
            e
        );
    }

    // Also register in the specific schema if it's not default
    if !schema_val.is_empty() && schema_val != "public" && schema_val != "default" {
        if let Err(e) = source.register_with_schema(&state.ctx, &schema_val, &scoped_name).await {
             crate::app_log!("Failed to register Oracle table in schema {}: {}", schema_val, e);
        } else {
             crate::app_log!("Registered Oracle table in schema {}: {}", schema_val, scoped_name);
        }
    }

    // Add routing rule for schema.table -> scoped_name
    for rule in build_routing_rules(&schema_val, &payload.table_name) {
        query_rewriter::set_routing_rule(rule.clone(), scoped_name.clone());
        crate::app_log!("Added routing rule: {} -> {}", rule, scoped_name);
    }

    match source.register(&state.ctx).await {
        Ok(_) => {
            // 3. Persist Metadata
            // Collect Stats
            let stats_json = match source.get_table_stats() {
                Ok(Some((num_rows, avg_len))) => Some(
                    serde_json::json!({
                       "num_rows": num_rows,
                       "avg_row_len": avg_len
                    })
                    .to_string(),
                ),
                Ok(None) => None,
                Err(e) => {
                    crate::app_log!(
                        "Failed to collect stats for Oracle table {}: {}",
                        scoped_name,
                        e
                    );
                    None
                }
            };

            let _ = state
                .metadata_manager
                .register_table(
                    &state.ctx,
                    "datafusion",
                    &schema_val,
                    &scoped_name,
                    &config_str,
                    "oracle",
                    Some(payload.table_name),
                    stats_json,
                )
                .await;

            crate::app_log!(
                "Registered Oracle table: {} (Schema: {})",
                scoped_name,
                schema_val
            );
            Json(serde_json::json!({ "status": "ok", "message": "Oracle table registered" }))
        }
        Err(e) => {
            crate::app_log!("Failed to register Oracle table: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

#[cfg(not(feature = "oracle"))]
async fn register_oracle(
    State(_): State<Arc<AppState>>,
    Json(_): Json<OracleRegisterRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "error", "message": "Oracle feature not enabled" }))
}

#[derive(Deserialize)]
struct YashanConnectRequest {
    user: String,
    pass: String,
    host: String,
    port: u16,
    service: String,
}

#[derive(Deserialize)]
struct YashanRegisterRequest {
    user: String,
    pass: String,
    host: String,
    port: u16,
    service: String,
    table_name: String,
    sql_query: Option<String>, // Optional custom query, default to SELECT * FROM table_name
    alias: Option<String>,
    schema: Option<String>,
}

async fn debug_yashandb(Json(payload): Json<YashanConnectRequest>) -> Json<serde_json::Value> {
    match datasources::yashandb::YashanDataSource::test_connection(
        &payload.user,
        &payload.pass,
        &payload.host,
        payload.port,
        &payload.service,
        None,
        None,
        None,
    ) {
        Ok(tables) => Json(serde_json::json!({ "status": "ok", "tables": tables })),
        Err(e) => Json::<serde_json::Value>(
            serde_json::json!({ "status": "error", "message": e.to_string() }),
        ),
    }
}

async fn register_yashandb(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<YashanRegisterRequest>,
) -> Json<serde_json::Value> {
    let (schema_val, simple_table, schema_known) =
        resolve_schema_and_table(payload.schema.clone(), &payload.table_name, "default");
    let table_alias = payload.alias.clone().unwrap_or(simple_table.clone());
    let config = serde_json::json!({
        "user": payload.user,
        "pass": payload.pass,
        "host": payload.host,
        "port": payload.port,
        "service": payload.service,
        "sql_query": payload.sql_query
    });
    let config_str = config.to_string();

    // 1. Create DataSource
    let mut source = match datasources::yashandb::YashanDataSource::new(
        table_alias.clone(),
        if schema_known {
            Some(schema_val.clone())
        } else {
            None
        },
        payload.user.clone(),
        payload.pass.clone(),
        payload.host.clone(),
        payload.port,
        payload.service.clone(),
        payload.sql_query.clone(),
    ) {
        Ok(s) => s,
        Err(e) => {
            crate::app_log!("Failed to create YashanDB source: {}", e);
            return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
        }
    };

    // Fetch stats
    if let Err(e) = source.fetch_stats() {
        crate::app_log!("Warning: Failed to fetch stats for {}: {}", table_alias, e);
    }

    // 2. Register
    // Always register scoped name: yashan_schema_table
    let scoped_name = build_scoped_name("yashan", &schema_val, &simple_table, &config_str);
    if let Err(e) = source.register_with_name(&state.ctx, &scoped_name).await {
        crate::app_log!(
            "Failed to register scoped YashanDB table {}: {}",
            scoped_name,
            e
        );
    }

    // Also register in the specific schema if it's not default
    if !schema_val.is_empty() && schema_val != "public" && schema_val != "default" {
        if let Err(e) = source.register_with_schema(&state.ctx, &schema_val, &scoped_name).await {
             crate::app_log!("Failed to register YashanDB table in schema {}: {}", schema_val, e);
        } else {
             crate::app_log!("Registered YashanDB table in schema {}: {}", schema_val, scoped_name);
        }
    }

    // Add routing rule for schema.table -> scoped_name
    for rule in build_routing_rules(&schema_val, &payload.table_name) {
        query_rewriter::set_routing_rule(rule.clone(), scoped_name.clone());
        crate::app_log!("Added routing rule: {} -> {}", rule, scoped_name);
    }

    match source.register(&state.ctx).await {
        Ok(_) => {
            // 3. Persist Metadata
            // Collect Stats
            let stats_json = match source.get_table_stats() {
                Ok((num_rows, avg_len)) => {
                    if num_rows.is_some() || avg_len.is_some() {
                        Some(
                            serde_json::json!({
                               "num_rows": num_rows,
                               "avg_row_len": avg_len
                            })
                            .to_string(),
                        )
                    } else {
                        None
                    }
                }
                Err(e) => {
                    crate::app_log!(
                        "Failed to collect stats for YashanDB table {}: {}",
                        scoped_name,
                        e
                    );
                    None
                }
            };

            let _ = state
                .metadata_manager
                .register_table(
                    &state.ctx,
                    "datafusion",
                    &schema_val,
                    &scoped_name,
                    &config_str,
                    "yashandb",
                    Some(payload.table_name),
                    stats_json,
                )
                .await;

            crate::app_log!(
                "Registered YashanDB table: {} (Schema: {})",
                scoped_name,
                schema_val
            );
            Json(serde_json::json!({ "status": "ok", "message": "YashanDB table registered" }))
        }
        Err(e) => {
            crate::app_log!("Failed to register YashanDB table: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
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
    status: String, // Added status field
    plan_json: serde_json::Value,
    physical_plan_text: String,
    cost_est: f64,
    estimated_rows: Option<usize>,
    estimated_bytes: Option<usize>,
    warnings: Vec<String>,
    message: Option<String>, // Added for error messages
}

#[derive(Deserialize)]
struct RoutingConfig {
    rules: HashMap<String, String>,
}

async fn update_routing_config(Json(payload): Json<RoutingConfig>) -> Json<serde_json::Value> {
    for (table, target) in payload.rules {
        query_rewriter::set_routing_rule(table.clone(), target.clone());
        crate::app_log!("Updated routing rule: {} -> {}", table, target);
    }
    Json(serde_json::json!({ "status": "ok", "message": "Routing config updated" }))
}

#[derive(Deserialize)]
struct ExecuteRequest {
    sql: String,
    params: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ExecuteResponse {
    status: String, // Added status field
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    execution_time_ms: u64,
    error: Option<String>,
    message: Option<String>,
}

#[cfg(test)]
mod routing_rule_tests {
    use super::{build_routing_rules, build_scoped_name, resolve_schema_and_table};

    #[test]
    fn build_routing_rules_with_schema() {
        let rules = build_routing_rules("public", "orders");
        assert_eq!(
            rules,
            vec!["public.orders".to_string(), "orders".to_string()]
        );
    }

    #[test]
    fn build_routing_rules_without_schema() {
        let rules = build_routing_rules("", "orders");
        assert_eq!(rules, vec!["orders".to_string()]);
    }

    #[test]
    fn build_routing_rules_with_qualified_table() {
        let rules = build_routing_rules("public", "zx_admin.orders");
        assert_eq!(
            rules,
            vec!["zx_admin.orders".to_string(), "orders".to_string()]
        );
    }

    #[test]
    fn resolve_schema_from_payload() {
        let (schema, table, known) =
            resolve_schema_and_table(Some("ZX_ADMIN".to_string()), "TB_SYS_USER", "public");
        assert_eq!(schema, "ZX_ADMIN");
        assert_eq!(table, "TB_SYS_USER");
        assert!(known);
    }

    #[test]
    fn resolve_schema_from_table_name() {
        let (schema, table, known) =
            resolve_schema_and_table(None, "ZX_ADMIN.TB_SYS_USER", "public");
        assert_eq!(schema, "ZX_ADMIN");
        assert_eq!(table, "TB_SYS_USER");
        assert!(known);
    }

    #[test]
    fn resolve_schema_fallback_default() {
        let (schema, table, known) = resolve_schema_and_table(None, "orders", "public");
        assert_eq!(schema, "public");
        assert_eq!(table, "orders");
        assert!(!known);
    }

    #[test]
    fn scoped_name_includes_connection_hash() {
        let schema = "public";
        let table = "orders";
        let config_a = r#"{"host":"a","port":1688,"user":"u","pass":"p","service":"s"}"#;
        let config_b = r#"{"host":"b","port":1688,"user":"u","pass":"p","service":"s"}"#;

        let name_a = build_scoped_name("yashan", schema, table, config_a);
        let name_b = build_scoped_name("yashan", schema, table, config_b);

        assert_ne!(name_a, name_b);
        assert!(name_a.ends_with("_orders"));
        assert!(name_b.ends_with("_orders"));
    }
}

#[cfg(test)]
mod session_state_tests {
    use super::build_session_context;
    use datafusion::execution::runtime_env::RuntimeEnvBuilder;
    use datafusion::prelude::SessionConfig;
    use std::sync::Arc;

    #[test]
    fn default_functions_registered() {
        let runtime_env = RuntimeEnvBuilder::new().build().unwrap();
        let ctx = build_session_context(SessionConfig::new(), Arc::new(runtime_env));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            ctx.state()
                .create_logical_plan("select count(*) from (values (1)) t(x)")
                .await
        });
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod api_removal_tests {
    use super::{build_app, build_session_context, metadata_manager::MetadataManager, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use datafusion::execution::runtime_env::RuntimeEnvBuilder;
    use datafusion::prelude::SessionConfig;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    fn build_test_state() -> Arc<AppState> {
        let runtime_env = RuntimeEnvBuilder::new().build().unwrap();
        let ctx = build_session_context(SessionConfig::new(), Arc::new(runtime_env));
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("metadata_test_{}.db", nanos));
        let db_path_str = db_path.to_string_lossy().to_string();
        let metadata_manager = MetadataManager::new(&db_path_str).unwrap();
        Arc::new(AppState {
            ctx,
            metadata_manager: Arc::new(metadata_manager),
        })
    }

    #[tokio::test]
    async fn auto_link_scan_route_removed() {
        let app = build_app(build_test_state(), ".");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auto_link_scan")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "变更备注 2026-02-27: auto_link_scan 已移除，仅保留其他API，防止旧元数据清理逻辑误用"
        );
    }
}

use crate::datasources::yashandb::{get_cache_filename, YashanTable};

#[derive(Deserialize)]
struct LinkIdenticalTableRequest {
    source_table: String,
    target_table: String,
}

async fn link_identical_tables(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LinkIdenticalTableRequest>,
) -> Json<serde_json::Value> {
    crate::app_log!(
        "Consistency Check: Comparing {} vs {}",
        payload.source_table,
        payload.target_table
    );

    // 1. Check Consistency (EXCEPT)
    // Use CAST to ensure type compatibility if needed, but for now rely on identical schema assumption
    let check_sql = format!(
        "SELECT count(*) FROM (SELECT * FROM {} EXCEPT SELECT * FROM {})",
        payload.source_table, payload.target_table
    );

    let df = match state.ctx.sql(&check_sql).await {
        Ok(df) => df,
        Err(e) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("SQL Parse Error: {}", e) }),
            )
        }
    };

    let batches = match df.collect().await {
        Ok(b) => b,
        Err(e) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("Execution Error: {}", e) }),
            )
        }
    };

    if batches.is_empty() {
        return Json(
            serde_json::json!({ "status": "error", "message": "No result from check query" }),
        );
    }

    let count_val = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::Int64Array>()
        .map(|arr| arr.value(0))
        .unwrap_or(-1);

    if count_val != 0 {
        return Json(serde_json::json!({
            "status": "diff",
            "message": format!("Tables are not identical. Found {} differences.", count_val)
        }));
    }

    crate::app_log!("Consistency Check Passed. Linking tables...");

    // 2. Link / Clone
    // Get Target Table Provider to get conn_str
    let provider = match state.ctx.table_provider(&payload.target_table).await {
        Ok(p) => p,
        Err(_) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("Target table {} not found", payload.target_table) }),
            )
        }
    };

    // Downcast to YashanTable to get connection info
    let yashan_table = match provider.as_any().downcast_ref::<YashanTable>() {
        Some(t) => t,
        None => {
            return Json(
                serde_json::json!({ "status": "error", "message": "Target table is not a YashanDB table" }),
            )
        }
    };

    let conn_str = yashan_table.conn_str().to_string();
    // We use the raw table name from the struct, assuming it matches what was registered or what we want to cache
    let table_name = yashan_table.name().to_string();

    let cache_filename = get_cache_filename(&table_name, &conn_str);
    let cache_dir = std::path::Path::new("cache/yashandb");
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(cache_dir);
    }
    let cache_path = cache_dir.join(&cache_filename);
    let cache_path_str = cache_path.to_string_lossy().to_string();

    crate::app_log!("Writing Source Data to Target Cache: {}", cache_path_str);

    // Write Source to Target Cache
    // We use datafusion's write_parquet
    let source_df = match state
        .ctx
        .sql(&format!("SELECT * FROM {}", payload.source_table))
        .await
    {
        Ok(df) => df,
        Err(e) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("Failed to read source: {}", e) }),
            )
        }
    };

    // Use default parquet options
    match source_df
        .write_parquet(&cache_path_str, DataFrameWriteOptions::default(), None)
        .await
    {
        Ok(_) => {
            crate::app_log!(
                "Link Successful. {} is now served from local cache.",
                payload.target_table
            );
            Json(serde_json::json!({
                "status": "success",
                "message": format!("Tables verified identical. Linked {} to local cache: {}", payload.target_table, cache_path_str),
                "cache_path": cache_path_str
            }))
        }
        Err(e) => Json(
            serde_json::json!({ "status": "error", "message": format!("Failed to write cache: {}", e) }),
        ),
    }
}

async fn execute_sql(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    let mut sql = payload.sql;

    // Replace parameters if provided
    if let Some(params) = payload.params {
        for (key, value) in params {
            let placeholder = format!(":{}", key);
            let is_number = value.parse::<f64>().is_ok();
            let replacement = if is_number || (value.starts_with('\'') && value.ends_with('\'')) {
                value
            } else {
                format!("'{}'", value)
            };
            sql = sql.replace(&placeholder, &replacement);
        }
    }

    let start = std::time::Instant::now();

    crate::app_log!("Executing SQL: {}", sql);

    // Rewrite Query
    let final_sql = match query_rewriter::rewrite_query(
        &state.ctx,
        Some(&state.metadata_manager),
        &sql,
    )
    .await
    {
        Ok(rewritten) => {
            if rewritten != sql {
                crate::app_log!("Rewritten SQL: {}", rewritten);
            }
            rewritten
        }
        Err(e) => {
            crate::app_log!("Rewrite Error: {}", e);
            sql.clone()
        }
    };

    // --- Single Source Pushdown Optimization ---
    // Try Full Pushdown first
    let pushdown_provider = 'pushdown: {
        // 1. Create Logical Plan to identify tables
        // We use state.ctx.state() to access the SessionState
        let plan = match state.ctx.state().create_logical_plan(&final_sql).await {
            Ok(p) => p,
            Err(e) => {
                crate::app_log!(
                    "Pushdown Analysis Failed: Logical plan creation failed: {}",
                    e
                );
                break 'pushdown None;
            }
        };

        // 2. Collect all tables involved
        fn collect_tables(plan: &LogicalPlan, tables: &mut Vec<String>) {
            match plan {
                LogicalPlan::TableScan(scan) => {
                    tables.push(scan.table_name.to_string());
                }
                _ => {
                    for child in plan.inputs() {
                        collect_tables(child, tables);
                    }
                }
            }
        }

        let mut table_names = Vec::new();
        collect_tables(&plan, &mut table_names);

        if table_names.is_empty() {
            crate::app_log!("Pushdown Analysis Failed: No tables found in query.");
            break 'pushdown None;
        }

        // 3. Verify if all tables belong to the same YashanDB connection
        let all_tables = match state.metadata_manager.list_tables() {
            Ok(t) => t,
            Err(e) => {
                crate::app_log!(
                    "Pushdown Analysis Failed: Failed to list metadata tables: {}",
                    e
                );
                break 'pushdown None;
            }
        };

        let mut common_config: Option<String> = None;
        let mut common_source_type: Option<String> = None;
        let mut all_match = true;

        for name in &table_names {
            // Find metadata for this table
            let clean_name = name.replace("\"", ""); // Remove quotes
            let parts: Vec<&str> = clean_name.split('.').collect();
            let table_part = parts.last().unwrap_or(&"");

            let meta = all_tables
                .iter()
                .find(|t| t.table_name.eq_ignore_ascii_case(table_part));

            match meta {
                Some(t) => {
                    if t.source_type != "yashandb" && t.source_type != "oracle" {
                        crate::app_log!("Pushdown Analysis Failed: Table {} is not YashanDB or Oracle (type: {})", name, t.source_type);
                        all_match = false;
                        break;
                    }

                    if let Some(ref st) = common_source_type {
                        if st != &t.source_type {
                            crate::app_log!(
                                "Pushdown Analysis Failed: Mixed source types: {} vs {}",
                                st,
                                t.source_type
                            );
                            all_match = false;
                            break;
                        }
                    } else {
                        common_source_type = Some(t.source_type.clone());
                    }

                    if let Some(ref cfg) = common_config {
                        if cfg != &t.file_path {
                            crate::app_log!("Pushdown Analysis Failed: Table {} has different config than others", name);
                            all_match = false;
                            break;
                        }
                    } else {
                        common_config = Some(t.file_path.clone());
                    }
                }
                Option::None => {
                    crate::app_log!(
                        "Pushdown Analysis Failed: Table {} not found in metadata (looked for {})",
                        name,
                        table_part
                    );
                    // Print available tables for debugging
                    let available: Vec<String> =
                        all_tables.iter().map(|t| t.table_name.clone()).collect();
                    crate::app_log!("Available tables: {:?}", available);
                    all_match = false;
                    break;
                }
            }
        }

        if !all_match {
            break 'pushdown None;
        }

        // 4. If we are here, all tables are from same source
        if let Some(config) = common_config {
            let source_type = common_source_type.unwrap_or_default();

            if source_type == "yashandb" {
                // Check if all tables are cached locally
                let mut all_cached = true;

                // Reconstruct connection string for hashing (same logic as below)
                let conn_str_for_hash = if let Ok(parsed) =
                    serde_json::from_str::<serde_json::Value>(&config)
                {
                    let host = parsed["host"].as_str().unwrap_or("");
                    let port = parsed["port"].as_u64().unwrap_or(1688) as u16;
                    let user = parsed["user"].as_str().unwrap_or("");
                    let pass = parsed["pass"].as_str().unwrap_or("");
                    let service = parsed["service"].as_str().unwrap_or("");
                    datasources::yashandb::build_yashandb_conn_str(host, port, user, pass, service)
                } else {
                    config.clone()
                };

                for name in &table_names {
                    let clean_name = name.replace("\"", "").to_lowercase();
                    let safe_table = clean_name.replace(|c: char| !c.is_alphanumeric(), "_");

                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    conn_str_for_hash.hash(&mut hasher);
                    let conn_hash = hasher.finish();

                    let filename = format!("{}_{}.parquet", safe_table, conn_hash);
                    let cache_path = std::path::Path::new("cache/yashandb").join(&filename);

                    if !cache_path.exists()
                        || std::fs::metadata(&cache_path)
                            .map(|m| m.len() == 0)
                            .unwrap_or(true)
                    {
                        all_cached = false;
                        break;
                    }
                }

                if all_cached {
                    crate::app_log!("Single Source Pushdown Optimization Skipped: All tables cached locally. Using local execution.");
                    break 'pushdown None;
                }

                crate::app_log!("Single Source Pushdown Optimization: All tables from same YashanDB source. Pushing down SQL.");

                // 4.1 Rewrite SQL to use Remote Table Names
                let mut remote_sql = final_sql.clone();
                for name in &table_names {
                    // name is the table name as found in the plan (e.g. "yashan_tpcc_..."). 
                    // It might be unquoted in the plan table_names list, but quoted in SQL.
                    let clean_name = name.replace("\"", "");
                    
                    // Find metadata to get remote name
                    let parts: Vec<&str> = clean_name.split('.').collect();
                    let table_part = parts.last().unwrap_or(&"");
                    
                    if let Some(meta) = all_tables.iter().find(|t| t.table_name.eq_ignore_ascii_case(table_part)) {
                         let remote_name = meta.sheet_name.clone().unwrap_or(meta.table_name.clone());
                         
                         // Fix: Prepend schema if remote_name doesn't have it and schema is relevant
                         let final_remote_name = if !remote_name.contains('.') && !meta.schema_name.is_empty() && meta.schema_name != "public" && meta.schema_name != "default" {
                             format!("{}.{}", meta.schema_name, remote_name)
                         } else {
                             remote_name
                         };
                         
                         // Replace fully qualified usages in SQL
                         // 1. "datafusion"."schema"."physical"
                         // 2. "schema"."physical"
                         // 3. "physical"
                         // We use simple string replacement for now, assuming physical names are unique (hashes)
                         
                         // Handle Quotes in SQL: "datafusion"."TPCC"."yashan_..."
                         // We construct the likely FQN string in SQL
                         let schema = if meta.schema_name.is_empty() { "public".to_string() } else { meta.schema_name.clone() };
                         
                         let fqn_quoted = format!("\"datafusion\".\"{}\".\"{}\"", schema, meta.table_name);
                         let schema_quoted = format!("\"{}\".\"{}\"", schema, meta.table_name);
                         let table_quoted = format!("\"{}\"", meta.table_name);
                         
                         if remote_sql.contains(&fqn_quoted) {
                              remote_sql = remote_sql.replace(&fqn_quoted, &final_remote_name);
                         } else if remote_sql.contains(&schema_quoted) {
                              remote_sql = remote_sql.replace(&schema_quoted, &final_remote_name);
                         } else {
                              remote_sql = remote_sql.replace(&table_quoted, &final_remote_name);
                              // Also try unquoted if it exists
                              remote_sql = remote_sql.replace(&meta.table_name, &final_remote_name);
                         }
                     }
                }
                
                crate::app_log!("Rewritten Remote SQL for Pushdown: {}", remote_sql);

                // Trigger Sidecar Caching for all involved tables
                let mut unique_tables = table_names.clone();
                unique_tables.sort();
                unique_tables.dedup();

                for name in unique_tables {
                    // Clean name (remove quotes if any)
                    let clean_name = name.replace("\"", "").to_lowercase();

                    // Get schema from registered table
                    if let Ok(df) = state.ctx.table(&name).await {
                        let arrow_schema = df.schema().inner().as_ref().clone();
                        let schema = Arc::new(arrow_schema);

                        // Parse JSON config to construct ODBC connection string for background task
                        let conn_str = if let Ok(parsed) =
                            serde_json::from_str::<serde_json::Value>(&config)
                        {
                            let host = parsed["host"].as_str().unwrap_or("");
                            let port = parsed["port"].as_u64().unwrap_or(1688) as u16;
                            let user = parsed["user"].as_str().unwrap_or("");
                            let pass = parsed["pass"].as_str().unwrap_or("");
                            let service = parsed["service"].as_str().unwrap_or("");
                            datasources::yashandb::build_yashandb_conn_str(
                                host, port, user, pass, service,
                            )
                        } else {
                            config.clone()
                        };

                        let physical_table_name =
                            if let Ok(tables) = state.metadata_manager.list_tables() {
                                tables
                                    .iter()
                                    .find(|t| t.table_name == clean_name)
                                    .and_then(|t| t.sheet_name.clone())
                                    .unwrap_or_else(|| clean_name.clone())
                            } else {
                                clean_name.clone()
                            };

                        datasources::yashandb::trigger_background_caching(
                            conn_str,
                            clean_name,
                            physical_table_name,
                            schema,
                        );
                    }
                }

                // Create Pushdown Provider
                match datasources::yashandb::YashanDataSource::create_pushdown_provider(
                    &config,
                    remote_sql.clone(),
                )
                .await
                {
                    Ok(p) => Some(p),
                    Err(e) => {
                        crate::app_log!("YashanDB Pushdown creation failed: {}", e);
                        None
                    }
                }
            } else if source_type == "oracle" {
                crate::app_log!("Single Source Pushdown Optimization: All tables from same Oracle source. Pushing down SQL.");

                #[cfg(feature = "oracle")]
                {
                    match datasources::oracle::OracleDataSource::create_pushdown_provider(
                        config,
                        final_sql.clone(),
                    )
                    .await
                    {
                        Ok(p) => Some(p),
                        Err(e) => {
                            crate::app_log!("Oracle Pushdown creation failed: {}", e);
                            None
                        }
                    }
                }
                #[cfg(not(feature = "oracle"))]
                {
                    crate::app_log!("Oracle feature not enabled");
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    // --- Partial Pushdown Optimization (Experimental) ---
    // If full pushdown failed, try partial pushdown (pushing down Joins of same-source tables)
    // This is a simplified heuristic: if we have a Join where both sides are from same source,
    // we assume we can push it down. But DataFusion LogicalPlan replacement is complex.
    // For now, we will stick to Full Pushdown or Local Execution to ensure stability.
    // Implementing robust Partial Pushdown requires walking the plan and rewriting subtrees,
    // which is better done as a proper OptimizerRule in future refactoring.

    // Create DataFrame
    let df_result = if let Some(provider) = pushdown_provider {
        // Use a unique name for the pushdown result table
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp_name = format!("pushdown_{}", timestamp);
        if let Err(e) = state.ctx.register_table(&tmp_name, provider) {
            crate::app_log!("Failed to register pushdown table: {}", e);
            state.ctx.sql(&final_sql).await
        } else {
            let res = state.ctx.table(&tmp_name).await;
            // We should unregister later, but for now it's fine (session scoped?)
            // Actually, DataFusion ctx is shared in AppState. We MUST unregister.
            // But we can't unregister easily after returning.
            // However, reusing the same context might clutter it.
            // Given the architecture, maybe we shouldn't worry about cleanup too much for now, or use a separate context?
            // Using unique name prevents collision.
            res
        }
    } else {
        state.ctx.sql(&final_sql).await
    };

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
                    crate::app_log!("Detected missing field '{}', attempting fix...", field);
                    match query_rewriter::fix_query(&state.ctx, &final_sql, &field).await {
                        Ok(fixed_sql) => {
                            crate::app_log!("Retrying with fixed SQL: {}", fixed_sql);
                            state.ctx.sql(&fixed_sql).await
                        }
                        Err(_) => Err(e), // Return original error if fix fails
                    }
                } else {
                    Err(e)
                }
            } else if err_msg.contains("table") && err_msg.contains("not found") {
                let mut attempt = 0usize;
                let max_attempts = 8usize;
                let mut current_err = e;

                loop {
                    let current_msg = current_err.to_string();
                    if !(current_msg.contains("table") && current_msg.contains("not found")) {
                        break Err(current_err);
                    }

                    let Some(missing_table) = extract_missing_table_name(&current_msg) else {
                        break Err(current_err);
                    };

                    crate::app_log!(
                        "Detected missing table '{}', attempting auto-registration... ({}/{})",
                        missing_table,
                        attempt + 1,
                        max_attempts
                    );

                    match auto_register_missing_oracle_table(&state, &missing_table).await {
                        Ok(Some(scoped_name)) => {
                            crate::app_log!(
                                "Auto-registered missing table '{}' as '{}', retrying query",
                                missing_table,
                                scoped_name
                            );
                        }
                        Ok(None) => break Err(current_err),
                        Err(reg_err) => {
                            crate::app_log!(
                                "Auto-registration failed for '{}': {}",
                                missing_table,
                                reg_err
                            );
                            break Err(current_err);
                        }
                    }

                    attempt += 1;
                    if attempt >= max_attempts {
                        crate::app_log!(
                            "Auto-registration reached max attempts ({}) for query",
                            max_attempts
                        );
                        break Err(current_err);
                    }

                    match state.ctx.sql(&final_sql).await {
                        Ok(df) => break Ok(df),
                        Err(next_err) => current_err = next_err,
                    }
                }
            } else {
                Err(e)
            }
        }
    };

    match df_result {
        Ok(df) => {
            // Streaming Execution Logic
            // User Request: "Display first batch immediately, process rest in background"
            match df.execute_stream().await {
                Ok(mut stream) => {
                    let mut initial_batches = Vec::new();

                    // Fetch just the first batch for preview
                    match stream.next().await {
                        Some(Ok(batch)) => {
                            initial_batches.push(batch);
                        }
                        Some(Err(e)) => {
                            let err_msg = format!("External error: {}", e);
                            crate::app_log!("{}", err_msg);
                            return Json(ExecuteResponse {
                                status: "error".to_string(),
                                columns: vec![],
                                rows: vec![],
                                execution_time_ms: start.elapsed().as_millis() as u64,
                                error: Some(err_msg),
                                message: None,
                            });
                        }
                        None => {} // Empty result
                    }

                    let duration = start.elapsed();

                    if initial_batches.is_empty() {
                        return Json(ExecuteResponse {
                            status: "ok".to_string(),
                            columns: vec![],
                            rows: vec![],
                            execution_time_ms: duration.as_millis() as u64,
                            error: None,
                            message: None,
                        });
                    }

                    // Format Output (from initial batches only)
                    let schema = initial_batches[0].schema();
                    let columns: Vec<String> =
                        schema.fields().iter().map(|f| f.name().clone()).collect();

                    let mut rows = Vec::new();
                    for batch in initial_batches {
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

                    crate::app_log!(
                        "Query preview returned successfully. {} rows returned in {}ms",
                        rows.len(),
                        duration.as_millis()
                    );

                    Json(ExecuteResponse {
                        status: "ok".to_string(),
                        columns,
                        rows,
                        execution_time_ms: duration.as_millis() as u64,
                        error: None,
                        message: Some("Preview Mode: First batch loaded.".to_string()),
                    })
                }
                Err(e) => {
                    crate::app_log!("Execution Error: {}", e);
                    Json(ExecuteResponse {
                        status: "error".to_string(),
                        columns: vec![],
                        rows: vec![],
                        execution_time_ms: start.elapsed().as_millis() as u64,
                        error: Some(e.to_string()),
                        message: None,
                    })
                }
            }
        }
        Err(e) => {
            crate::app_log!("Planning Error: {}", e);
            Json(ExecuteResponse {
                status: "error".to_string(),
                columns: vec![],
                rows: vec![],
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                message: None,
            })
        }
    }
}

async fn upload_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<serde_json::Value> {
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return Json(serde_json::json!({ "status": "error", "message": "No file" }));
        }
        Err(e) => {
            return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
        }
    };
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

    crate::app_log!("File uploaded: {}", file_name);

    // Generate Table Name
    // Remove extension
    let raw_name = std::path::Path::new(&file_name)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    // Sanitize table name: replace . - ( ) space with _
    let table_name = raw_name
        .chars()
        .map(|c| {
            if matches!(c, '.' | '-' | ' ' | '(' | ')') {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();

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
                    crate::app_log!(
                        "Transcoding failed for {}, falling back to CSV: {}",
                        file_name,
                        e
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
            match CacheManager::ensure_parquet_cache(&path_str, "excel", Some(sheet.clone())).await
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
                    crate::app_log!(
                        "Transcoding failed for {}, falling back to Excel: {}",
                        file_name,
                        e
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
            crate::app_log!("Auto-registered uploaded data source: {}", source.name());

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
                    None,
                )
                .await
            {
                crate::app_log!("Failed to persist metadata for table {}: {}", table_name, e);
            } else {
                crate::app_log!("Persisted metadata for table: {}", table_name);
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

    Json(
        serde_json::json!({ "status": "ok", "message": format!("Uploaded {} (not registered, unsupported type)", file_name) }),
    )
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
                crate::app_log!(
                    "Transcoding failed for {}, using original Excel: {}",
                    payload.table_name,
                    e
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
                crate::app_log!(
                    "Transcoding failed for {}, using original CSV: {}",
                    payload.table_name,
                    e
                );
                Some(Box::new(CsvDataSource::new(
                    payload.table_name.clone(),
                    payload.file_path.clone(),
                )))
            }
        }
    } else if payload.source_type == "sqlite" {
        // let internal_name = payload.sheet_name.clone().unwrap_or(payload.table_name.clone());
        // Some(Box::new(SqliteDataSource::new(payload.table_name.clone(), payload.file_path.clone(), internal_name)))
        None
    } else {
        None
    };

    if let Some(source) = source {
        if source.register(&state.ctx).await.is_ok() {
            crate::app_log!("Manually registered data source: {}", source.name());

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
                    None,
                )
                .await
            {
                crate::app_log!(
                    "Failed to persist metadata for table {}: {}",
                    payload.table_name,
                    e
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
    let final_sql = match query_rewriter::rewrite_query(
        &state.ctx,
        Some(&state.metadata_manager),
        &sql,
    )
    .await
    {
        Ok(rewritten) => rewritten,
        Err(_) => sql.clone(),
    };

    // --- Parameter Injection (Match execute_sql logic) ---
    // (Removed)

    // --- Single Source Pushdown Optimization ---
    // Try Full Pushdown first
    let pushdown_provider = 'pushdown: {
        // 1. Create Logical Plan to identify tables
        let plan = match state.ctx.state().create_logical_plan(&final_sql).await {
            Ok(p) => p,
            Err(_) => break 'pushdown None, // Ignore error here, let main flow handle it
        };

        // 2. Collect all tables involved
        fn collect_tables(plan: &LogicalPlan, tables: &mut Vec<String>) {
            match plan {
                LogicalPlan::TableScan(scan) => {
                    tables.push(scan.table_name.to_string());
                }
                _ => {
                    for child in plan.inputs() {
                        collect_tables(child, tables);
                    }
                }
            }
        }

        let mut table_names = Vec::new();
        collect_tables(&plan, &mut table_names);

        if table_names.is_empty() {
            break 'pushdown None;
        }

        // 3. Verify if all tables belong to the same connection (YashanDB or Oracle)
        let all_tables = match state.metadata_manager.list_tables() {
            Ok(t) => t,
            Err(_) => break 'pushdown None,
        };

        let mut common_config: Option<String> = None;
        let mut common_source_type: Option<String> = None;
        let mut all_match = true;

        for name in &table_names {
            // Find metadata for this table
            let clean_name = name.replace("\"", ""); // Remove quotes
            let parts: Vec<&str> = clean_name.split('.').collect();
            let table_part = parts.last().unwrap_or(&"");

            let meta = all_tables
                .iter()
                .find(|t| t.table_name.eq_ignore_ascii_case(table_part));

            match meta {
                Some(t) => {
                    // Check supported types
                    let is_yashan = t.source_type == "yashandb";
                    let is_oracle = t.source_type == "oracle";

                    if !is_yashan && !is_oracle {
                        all_match = false;
                        break;
                    }

                    if let Some(ref st) = common_source_type {
                        if st != &t.source_type {
                            all_match = false; // Mixing sources
                            break;
                        }
                    } else {
                        common_source_type = Some(t.source_type.clone());
                    }

                    if let Some(ref cfg) = common_config {
                        if cfg != &t.file_path {
                            all_match = false;
                            break;
                        }
                    } else {
                        common_config = Some(t.file_path.clone());
                    }
                }
                None => {
                    all_match = false;
                    break;
                }
            }
        }

        if !all_match {
            break 'pushdown None;
        }

        // 4. If we are here, all tables are from same connection
        if let Some(config) = common_config {
            if let Some(source_type) = common_source_type {
                // Optimization: If all tables are cached locally, use local execution (skip pushdown)
                if source_type == "yashandb" {
                    let mut all_cached = true;

                    // Reconstruct connection string for hashing
                    let conn_str_for_hash =
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&config) {
                            let host = parsed["host"].as_str().unwrap_or("");
                            let port = parsed["port"].as_u64().unwrap_or(1688);
                            let user = parsed["user"].as_str().unwrap_or("");
                            let pass = parsed["pass"].as_str().unwrap_or("");
                            format!(
                                "Driver=YashanDB;Server={};Port={};Uid={};Pwd={};",
                                host, port, user, pass
                            )
                        } else {
                            config.clone()
                        };

                    for name in &table_names {
                        let clean_name = name.replace("\"", "").to_lowercase();
                        let safe_table = clean_name.replace(|c: char| !c.is_alphanumeric(), "_");

                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        conn_str_for_hash.hash(&mut hasher);
                        let conn_hash = hasher.finish();

                        let filename = format!("{}_{}.parquet", safe_table, conn_hash);
                        let cache_path = std::path::Path::new("cache/yashandb").join(&filename);

                        if !cache_path.exists()
                            || std::fs::metadata(&cache_path)
                                .map(|m| m.len() == 0)
                                .unwrap_or(true)
                        {
                            all_cached = false;
                            break;
                        }
                    }

                    if all_cached {
                        // Use local execution (Plan will use ParquetExec directly)
                        break 'pushdown None;
                    }
                }

                // Rewrite SQL with physical table names (schema.table)
                let mut table_map = std::collections::HashMap::new();
                for name in &table_names {
                    let clean_name = name.replace("\"", "");
                    let parts: Vec<&str> = clean_name.split('.').collect();
                    let table_part = parts.last().unwrap_or(&"");

                    if let Some(t) = all_tables
                        .iter()
                        .find(|t| t.table_name.eq_ignore_ascii_case(table_part))
                    {
                        // Use sheet_name as physical table name if available, else table_name
                        let physical_table = t.sheet_name.as_ref().unwrap_or(&t.table_name);
                        let schema = &t.schema_name;

                        let full_name = if schema == "public" {
                            physical_table.clone()
                        } else {
                            format!("{}.{}", schema, physical_table)
                        };

                        table_map.insert(clean_name.clone(), full_name.clone());
                        table_map.insert(table_part.to_string(), full_name);
                    }
                }

                let pushdown_sql =
                    query_rewriter::rewrite_with_physical_tables(&final_sql, &table_map)
                        .unwrap_or(final_sql.clone());

                match source_type.as_str() {
                    "yashandb" => {
                        datasources::yashandb::YashanDataSource::create_pushdown_provider(
                            &config,
                            pushdown_sql,
                        )
                        .await
                        .ok()
                    }
                    #[cfg(feature = "oracle")]
                    "oracle" => datasources::oracle::OracleDataSource::create_pushdown_provider(
                        config,
                        pushdown_sql,
                    )
                    .await
                    .ok(),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    // 1. Create Logical Plan
    let mut plan_result = if let Some(provider) = pushdown_provider {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp_name = format!("pushdown_plan_{}", timestamp);
        if state.ctx.register_table(&tmp_name, provider).is_ok() {
            state
                .ctx
                .state()
                .create_logical_plan(&format!("SELECT * FROM {}", tmp_name))
                .await
        } else {
            state.ctx.state().create_logical_plan(&final_sql).await
        }
    } else {
        state.ctx.state().create_logical_plan(&final_sql).await
    };

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

                    #[allow(deprecated)]
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

                    // Use rows as cost estimate if available
                    let cost_est = estimated_rows.map(|r| r as f64).unwrap_or(0.0);

                    Json(PlanResponse {
                        status: "ok".to_string(),
                        plan_json,
                        physical_plan_text: plan_text,
                        cost_est,
                        estimated_rows,
                        estimated_bytes,
                        warnings: vec![],
                        message: None,
                    })
                }
                Err(e) => Json(PlanResponse {
                    status: "error".to_string(),
                    plan_json: serde_json::json!({}),
                    physical_plan_text: format!("Physical Plan Error: {}", e),
                    cost_est: 0.0,
                    estimated_rows: None,
                    estimated_bytes: None,
                    warnings: vec![e.to_string()],
                    message: Some(e.to_string()),
                }),
            }
        }
        Err(e) => Json(PlanResponse {
            status: "error".to_string(),
            plan_json: serde_json::json!({}),
            physical_plan_text: format!("Logical Plan Error: {}", e),
            cost_est: 0.0,
            estimated_rows: None,
            estimated_bytes: None,
            warnings: vec![e.to_string()],
            message: Some(e.to_string()),
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
    crate::app_log!("Warning: users.xlsx missing. Please provide it.");
}

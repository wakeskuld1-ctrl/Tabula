pub mod api;
#[cfg(test)]
pub mod cache_e2e_test;
pub mod cache_manager;
#[cfg(test)]
pub mod cache_stress_test;
pub mod datasources;
pub mod metadata_manager;
pub mod query_rewriter;
pub mod services;
pub mod session_manager;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    routing::{get, post},
    Json, Router,
};
use datafusion::prelude::*;
use metadata_manager::MetadataManager;
use session_manager::SessionManager;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

// Global App State
pub struct AppState {
    pub ctx: SessionContext,
    pub metadata_manager: Arc<MetadataManager>,
    pub session_manager: Arc<SessionManager>,
    pub logs: Arc<RwLock<VecDeque<String>>>,
}

// Helper to add log
pub fn add_log(logs: &Arc<RwLock<VecDeque<String>>>, msg: String) {
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

pub fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace("\"", "\"\""))
}

pub async fn create_app() -> Router {
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

    // Initialize Session Manager
    let session_manager = Arc::new(SessionManager::new(data_path_str, metadata_manager.clone()));
    session_manager.start_auto_flush();

    // Create data dir
    let data_dir = std::path::Path::new(data_path_str);
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).unwrap();
    }

    // Start Cache Maintenance
    cache_manager::CacheManager::start_maintenance_task();

    // Shared State
    let logs = Arc::new(RwLock::new(VecDeque::new()));
    let state = Arc::new(AppState {
        ctx,
        metadata_manager,
        session_manager,
        logs,
    });

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build Router
    Router::new()
        .route("/api/upload", post(api::upload_handler::upload_file))
        .route(
            "/api/register_table",
            post(api::register_handler::register_table_endpoint),
        )
        .route("/api/execute", post(api::execute_handler::execute_sql))
        .route("/api/tables", get(api::grid_handler::list_tables))
        .route("/api/plan", post(api::plan_handler::plan))
        .route("/api/metrics", get(api::health_handler::get_metrics))
        .route("/api/health", get(api::health_handler::health))
        .route("/api/grid-data", get(api::grid_handler::get_grid_data))
        .route("/api/update_cell", post(api::update_handler::update_cell))
        .route(
            "/api/batch_update_cells",
            post(api::update_handler::batch_update_cells),
        )
        .route("/api/update_style", post(api::update_handler::update_style))
        .route("/api/update_merge", post(api::update_handler::update_merge))
        .route(
            "/api/create_session",
            post(api::session_handler::create_session),
        )
        // - **2026-03-14**: Add sessions list/switch APIs for sandbox tabs.
        // - **Reason**: Frontend needs session list + active selection switching.
        .route("/api/sessions", get(api::session_handler::list_sessions))
        .route(
            "/api/switch_session",
            post(api::session_handler::switch_session),
        )
        .route(
            "/api/save_session",
            post(api::session_handler::save_session),
        )
        .route(
            "/api/delete_table",
            post(api::session_handler::delete_table),
        )
        .fallback_service(ServeDir::new(public_path_str).append_index_html_on_directories(true))
        .layer(cors)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .with_state(state)
}

pub async fn run() {
    let app = create_app().await;

    // Run Server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    println!("Backend server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

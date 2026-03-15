use axum::Json;
use serde_json::Value;

use crate::cache_manager;
use crate::services::health_service::build_health_payload;

pub async fn health() -> Json<Value> {
    Json(build_health_payload())
}

pub async fn get_metrics() -> Json<cache_manager::MetricsSnapshot> {
    Json(cache_manager::get_metrics_registry().snapshot())
}

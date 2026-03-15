use serde_json::Value;

pub fn build_health_payload() -> Value {
    serde_json::json!({ "status": "ok", "version": "0.1.0" })
}

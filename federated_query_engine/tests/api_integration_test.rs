use reqwest::{Client, StatusCode};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tabula_server::create_app;
use tokio::net::TcpListener;

async fn spawn_test_server() -> (Client, String) {
    let app = create_app().await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (Client::new(), format!("http://{}", addr))
}

fn unique_table_name(prefix: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", prefix, ts)
}

fn metadata_db_path() -> PathBuf {
    if Path::new("federated_query_engine").exists() {
        PathBuf::from("federated_query_engine/metadata.db")
    } else {
        PathBuf::from("metadata.db")
    }
}

async fn register_csv_table(
    client: &Client,
    base_url: &str,
    prefix: &str,
    csv_content: &str,
) -> String {
    let table_name = unique_table_name(prefix);
    let file_path = std::env::temp_dir().join(format!("{}.csv", table_name));

    tokio::fs::write(&file_path, csv_content).await.unwrap();

    let res = client
        .post(format!("{}/api/register_table", base_url))
        .json(&json!({
            "file_path": file_path.to_string_lossy().to_string(),
            "table_name": table_name,
            "sheet_name": null,
            "source_type": "csv",
            "header_rows": 0,
            "header_mode": "none"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    table_name
}

async fn create_session_and_get_id(client: &Client, base_url: &str, table_name: &str) -> String {
    let res = client
        .post(format!("{}/api/create_session", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_name": "integration_test_session"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    body["session"]["session_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_core_routes_health_tables_and_save_session() {
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .get(format!("{}/api/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .get(format!("{}/api/tables", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body.get("status").is_some());

    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_session_and_update_routes_error_path_smoke() {
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/create_session", base_url))
        .json(&json!({
            "table_name": "non_existent_table"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");

    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": "any_table",
            "session_id": "invalid_session",
            "row": 0,
            "col_name": "col1",
            "new_value": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");

    let res = client
        .post(format!("{}/api/batch_update_cells", base_url))
        .json(&json!({
            "table_name": "any_table",
            "session_id": "invalid_session",
            "updates": [
                { "row": 0, "col_name": "c1", "new_value": "v1" },
                { "row": 1, "col_name": "c1", "new_value": "v2" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");

    let res = client
        .post(format!("{}/api/update_style", base_url))
        .json(&json!({
            "table_name": "any_table",
            "row": 0,
            "col": 0,
            "style": { "bold": true, "format": "number" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");

    let res = client
        .post(format!("{}/api/delete_table", base_url))
        .json(&json!({
            "table_name": "non_existent_table"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body.get("status").is_some());

    let res = client
        .get(format!(
            "{}/api/grid-data?table_name=non_existent_table&page=1&page_size=20",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
}

#[tokio::test]
async fn test_unimplemented_routes_return_404() {
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(
        res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::METHOD_NOT_ALLOWED
    );

    let res = client
        .post(format!("{}/api/insert-column", base_url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(
        res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::METHOD_NOT_ALLOWED
    );

    let res = client
        .post(format!("{}/api/delete-column", base_url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(
        res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn test_positive_flow_create_update_save_and_read_consistency() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_create_update_save_read",
        "id,amount,name\n1,10,A\n2,20,B\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "row": 0,
            "col_name": "amount",
            "new_value": "99"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let updated_session_id = body["session_id"].as_str().unwrap().to_string();

    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .get(format!(
            "{}/api/grid-data?session_id={}&table_name={}&page=1&page_size=20",
            base_url, updated_session_id, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .post(format!("{}/api/execute", base_url))
        .json(&json!({
            "sql": format!(
                "SELECT \"amount\" FROM \"{}\" ORDER BY \"id\" ASC LIMIT 1",
                table_name
            )
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body["error"].is_null());
    let value = body["rows"][0][0].as_str().unwrap().parse::<f64>().unwrap();
    assert!((value - 99.0).abs() < 1e-9);
}

#[tokio::test]
async fn test_positive_flow_batch_update_mixed_types_and_sql_aggregation() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_batch_mixed_agg",
        "id,qty,price,name\n1,1,1.5,foo\n2,2,2.5,bar\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/batch_update_cells", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "updates": [
                { "row": 0, "col_name": "qty", "new_value": "10" },
                { "row": 1, "col_name": "qty", "new_value": "20" },
                { "row": 0, "col_name": "price", "new_value": "1.25" },
                { "row": 1, "col_name": "price", "new_value": "2.75" },
                { "row": 0, "col_name": "name", "new_value": "alpha" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let updated_session_id = body["session_id"].as_str().unwrap().to_string();

    let res = client
        .get(format!(
            "{}/api/grid-data?session_id={}&table_name={}&page=1&page_size=20",
            base_url, updated_session_id, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .post(format!("{}/api/execute", base_url))
        .json(&json!({
            "sql": format!(
                "SELECT SUM(\"qty\") AS total_qty, SUM(\"price\") AS total_price FROM \"{}\"",
                table_name
            )
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body["error"].is_null());
    let qty_sum = body["rows"][0][0].as_str().unwrap().parse::<f64>().unwrap();
    let price_sum = body["rows"][0][1].as_str().unwrap().parse::<f64>().unwrap();
    assert!((qty_sum - 30.0).abs() < 1e-9);
    assert!((price_sum - 4.0).abs() < 1e-9);

    let res = client
        .post(format!("{}/api/execute", base_url))
        .json(&json!({
            "sql": format!(
                "SELECT \"name\" FROM \"{}\" ORDER BY \"id\" ASC LIMIT 1",
                table_name
            )
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body["error"].is_null());
    assert_eq!(body["rows"][0][0], "alpha");
}

#[tokio::test]
async fn test_positive_flow_update_style_and_verify_metadata_persistence() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_style_metadata",
        "id,name\n1,A\n2,B\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/update_style", base_url))
        .json(&json!({
            "table_name": table_name,
            "row": 0,
            "col": 1,
            "style": {
                "bold": true,
                "color": "#ff0000",
                "format": "currency"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .get(format!(
            "{}/api/grid-data?session_id={}&table_name={}&page=1&page_size=20",
            base_url, session_id, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["metadata"]["styles"]["0,1"]["bold"], true);
    assert_eq!(body["metadata"]["styles"]["0,1"]["color"], "#ff0000");
    assert_eq!(body["metadata"]["styles"]["0,1"]["format"], "currency");

    let db_path = metadata_db_path();
    let conn = Connection::open(db_path).unwrap();
    let attr_value: String = conn
        .query_row(
            "SELECT attr_value FROM sheet_attributes WHERE session_id = ?1 AND cell_key = ?2 AND attr_type = ?3",
            (&session_id, "0,1", "style"),
            |row| row.get(0),
        )
        .unwrap();
    let persisted_style: Value = serde_json::from_str(&attr_value).unwrap();
    assert_eq!(persisted_style["bold"], true);
    assert_eq!(persisted_style["color"], "#ff0000");
    assert_eq!(persisted_style["format"], "currency");
}

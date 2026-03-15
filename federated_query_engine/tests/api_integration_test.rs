use reqwest::{Client, StatusCode};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
// **[2026-03-15]** Reason: tests poll for auto-flush version creation.
// **[2026-03-15]** Purpose: allow time-machine versions to materialize before checkout.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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

// **[2026-03-15]** Reason: auto-flush is asynchronous and versions may appear later.
// **[2026-03-15]** Purpose: poll versions until at least two are available or timeout.
async fn wait_for_versions(
    client: &Client,
    base_url: &str,
    table_name: &str,
    session_id: Option<&str>,
) -> Vec<Value> {
    // **[2026-03-15]** Reason: avoid flaky time-machine tests.
    // **[2026-03-15]** Purpose: give auto-flush time to persist changes.
    let start = Instant::now();
    let timeout = Duration::from_secs(8);
    loop {
        let url = match session_id {
            Some(sid) => format!(
                "{}/api/versions?table_name={}&session_id={}",
                base_url, table_name, sid
            ),
            None => format!("{}/api/versions?table_name={}", base_url, table_name),
        };

        let res = client.get(url).send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["status"], "ok");
        let versions = body["versions"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if versions.len() >= 2 || start.elapsed() >= timeout {
            return versions;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
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

// - **2026-03-14**: TDD for `/api/sessions` list behavior before implementation.
// - **Reason**: The frontend sheet tabs must be driven by session list; missing API should fail fast.
// - **Purpose**: Ensure created sessions are surfaced and the active session id is present.
// - **Scope**: Response shape + minimal data invariants (status, sessions array, active id).
#[tokio::test]
async fn test_sessions_list_returns_created_session_and_active_id() {
    // - **2026-03-14**: Use a real registered table to mirror production flow.
    // - **Reason**: Session listing is table-scoped; we need a valid table_name.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_sessions_list",
        "id,name\n1,A\n2,B\n",
    )
    .await;

    // - **2026-03-14**: Create one session to verify it appears in the list.
    // - **Reason**: The list should include sessions created via the API.
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // - **2026-03-14**: Call sessions list endpoint to validate response shape.
    // - **Reason**: This is the contract the frontend will depend on.
    let res = client
        .get(format!(
            "{}/api/sessions?table_name={}",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // - **2026-03-14**: Ensure sessions array contains the created session id.
    // - **Reason**: Frontend needs stable ids to drive selection.
    let sessions = body["sessions"].as_array().unwrap();
    assert!(sessions
        .iter()
        .any(|session| session["session_id"] == session_id));

    // - **2026-03-14**: Active session id must be returned and match the created session.
    // - **Reason**: UI needs to highlight the active tab deterministically.
    assert_eq!(body["active_session_id"], session_id);
}

// - **2026-03-14**: TDD for `/api/switch_session` behavior before implementation.
// - **Reason**: Users must switch between sandboxes; switching should update active session.
// - **Purpose**: Validate server returns ok and active session id changes accordingly.
// - **Scope**: Switch endpoint response + follow-up list verification.
#[tokio::test]
async fn test_switch_session_updates_active_session() {
    // - **2026-03-14**: Register a table so sessions can be created.
    // - **Reason**: Sessions are table-scoped and require a valid table_name.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_sessions_switch",
        "id,name\n1,A\n2,B\n",
    )
    .await;

    // - **2026-03-14**: Create two sessions to exercise switching logic.
    // - **Reason**: We need a non-trivial change in active session id.
    let first_session_id = create_session_and_get_id(&client, &base_url, &table_name).await;
    let res = client
        .post(format!("{}/api/create_session", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_name": "integration_test_session_2"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let second_session_id = body["session"]["session_id"].as_str().unwrap().to_string();

    // - **2026-03-14**: Switch to the second session.
    // - **Reason**: This is the primary UX action for sandbox tabs.
    let res = client
        .post(format!("{}/api/switch_session", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": second_session_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // - **2026-03-14**: Re-fetch sessions to verify active session id changed.
    // - **Reason**: Ensures switch has a durable effect, not just a response payload.
    let res = client
        .get(format!(
            "{}/api/sessions?table_name={}",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["active_session_id"], second_session_id);
    assert!(body["active_session_id"] != first_session_id);
}

#[tokio::test]
async fn test_unimplemented_routes_return_404() {
    let (client, base_url) = spawn_test_server().await;

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

// **[2026-03-15]** Reason: add /api/versions integration coverage in main repo.
// **[2026-03-15]** Purpose: validate optional session_id path and response shape.
#[tokio::test]
async fn test_versions_endpoint_with_and_without_session_id() {
    // **[2026-03-15]** Reason: prepare a table to generate versions.
    // **[2026-03-15]** Purpose: keep test data isolated and deterministic.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "versions",
        "id,amount\n1,10\n2,20\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // **[2026-03-15]** Reason: cover default path without session_id.
    // **[2026-03-15]** Purpose: ensure active session versions are returned.
    let res = client
        .get(format!(
            "{}/api/versions?table_name={}",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["versions"].is_array());

    // **[2026-03-15]** Reason: cover explicit session_id path.
    // **[2026-03-15]** Purpose: ensure targeted session history is read.
    let res = client
        .get(format!(
            "{}/api/versions?table_name={}&session_id={}",
            base_url, table_name, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["versions"].is_array());
}

// **[2026-03-15]** Reason: TDD for /api/checkout_version without session_id.
// **[2026-03-15]** Purpose: ensure data-only rollback works for active session.
// **[2026-03-15]** Scope: query-parameter API shape + data content verification.
#[tokio::test]
async fn test_checkout_version_without_session_id() {
    // **[2026-03-15]** Reason: isolate table for rollback verification.
    // **[2026-03-15]** Purpose: keep version history deterministic.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "checkout_no_sid",
        "id,name\n1,Alice\n",
    )
    .await;

    // **[2026-03-15]** Reason: create a session and make it active.
    // **[2026-03-15]** Purpose: ensure checkout falls back to active session.
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // **[2026-03-15]** Reason: mutate data to create a new version candidate.
    // **[2026-03-15]** Purpose: ensure rollback has an older value to restore.
    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "row": 0,
            "col_name": "name",
            "new_value": "Zed"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: trigger auto-flush sooner via pending write count.
    // **[2026-03-15]** Purpose: keep versions available within polling timeout.
    for _ in 0..19 {
        let res = client
            .post(format!("{}/api/update_cell", base_url))
            .json(&json!({
                "table_name": table_name,
                "session_id": session_id,
                "row": 0,
                "col_name": "name",
                "new_value": "Zed"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // **[2026-03-15]** Reason: force persistence to generate a new version.
    // **[2026-03-15]** Purpose: ensure versions list includes old + new.
    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: verify data changed before rollback.
    // **[2026-03-15]** Purpose: ensure checkout test validates actual behavior.
    let res = client
        .get(format!(
            "{}/api/grid-data?table_name={}&page=1&page_size=10",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let cols = body["columns"].as_array().unwrap();
    let col_idx = cols
        .iter()
        .position(|c| c.as_str() == Some("name"))
        .unwrap();
    let rows = body["data"].as_array().unwrap();
    assert_eq!(
        rows[0][col_idx].as_str().unwrap_or_default(),
        "Zed"
    );

    // **[2026-03-15]** Reason: fetch versions to determine rollback target.
    // **[2026-03-15]** Purpose: wait for auto-flush to create a second version.
    let versions = wait_for_versions(&client, &base_url, &table_name, None).await;
    assert!(versions.len() >= 2);
    let min_version = versions
        .iter()
        .filter_map(|v| v["version"].as_u64())
        .min()
        .unwrap();

    // **[2026-03-15]** Reason: call checkout without session_id (active session path).
    // **[2026-03-15]** Purpose: restore original data.
    let res = client
        .post(format!(
            "{}/api/checkout_version?table_name={}&version={}",
            base_url, table_name, min_version
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: verify data after checkout.
    // **[2026-03-15]** Purpose: ensure rollback restored the original value.
    let res = client
        .get(format!(
            "{}/api/grid-data?table_name={}&page=1&page_size=10",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let cols = body["columns"].as_array().unwrap();
    let col_idx = cols
        .iter()
        .position(|c| c.as_str() == Some("name"))
        .unwrap();
    let rows = body["data"].as_array().unwrap();
    assert_eq!(
        rows[0][col_idx].as_str().unwrap_or_default(),
        "Alice"
    );
}

// **[2026-03-15]** Reason: TDD for /api/checkout_version with session_id.
// **[2026-03-15]** Purpose: ensure explicit session path is honored.
// **[2026-03-15]** Scope: query-parameter API shape + data content verification.
#[tokio::test]
async fn test_checkout_version_with_session_id() {
    // **[2026-03-15]** Reason: isolate table for explicit-session rollback.
    // **[2026-03-15]** Purpose: avoid cross-test interference.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "checkout_with_sid",
        "id,name\n1,Alice\n",
    )
    .await;

    // **[2026-03-15]** Reason: create an explicit session for rollback.
    // **[2026-03-15]** Purpose: test session_id path explicitly.
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // **[2026-03-15]** Reason: mutate data to generate a new version.
    // **[2026-03-15]** Purpose: ensure checkout can revert changes.
    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "row": 0,
            "col_name": "name",
            "new_value": "Zed"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: trigger auto-flush sooner via pending write count.
    // **[2026-03-15]** Purpose: keep versions available within polling timeout.
    for _ in 0..19 {
        let res = client
            .post(format!("{}/api/update_cell", base_url))
            .json(&json!({
                "table_name": table_name,
                "session_id": session_id,
                "row": 0,
                "col_name": "name",
                "new_value": "Zed"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // **[2026-03-15]** Reason: force persistence to create a new version.
    // **[2026-03-15]** Purpose: ensure versions list includes old + new.
    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: fetch versions to identify rollback target.
    // **[2026-03-15]** Purpose: wait for auto-flush to create a second version.
    let versions =
        wait_for_versions(&client, &base_url, &table_name, Some(&session_id)).await;
    assert!(versions.len() >= 2);
    let min_version = versions
        .iter()
        .filter_map(|v| v["version"].as_u64())
        .min()
        .unwrap();

    // **[2026-03-15]** Reason: call checkout with session_id.
    // **[2026-03-15]** Purpose: restore original data for that session.
    let res = client
        .post(format!(
            "{}/api/checkout_version?table_name={}&version={}&session_id={}",
            base_url, table_name, min_version, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: verify data after explicit-session checkout.
    // **[2026-03-15]** Purpose: ensure rollback restored the original value.
    let res = client
        .get(format!(
            "{}/api/grid-data?table_name={}&session_id={}&page=1&page_size=10",
            base_url, table_name, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let cols = body["columns"].as_array().unwrap();
    let col_idx = cols
        .iter()
        .position(|c| c.as_str() == Some("name"))
        .unwrap();
    let rows = body["data"].as_array().unwrap();
    assert_eq!(
        rows[0][col_idx].as_str().unwrap_or_default(),
        "Alice"
    );
}

// **[2026-03-15]** Reason: TDD for checkout mismatch session/table.
// **[2026-03-15]** Purpose: ensure backend rejects cross-table session rollback.
#[tokio::test]
async fn test_checkout_version_session_table_mismatch() {
    // **[2026-03-15]** Reason: isolate tables for mismatch verification.
    // **[2026-03-15]** Purpose: avoid cross-test interference.
    let (client, base_url) = spawn_test_server().await;
    let table_a = register_csv_table(
        &client,
        &base_url,
        "checkout_mismatch_a",
        "id,name\n1,Alice\n",
    )
    .await;
    let table_b = register_csv_table(
        &client,
        &base_url,
        "checkout_mismatch_b",
        "id,name\n1,Bob\n",
    )
    .await;

    // **[2026-03-15]** Reason: create a session bound to table A.
    // **[2026-03-15]** Purpose: reuse session_id against table B to trigger mismatch.
    let session_id = create_session_and_get_id(&client, &base_url, &table_a).await;

    // **[2026-03-15]** Reason: mutate data to create a new version candidate.
    // **[2026-03-15]** Purpose: ensure checkout target exists before mismatch call.
    let res = client
        .post(format!("{}/api/update_cell", base_url))
        .json(&json!({
            "table_name": table_a,
            "session_id": session_id,
            "row": 0,
            "col_name": "name",
            "new_value": "Zed"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // **[2026-03-15]** Reason: force persistence to create a new version.
    // **[2026-03-15]** Purpose: make sure versions list is not empty.
    let res = client
        .post(format!("{}/api/save_session", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // **[2026-03-15]** Reason: wait for versions to materialize.
    // **[2026-03-15]** Purpose: find a valid version number for checkout.
    let versions = wait_for_versions(&client, &base_url, &table_a, Some(&session_id)).await;
    let min_version = versions
        .iter()
        .filter_map(|v| v["version"].as_u64())
        .min()
        .unwrap_or(0);

    // **[2026-03-15]** Reason: call checkout with mismatched table_name.
    // **[2026-03-15]** Purpose: verify backend returns error for cross-table session usage.
    let res = client
        .post(format!(
            "{}/api/checkout_version?table_name={}&version={}&session_id={}",
            base_url, table_b, min_version, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"]
        .as_str()
        .unwrap_or("")
        .contains("Session table mismatch"));
}

// **[2026-03-15]** Reason: add explicit session_id=null coverage.
// **[2026-03-15]** Purpose: ensure null normalization falls back to active session.
#[tokio::test]
async fn test_versions_endpoint_accepts_null_session_id() {
    // **[2026-03-15]** Reason: isolate table data for null-session test.
    // **[2026-03-15]** Purpose: avoid interference with other version tests.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "versions_null_sid",
        "id,amount\n1,10\n",
    )
    .await;
    let _ = create_session_and_get_id(&client, &base_url, &table_name).await;

    // **[2026-03-15]** Reason: frontend sends session_id=null when empty.
    // **[2026-03-15]** Purpose: validate backend normalization behavior.
    let res = client
        .get(format!(
            "{}/api/versions?table_name={}&session_id=null",
            base_url, table_name
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["versions"].is_array());
}

// **[2026-03-15]** 变更原因：新增 ensure_columns 接口回归测试。
// **[2026-03-15]** 变更目的：验证幂等扩列与 batch_update_cells 写入新列。
#[tokio::test]
async fn test_ensure_columns_idempotent_and_batch_update() {
    // **[2026-03-15]** 变更原因：准备独立表数据。
    // **[2026-03-15]** 变更目的：避免影响其他测试用例。
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "ensure_cols",
        "id,amount\n1,10\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "columns": [
                { "name": "pivot_col_5", "type": "utf8" },
                { "name": "pivot_col_6", "type": "utf8" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    let columns = body["columns"].as_array().unwrap();
    assert!(columns.iter().any(|v| v == "pivot_col_5"));
    assert!(columns.iter().any(|v| v == "pivot_col_6"));

    // **[2026-03-15]** 变更原因：ensure_columns 可能触发会话自动分叉。
    // **[2026-03-15]** 变更目的：后续更新使用服务端返回的 session_id。
    let effective_session_id = body["session_id"]
        .as_str()
        .unwrap_or(&session_id)
        .to_string();

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": effective_session_id,
            "columns": [
                { "name": "pivot_col_5", "type": "utf8" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    let res = client
        .post(format!("{}/api/batch_update_cells", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": effective_session_id,
            "updates": [
                { "row": 0, "col_name": "pivot_col_5", "new_value": "x" },
                { "row": 0, "col_name": "pivot_col_6", "new_value": "y" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// **[2026-03-15]** 变更原因：补齐 ensure_columns 错误响应字段断言。
// **[2026-03-15]** 变更目的：确保前端可读取 error 字段并保持兼容。
#[tokio::test]
async fn test_ensure_columns_error_includes_error_field() {
    // **[2026-03-15]** 变更原因：构造空 table_name 触发校验失败。
    // **[2026-03-15]** 变更目的：验证错误响应包含 status/error 字段。
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/ensure_columns", base_url))
        .json(&json!({
            "table_name": "",
            "columns": [
                { "name": "pivot_col_1", "type": "utf8" }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body.get("error").is_some());
}

// **[2026-03-15]** Reason: add /api/update_style_range integration coverage.
// **[2026-03-15]** Purpose: verify optional session_id and extended style fields.
#[tokio::test]
async fn test_update_style_range_with_optional_session_id() {
    // **[2026-03-15]** Reason: create independent table + session.
    // **[2026-03-15]** Purpose: keep style-range tests isolated.
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "style_range",
        "id,name\n1,A\n2,B\n",
    )
    .await;
    let session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    // **[2026-03-15]** Reason: cover active-session path (no session_id).
    // **[2026-03-15]** Purpose: ensure style range updates are accepted.
    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": table_name,
            "range": { "start_row": 0, "start_col": 0, "end_row": 0, "end_col": 1 },
            "style": {
                "bold": true,
                "italic": true,
                "underline": true,
                "color": "#FF0000",
                "bg_color": "#00FF00"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // **[2026-03-15]** Reason: cover explicit session_id path.
    // **[2026-03-15]** Purpose: ensure specified session gets style update.
    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": table_name,
            "session_id": session_id,
            "range": { "start_row": 0, "start_col": 0, "end_row": 0, "end_col": 1 },
            "style": { "bold": false }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// **[2026-03-15]** 变更原因：补齐 update_style_range 错误响应字段断言。
// **[2026-03-15]** 变更目的：确保前端可读取 error 字段并保持兼容。
#[tokio::test]
async fn test_update_style_range_error_includes_error_field() {
    // **[2026-03-15]** 变更原因：构造不存在的表名触发会话缺失错误。
    // **[2026-03-15]** 变更目的：验证错误响应包含 status/error 字段。
    let (client, base_url) = spawn_test_server().await;

    let res = client
        .post(format!("{}/api/update_style_range", base_url))
        .json(&json!({
            "table_name": "missing_table",
            "range": { "start_row": 0, "start_col": 0, "end_row": 0, "end_col": 0 },
            "style": { "bold": true }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body.get("error").is_some());
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
    // **[2026-03-15]** 变更原因：前端要求 /api/execute 返回统一 status 字段。
    // **[2026-03-15]** 变更目的：验证成功路径返回 status=ok。
    // **[2026-03-15]** 变更说明：保持原有 error/rows 校验不变。
    assert_eq!(body["status"], "ok");
    assert!(body["error"].is_null());
    let value = body["rows"][0][0].as_str().unwrap().parse::<f64>().unwrap();
    assert!((value - 99.0).abs() < 1e-9);
}

// **[2026-03-15]** 变更原因：补齐 /api/execute 错误路径的 status 验证。
// **[2026-03-15]** 变更目的：确保失败时仍返回 status=error 与错误字段。
#[tokio::test]
async fn test_execute_returns_status_on_error() {
    // **[2026-03-15]** 变更原因：构造最小表用于触发查询错误。
    // **[2026-03-15]** 变更目的：保证错误来自字段缺失而非环境问题。
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "execute_status_error",
        "id,name\n1,A\n",
    )
    .await;

    // **[2026-03-15]** 变更原因：故意查询不存在的列。
    // **[2026-03-15]** 变更目的：验证 /api/execute 错误响应携带 status/error。
    let res = client
        .post(format!("{}/api/execute", base_url))
        .json(&json!({
            "sql": format!("SELECT missing FROM \"{}\"", table_name)
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["error"].as_str().unwrap_or("").len() > 0);
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
    // **[2026-03-15]** 变更原因：前端要求 /api/execute 返回统一 status 字段。
    // **[2026-03-15]** 变更目的：验证成功路径返回 status=ok。
    // **[2026-03-15]** 变更说明：继续保持 error/rows 原有断言。
    assert_eq!(body["status"], "ok");
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
    // **[2026-03-15]** 变更原因：前端要求 /api/execute 返回统一 status 字段。
    // **[2026-03-15]** 变更目的：验证成功路径返回 status=ok。
    // **[2026-03-15]** 变更说明：保持旧断言不变以确保兼容。
    assert_eq!(body["status"], "ok");
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

#[tokio::test]
async fn test_update_merge_returns_ok_for_active_session() {
    let (client, base_url) = spawn_test_server().await;
    let table_name = register_csv_table(
        &client,
        &base_url,
        "pos_merge",
        "id,name\n1,A\n2,B\n",
    )
    .await;

    let _session_id = create_session_and_get_id(&client, &base_url, &table_name).await;

    let res = client
        .post(format!("{}/api/update_merge", base_url))
        .json(&json!({
            "table_name": table_name,
            "range": {
                "start_col": 0,
                "start_row": 0,
                "end_col": 1,
                "end_row": 0
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["message"] == "Merged" || body["message"] == "Unmerged");
}

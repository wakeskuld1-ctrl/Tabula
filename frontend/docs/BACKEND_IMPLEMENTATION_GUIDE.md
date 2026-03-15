# Backend Implementation Guide for Cell Update

Since you have access to modify the Rust backend, implementing the `update_cell` logic in the backend is the correct approach. This guide provides the necessary code changes.

## 1. Modify `federated_query_engine/src/session_manager/mod.rs`

Add the `update_cell` method to the `impl SessionManager` block. This method handles finding the correct session, locking the data, and updating the Arrow array in memory.

**Add this method to `impl SessionManager` (e.g., after `update_merge`):**

```rust
    // **[2026-03-12]** Change: Implement real backend cell update logic
    pub async fn update_cell(
        &self,
        table_name: &str,
        row_idx: usize,
        col_name: &str,
        new_value: String,
    ) -> Result<(), UpdateCellError> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active
                .get(table_name)
                .cloned()
                .ok_or(UpdateCellError::NoActiveSession {
                    table_name: table_name.to_string(),
                })?
        };

        // We need to release the lock on `sessions` before we acquire `data_lock` write lock
        // to avoid potential deadlocks if `current_data` logic is complex (though here it's just a RwLock).
        // However, `current_data` is inside `SessionInfo` which is inside `sessions` Mutex.
        // So we hold `sessions` lock, get a reference to `current_data`, then lock `current_data`.
        
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(&session_id)
            .ok_or(UpdateCellError::SessionNotFound {
                session_id: session_id.clone(),
            })?;

        let data_lock = session
            .current_data
            .as_ref()
            .ok_or(UpdateCellError::SessionDataNotLoaded)?;

        let mut batches = data_lock.write().await;

        if batches.is_empty() {
             return Err(UpdateCellError::EmptyDataset);
        }

        // Find the batch and row index within the batch
        let mut current_row_count = 0;
        let mut target_batch_idx = None;
        let mut row_in_batch = 0;

        for (i, batch) in batches.iter().enumerate() {
            let rows = batch.num_rows();
            if row_idx < current_row_count + rows {
                target_batch_idx = Some(i);
                row_in_batch = row_idx - current_row_count;
                break;
            }
            current_row_count += rows;
        }

        let batch_idx = target_batch_idx.ok_or(UpdateCellError::Internal {
             reason: format!("Row index {} out of bounds", row_idx),
        })?;
        
        let batch = &batches[batch_idx];
        let schema = batch.schema();
        let col_idx = schema.index_of(col_name).map_err(|_| UpdateCellError::ColumnNotFound {
             column: col_name.to_string(),
        })?;
        
        let field = schema.field(col_idx);
        let col_array = batch.column(col_idx);

        // Update the array
        let new_array = match field.data_type() {
            DataType::Utf8 => {
                 let any_array = col_array.as_any();
                 let string_array = any_array.downcast_ref::<StringArray>().ok_or(UpdateCellError::CastFailed{ reason: "Not a StringArray".to_string() })?;
                 
                 let mut builder = arrow::array::StringBuilder::new();
                 for i in 0..string_array.len() {
                     if i == row_in_batch {
                         builder.append_value(&new_value);
                     } else {
                         if string_array.is_null(i) {
                             builder.append_null();
                         } else {
                             builder.append_value(string_array.value(i));
                         }
                     }
                 }
                 Arc::new(builder.finish()) as ArrayRef
            },
            DataType::Int64 => {
                 let any_array = col_array.as_any();
                 let int_array = any_array.downcast_ref::<Int64Array>().ok_or(UpdateCellError::CastFailed{ reason: "Not an Int64Array".to_string() })?;
                 
                 let parsed_val = new_value.parse::<i64>().map_err(|e| UpdateCellError::CastFailed { reason: e.to_string() })?;
                 
                 let mut builder = arrow::array::Int64Builder::new();
                 for i in 0..int_array.len() {
                     if i == row_in_batch {
                         builder.append_value(parsed_val);
                     } else {
                         if int_array.is_null(i) {
                             builder.append_null();
                         } else {
                             builder.append_value(int_array.value(i));
                         }
                     }
                 }
                 Arc::new(builder.finish()) as ArrayRef
            },
            DataType::Float64 => {
                 let any_array = col_array.as_any();
                 let float_array = any_array.downcast_ref::<Float64Array>().ok_or(UpdateCellError::CastFailed{ reason: "Not a Float64Array".to_string() })?;
                 
                 let parsed_val = new_value.parse::<f64>().map_err(|e| UpdateCellError::CastFailed { reason: e.to_string() })?;
                 
                 let mut builder = arrow::array::Float64Builder::new();
                 for i in 0..float_array.len() {
                     if i == row_in_batch {
                         builder.append_value(parsed_val);
                     } else {
                         if float_array.is_null(i) {
                             builder.append_null();
                         } else {
                             builder.append_value(float_array.value(i));
                         }
                     }
                 }
                 Arc::new(builder.finish()) as ArrayRef
            },
            dt => return Err(UpdateCellError::UnsupportedType { data_type: dt.to_string() }),
        };

        // Reconstruct batch
        let mut new_columns = batch.columns().to_vec();
        new_columns[col_idx] = new_array;

        let new_batch = RecordBatch::try_new(schema.clone(), new_columns).map_err(|e| UpdateCellError::Internal { reason: e.to_string() })?;
        
        batches[batch_idx] = new_batch;
        
        Ok(())
    }
```

## 2. Update `federated_query_engine/src/api/mod.rs`

Ensure `update_handler` is exported:

```rust
pub mod execute_handler;
pub mod grid_handler;
pub mod health_handler;
pub mod plan_handler;
pub mod register_handler;
pub mod upload_handler;
pub mod update_handler; // Add this line
```

## 3. Implement `federated_query_engine/src/api/update_handler.rs`

Create or update this file with the following content:

```rust
use axum::{Json, extract::State};
use std::sync::Arc;
use serde::Deserialize;
use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateCellRequest {
    pub table_name: String,
    pub row_idx: usize,
    pub col_name: String,
    pub new_value: String,
}

pub async fn update_cell(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateCellRequest>,
) -> Json<serde_json::Value> {
    match state.session_manager.update_cell(
        &payload.table_name,
        payload.row_idx,
        &payload.col_name,
        payload.new_value,
    ).await {
        Ok(_) => Json(serde_json::json!({ "status": "ok" })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "code": e.code(),
            "message": e.message(),
            "details": e.details()
        })),
    }
}
```

## 4. Register Route in `federated_query_engine/src/main.rs`

Add the route to the `Router` in `main()`:

```rust
// In main() function
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
    // Add these routes:
    .route("/api/grid-data", get(crate::api::grid_handler::get_grid_data))
    .route("/api/update_cell", post(crate::api::update_handler::update_cell))
    
    .fallback_service(ServeDir::new(public_path_str).append_index_html_on_directories(true));
```

Once these changes are applied and the backend is restarted, the frontend can be simplified to call these endpoints directly.

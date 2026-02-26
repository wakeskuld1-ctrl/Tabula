use super::DataSource;
use async_trait::async_trait;
use calamine::{open_workbook, Data as CalData, Reader, Xlsx};
use datafusion::arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, Int64Builder, StringBuilder,
};
use datafusion::arrow::datatypes::{DataType as ArrowDataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use datafusion::error::{DataFusionError, Result as DFResult};
use datafusion::prelude::SessionContext;
use std::sync::Arc;

pub struct ExcelDataSource {
    name: String,
    path: String,
    sheet_name: String,
}

impl ExcelDataSource {
    pub fn new(name: String, path: String, sheet_name: String) -> Self {
        Self {
            name,
            path,
            sheet_name,
        }
    }

    #[allow(dead_code)]
    pub fn get_sheet_names(path: &str) -> DFResult<Vec<String>> {
        let workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| DataFusionError::Execution(format!("Failed to open Excel file: {}", e)))?;
        Ok(workbook.sheet_names().to_vec())
    }

    pub fn load_table(&self) -> DFResult<Arc<MemTable>> {
        let path = &self.path;
        let sheet_name = &self.sheet_name;

        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| DataFusionError::Execution(format!("Failed to open Excel file: {}", e)))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| DataFusionError::Execution(format!("Failed to read range: {}", e)))?;

        if range.is_empty() {
            return Err(DataFusionError::Execution("Sheet is empty".to_string()));
        }

        // 1. Extract Headers
        let mut rows_iter = range.rows();
        let header_row = rows_iter
            .next()
            .ok_or_else(|| DataFusionError::Execution("Sheet has no headers".to_string()))?;

        let mut headers: Vec<String> = header_row.iter().map(|c: &CalData| c.to_string()).collect();

        println!("Found headers in sheet {}: {:?}", sheet_name, headers);

        // Fix: Deduplicate headers
        let mut seen_headers = std::collections::HashSet::new();
        for header in &mut headers {
            let original_name = header.clone();
            let mut name = original_name.clone();
            let mut count = 1;
            while seen_headers.contains(&name) {
                name = format!("{}_{}", original_name, count);
                count += 1;
            }
            seen_headers.insert(name.clone());
            *header = name;
        }

        // 2. Infer Schema (Scan first 100 rows or all)
        // For simplicity, we scan all rows in memory since we already loaded the range
        let rows: Vec<&[CalData]> = rows_iter.collect();
        // let row_count = rows.len();

        let mut column_types: Vec<ArrowDataType> = vec![ArrowDataType::Utf8; headers.len()];

        for col_idx in 0..headers.len() {
            let mut inferred_type = ArrowDataType::Null;

            for row in &rows {
                if col_idx >= (*row).len() {
                    continue;
                }
                let cell = &row[col_idx];

                let cell_type = match cell {
                    CalData::Int(_) => ArrowDataType::Int64,
                    CalData::Float(_) => ArrowDataType::Float64,
                    CalData::String(_) => ArrowDataType::Utf8,
                    CalData::Bool(_) => ArrowDataType::Boolean,
                    CalData::DateTime(_) => ArrowDataType::Utf8, // Treat dates as strings for now
                    CalData::Error(_) => ArrowDataType::Utf8,
                    CalData::Empty => ArrowDataType::Null,
                    _ => ArrowDataType::Utf8, // Handle variants like DateTimeIso if any
                };

                if cell_type == ArrowDataType::Null {
                    continue;
                }

                if inferred_type == ArrowDataType::Null {
                    inferred_type = cell_type;
                } else if inferred_type != cell_type {
                    // Type mismatch, fallback to String
                    // Exception: Int can be upgraded to Float
                    if (inferred_type == ArrowDataType::Int64
                        && cell_type == ArrowDataType::Float64)
                        || (inferred_type == ArrowDataType::Float64
                            && cell_type == ArrowDataType::Int64)
                    {
                        inferred_type = ArrowDataType::Float64;
                    } else {
                        inferred_type = ArrowDataType::Utf8;
                        break; // Once String, always String
                    }
                }
            }

            if inferred_type != ArrowDataType::Null {
                column_types[col_idx] = inferred_type;
            } else {
                // Default to String if all empty
                column_types[col_idx] = ArrowDataType::Utf8;
            }
        }

        // 3. Create Schema
        let fields: Vec<Field> = headers
            .iter()
            .zip(column_types.iter())
            .map(|(name, dtype): (&String, &ArrowDataType)| Field::new(name, dtype.clone(), true))
            .collect();
        let schema = Arc::new(Schema::new(fields));

        // 4. Build Columns
        let mut arrays: Vec<ArrayRef> = Vec::new();

        for (col_idx, dtype) in column_types.iter().enumerate() {
            let array: ArrayRef = match dtype {
                ArrowDataType::Int64 => {
                    let mut builder = Int64Builder::new();
                    for row_ref in &rows {
                        let row = *row_ref;
                        if col_idx < row.len() {
                            match &row[col_idx] {
                                CalData::Int(v) => builder.append_value(*v),
                                CalData::Float(v) => builder.append_value(*v as i64),
                                _ => builder.append_null(),
                            }
                        } else {
                            builder.append_null();
                        }
                    }
                    Arc::new(builder.finish())
                }
                ArrowDataType::Float64 => {
                    let mut builder = Float64Builder::new();
                    for row_ref in &rows {
                        let row = *row_ref;
                        if col_idx < row.len() {
                            match &row[col_idx] {
                                CalData::Float(v) => builder.append_value(*v),
                                CalData::Int(v) => builder.append_value(*v as f64),
                                _ => builder.append_null(),
                            }
                        } else {
                            builder.append_null();
                        }
                    }
                    Arc::new(builder.finish())
                }
                ArrowDataType::Boolean => {
                    let mut builder = BooleanBuilder::new();
                    for row_ref in &rows {
                        let row = *row_ref;
                        if col_idx < row.len() {
                            match &row[col_idx] {
                                CalData::Bool(v) => builder.append_value(*v),
                                _ => builder.append_null(),
                            }
                        } else {
                            builder.append_null();
                        }
                    }
                    Arc::new(builder.finish())
                }
                _ => {
                    // String or others
                    let mut builder = StringBuilder::new();
                    for row_ref in &rows {
                        let row = *row_ref;
                        if col_idx < row.len() {
                            match &row[col_idx] {
                                CalData::Empty => builder.append_null(),
                                v => builder.append_value(v.to_string()),
                            }
                        } else {
                            builder.append_null();
                        }
                    }
                    Arc::new(builder.finish())
                }
            };
            arrays.push(array);
        }

        // 5. Create MemTable
        let batch = RecordBatch::try_new(schema.clone(), arrays)?;
        let table = MemTable::try_new(schema, vec![vec![batch]])?;

        Ok(Arc::new(table))
    }
}

#[async_trait]
impl DataSource for ExcelDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn register(&self, ctx: &SessionContext) -> DFResult<()> {
        let table = self.load_table()?;
        ctx.register_table(&self.name, table)?;
        Ok(())
    }
}

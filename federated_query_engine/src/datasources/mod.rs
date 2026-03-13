use async_trait::async_trait;
use datafusion::arrow::datatypes::DataType;
use datafusion::error::Result;
use datafusion::prelude::SessionContext;

pub mod csv;
pub mod excel;
pub mod parquet;
// pub mod sqlite;
#[cfg(feature = "oracle")]
pub mod oracle;
pub mod sql_dialect;
pub mod yashandb;
#[cfg(test)]
mod yashandb_repro_test;

pub fn decimal_type(precision: i64, scale: i64) -> DataType {
    let mut p = precision;
    let mut s = scale;
    p = p.clamp(1, 76);
    s = s.clamp(0, p);
    if p <= 38 {
        DataType::Decimal128(p as u8, s as i8)
    } else {
        DataType::Decimal256(p as u8, s as i8)
    }
}

pub fn map_numeric_precision_scale(precision: Option<i64>, scale: Option<i64>) -> DataType {
    match (precision, scale) {
        (None, None) => DataType::Float64,
        (Some(p), Some(s)) => {
            if s > 0 {
                decimal_type(p, s)
            } else if p <= 19 {
                DataType::Int64
            } else {
                decimal_type(p, 0)
            }
        }
        (Some(p), None) => {
            if p <= 19 {
                DataType::Int64
            } else {
                decimal_type(p, 0)
            }
        }
        (None, Some(s)) => {
            if s > 0 {
                decimal_type(38, s)
            } else {
                decimal_type(38, 0)
            }
        }
    }
}

/// A unified trait for loading data sources into DataFusion context.
#[async_trait]
pub trait DataSource: Sync + Send {
    /// Returns the name of the table to be registered.
    fn name(&self) -> &str;

    /// Registers the data source into the given SessionContext.
    async fn register(&self, ctx: &SessionContext) -> Result<()>;

    /// Returns the table statistics (num_rows, avg_row_len) if available.
    fn get_table_stats(&self) -> std::result::Result<(Option<i64>, Option<i64>), String> {
        Ok((None, None))
    }
}

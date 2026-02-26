use async_trait::async_trait;
use datafusion::error::Result;
use datafusion::prelude::SessionContext;

pub mod csv;
pub mod excel;
#[cfg(feature = "oracle")]
pub mod oracle;
pub mod parquet;
pub mod sql_dialect;
pub mod sqlite;

/// A unified trait for loading data sources into DataFusion context.
#[async_trait]
pub trait DataSource: Sync + Send {
    /// Returns the name of the table to be registered.
    fn name(&self) -> &str;

    /// Registers the data source into the given SessionContext.
    async fn register(&self, ctx: &SessionContext) -> Result<()>;
}

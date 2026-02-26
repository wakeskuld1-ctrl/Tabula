use async_trait::async_trait;
use datafusion::prelude::SessionContext;
use datafusion::error::Result;

pub mod csv;
pub mod excel;
pub mod parquet;
pub mod sqlite;
pub mod sql_dialect;
#[cfg(feature = "oracle")]
pub mod oracle;

/// A unified trait for loading data sources into DataFusion context.
#[async_trait]
pub trait DataSource: Sync + Send {
    /// Returns the name of the table to be registered.
    fn name(&self) -> &str;
    
    /// Registers the data source into the given SessionContext.
    async fn register(&self, ctx: &SessionContext) -> Result<()>;
}

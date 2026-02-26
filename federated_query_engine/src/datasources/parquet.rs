use super::DataSource;
use async_trait::async_trait;
use datafusion::error::Result;
use datafusion::prelude::{ParquetReadOptions, SessionContext};

pub struct ParquetDataSource {
    name: String,
    path: String,
}

impl ParquetDataSource {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

#[async_trait]
impl DataSource for ParquetDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        ctx.register_parquet(&self.name, &self.path, ParquetReadOptions::default())
            .await?;
        Ok(())
    }
}

use super::DataSource;
use async_trait::async_trait;
use datafusion::error::Result;
use datafusion::prelude::{CsvReadOptions, SessionContext};

pub struct CsvDataSource {
    name: String,
    path: String,
}

impl CsvDataSource {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

#[async_trait]
impl DataSource for CsvDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        ctx.register_csv(&self.name, &self.path, CsvReadOptions::new())
            .await
    }
}

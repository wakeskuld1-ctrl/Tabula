use super::DataSource;
use async_trait::async_trait;
use datafusion::error::Result;
use datafusion::prelude::{ParquetReadOptions, SessionContext};

/// Parquet 数据源实现
///
/// **实现方案**:
/// 简单的 Parquet 文件包装器，利用 DataFusion 内置的 `ParquetReadOptions` 进行读取。
///
/// **关键问题点**:
/// - 利用 Parquet 文件的自描述 Schema，无需额外推断。
pub struct ParquetDataSource {
    name: String,
    path: String,
}

impl ParquetDataSource {
    /// 创建 Parquet 数据源
    ///
    /// **参数**:
    /// * `name`: 表名
    /// * `path`: Parquet 文件路径
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

#[async_trait]
impl DataSource for ParquetDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    /// 注册 Parquet 表到 DataFusion 上下文
    ///
    /// **实现方案**:
    /// 调用 `ctx.register_parquet`，使用默认的 `ParquetReadOptions`。
    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        ctx.register_parquet(&self.name, &self.path, ParquetReadOptions::default())
            .await?;
        Ok(())
    }
}

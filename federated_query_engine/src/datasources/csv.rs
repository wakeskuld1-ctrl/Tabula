use super::DataSource;
use async_trait::async_trait;
use datafusion::error::Result;
use datafusion::prelude::{CsvReadOptions, SessionContext};

/// CSV 数据源实现
///
/// **实现方案**:
/// 简单的 CSV 文件包装器，利用 DataFusion 内置的 `CsvReadOptions` 进行读取。
///
/// **关键问题点**:
/// - 依赖 DataFusion 的推断能力，未自定义 Schema 推断逻辑。
pub struct CsvDataSource {
    name: String,
    path: String,
}

impl CsvDataSource {
    /// 创建 CSV 数据源
    ///
    /// **参数**:
    /// * `name`: 表名
    /// * `path`: CSV 文件路径
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

#[async_trait]
impl DataSource for CsvDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    /// 注册 CSV 表到 DataFusion 上下文
    ///
    /// **实现方案**:
    /// 调用 `ctx.register_csv`，使用默认的 `CsvReadOptions`。
    async fn register(&self, ctx: &SessionContext) -> Result<()> {
        ctx.register_csv(&self.name, &self.path, CsvReadOptions::new())
            .await
    }
}

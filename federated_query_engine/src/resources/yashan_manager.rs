use odbc_api::{Connection, ConnectionOptions, Environment};
use r2d2::ManageConnection;

use lazy_static::lazy_static;

lazy_static! {
    /// 全局 ODBC 环境单例
    ///
    /// **实现方案**:
    /// ODBC Environment 必须是线程安全的且在整个应用生命周期内存在。
    pub static ref ODBC_ENV: Environment =
        Environment::new().expect("Failed to create ODBC environment");
}

/// YashanDB 连接管理器 (基于 ODBC)
///
/// **实现方案**:
/// 使用 `odbc-api` crate 管理 YashanDB 的 ODBC 连接。
/// 连接对象生命周期被绑定到 `static` 的 `ODBC_ENV`，因此连接可以跨线程传递。
#[derive(Debug)]
pub struct YashanConnectionManager {
    connection_string: String,
}

impl YashanConnectionManager {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

#[derive(Debug)]
pub struct YashanError(String);

impl std::fmt::Display for YashanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for YashanError {}

impl ManageConnection for YashanConnectionManager {
    type Connection = Connection<'static>;
    type Error = YashanError;

    /// 建立 ODBC 连接
    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let env: &'static Environment = &ODBC_ENV;
        env.connect_with_connection_string(&self.connection_string, ConnectionOptions::default())
            .map_err(|e| YashanError(e.to_string()))
    }

    /// 验证连接有效性
    ///
    /// **实现方案**:
    /// 执行简单查询 `SELECT 1 FROM DUAL`。
    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // YashanDB uses DUAL
        conn.execute("SELECT 1 FROM DUAL", ())
            .map(|_| ())
            .map_err(|e| YashanError(e.to_string()))
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

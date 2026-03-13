use oracle::Connection;
use r2d2::ManageConnection;

/// Oracle 连接管理器
///
/// **实现方案**:
/// 实现 `r2d2::ManageConnection` trait，用于管理 `oracle::Connection` 的生命周期。
///
/// **关键问题点**:
/// - 线程安全：`oracle::Connection` 在不同线程间传递需要是 Send 的（Rust Oracle Driver 支持）。
#[derive(Debug)]
pub struct OracleConnectionManager {
    user: String,
    pass: String,
    conn_str: String,
}

impl OracleConnectionManager {
    /// 创建连接管理器
    ///
    /// **参数**:
    /// * `service`: Oracle Service Name (非 SID)。
    pub fn new(user: &str, pass: &str, host: &str, port: u16, service: &str) -> Self {
        // Construct Easy Connect string: //host:port/service_name
        let conn_str = format!("//{}:{}/{}", host, port, service);
        Self {
            user: user.to_string(),
            pass: pass.to_string(),
            conn_str,
        }
    }
}

impl ManageConnection for OracleConnectionManager {
    type Connection = Connection;
    type Error = oracle::Error;

    /// 建立新连接
    fn connect(&self) -> Result<Connection, Self::Error> {
        Connection::connect(&self.user, &self.pass, &self.conn_str)
    }

    /// 验证连接有效性
    ///
    /// **实现方案**:
    /// 使用轻量级的 `ping` 方法。
    fn is_valid(&self, conn: &mut Connection) -> Result<(), Self::Error> {
        // Lightweight check (ping)
        conn.ping()
    }

    /// 检查连接是否已损坏
    ///
    /// **实现方案**:
    /// 默认返回 false，依赖 `is_valid` 进行主动检查。
    fn has_broken(&self, _conn: &mut Connection) -> bool {
        // r2d2 will call is_valid to check, so we can return false here
        // unless we have a specific way to detect broken state without IO.
        false
    }
}

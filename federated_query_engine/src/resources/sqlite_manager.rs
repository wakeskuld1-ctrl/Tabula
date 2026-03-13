use r2d2::ManageConnection;
use rusqlite::{Connection, Error};
use std::path::PathBuf;

/// SQLite 连接管理器
///
/// **实现方案**:
/// 管理 `rusqlite::Connection`。由于 SQLite 是文件型数据库，连接实际上是文件句柄。
#[derive(Debug)]
pub struct SqliteConnectionManager {
    path: PathBuf,
}

impl SqliteConnectionManager {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }
}

impl ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = Error;

    /// 打开 SQLite 数据库文件
    fn connect(&self) -> Result<Connection, Error> {
        Connection::open(&self.path)
            .map_err(|e| e)
    }

    /// 验证连接
    ///
    /// **实现方案**:
    /// 执行空语句测试。
    fn is_valid(&self, conn: &mut Connection) -> Result<(), Error> {
        conn.execute_batch("").map_err(|e| e)
    }

    fn has_broken(&self, _conn: &mut Connection) -> bool {
        false
    }
}

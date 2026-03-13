use crate::resources::yashan_manager::YashanConnectionManager;
use once_cell::sync::Lazy;
use r2d2::Pool;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

// 定义支持的数据库类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DbType {
    Oracle,
    Yashan,
    // Future: Mysql, Postgres...
}

// 统一数据库配置
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub service: Option<String>, // Oracle Service Name or Yashan DB Name
    pub max_pool_size: u32,
}

impl DbConfig {
    /// 生成唯一连接标识 (Key)
    ///
    /// **实现方案**:
    /// 对所有连接参数（类型、主机、端口、用户、密码、服务名）进行 Hash 计算。
    ///
    /// **关键问题点**:
    /// - 隔离性：确保不同用户、不同数据库实例使用不同的连接池。
    /// - 安全性：密码也参与 Hash，防止权限混淆（例如同一用户不同密码）。
    pub fn connection_key(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.db_type.hash(&mut hasher);
        self.host.hash(&mut hasher);
        self.port.hash(&mut hasher);
        self.user.hash(&mut hasher);
        self.pass.hash(&mut hasher); // 密码也参与 Hash，防止权限混淆
        self.service.hash(&mut hasher);

        format!("{:?}_{}", self.db_type, hasher.finish())
    }
}

// 连接池变体枚举
#[derive(Clone)]
pub enum PoolVariant {
    Oracle(Pool<crate::resources::oracle_manager::OracleConnectionManager>),
    Yashan(Pool<YashanConnectionManager>),
}

/// 全局连接池管理器
///
/// **实现方案**:
/// 使用 `RwLock<HashMap>` 存储不同配置的连接池。
/// 实现了单例模式 (`instance`)，确保全局唯一。
pub struct PoolManager {
    pools: RwLock<HashMap<String, PoolVariant>>,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: RwLock::new(HashMap::new()),
        }
    }

    /// 获取全局单例
    pub fn instance() -> &'static Self {
        static INSTANCE: Lazy<PoolManager> = Lazy::new(PoolManager::new);
        &INSTANCE
    }

    /// 获取 Oracle 连接池
    ///
    /// **实现方案**:
    /// 1. 计算连接 Key。
    /// 2. **读路径**: 尝试获取读锁，如果池已存在则直接返回。
    /// 3. **写路径**: 获取写锁，再次检查池是否存在（Double-Checked Locking）。
    /// 4. 如果不存在，创建新的 Oracle 连接池并存入 Map。
    ///
    /// **关键问题点**:
    /// - 并发性能：读多写少场景下，使用 `RwLock` 减少锁竞争。
    pub fn get_oracle_pool(
        &self,
        config: &DbConfig,
    ) -> Result<Pool<crate::resources::oracle_manager::OracleConnectionManager>, String> {
        if config.db_type != DbType::Oracle {
            return Err("Invalid DB Type for Oracle Pool".to_string());
        }

        let key = config.connection_key();

        // 1. 尝试读锁获取 (Fast Path)
        {
            let map = self.pools.read().map_err(|e| e.to_string())?;
            if let Some(PoolVariant::Oracle(pool)) = map.get(&key) {
                return Ok(pool.clone());
            }
        }

        // 2. 写锁创建 (Slow Path)
        let mut map = self.pools.write().map_err(|e| e.to_string())?;

        // Double-check locking
        if let Some(PoolVariant::Oracle(pool)) = map.get(&key) {
            return Ok(pool.clone());
        }

        // 创建新池
        crate::app_log!("Creating new Oracle Connection Pool for key: {}", key);

        // Construct Easy Connect string: //host:port/service_name
        let service = config.service.as_deref().unwrap_or("ORCL");

        let manager = crate::resources::oracle_manager::OracleConnectionManager::new(
            &config.user,
            &config.pass,
            &config.host,
            config.port,
            service,
        );

        let pool = Pool::builder()
            .max_size(config.max_pool_size)
            .build(manager)
            .map_err(|e| format!("Failed to create Oracle pool: {}", e))?;

        map.insert(key, PoolVariant::Oracle(pool.clone()));
        Ok(pool)
    }

    /// 获取 Yashan 连接池
    ///
    /// **实现方案**:
    /// 逻辑同 `get_oracle_pool`，但适配了 YashanDB 的连接字符串格式。
    pub fn get_yashan_pool(
        &self,
        config: &DbConfig,
    ) -> Result<Pool<YashanConnectionManager>, String> {
        if config.db_type != DbType::Yashan {
            return Err("Invalid DB Type for Yashan Pool".to_string());
        }

        let key = config.connection_key();

        // 1. 尝试读锁获取 (Fast Path)
        {
            let map = self.pools.read().map_err(|e| e.to_string())?;
            if let Some(PoolVariant::Yashan(pool)) = map.get(&key) {
                return Ok(pool.clone());
            }
        }

        // 2. 写锁创建 (Slow Path)
        let mut map = self.pools.write().map_err(|e| e.to_string())?;

        // Double-check locking
        if let Some(PoolVariant::Yashan(pool)) = map.get(&key) {
            return Ok(pool.clone());
        }

        // 创建新池
        crate::app_log!("Creating new Yashan Connection Pool for key: {}", key);

        // Yashan connection string format
        let conn_str = if let Some(service) = config.service.as_deref() {
            if service.trim().is_empty() {
                format!(
                    "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};",
                    config.host, config.port, config.user, config.pass
                )
            } else {
                format!(
                    "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};Database={};",
                    config.host, config.port, config.user, config.pass, service
                )
            }
        } else {
            format!(
                "Driver={{YashanDB}};Server={};Port={};Uid={};Pwd={};",
                config.host, config.port, config.user, config.pass
            )
        };

        let manager = YashanConnectionManager::new(conn_str);

        let pool = Pool::builder()
            .max_size(config.max_pool_size)
            .build(manager)
            .map_err(|e| format!("Failed to create Yashan pool: {}", e))?;

        map.insert(key, PoolVariant::Yashan(pool.clone()));
        Ok(pool)
    }
}

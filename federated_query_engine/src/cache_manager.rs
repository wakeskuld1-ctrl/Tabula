use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
// **[2026-02-26]** 变更原因：需要跨平台读取系统内存与磁盘信息。
// **[2026-02-26]** 变更目的：替代平台相关命令，降低外部依赖。
// **[2026-02-26]** 变更说明：依赖已在 Cargo.toml 中声明。
// **[2026-02-26]** 变更说明：仅用于系统信息读取，不影响缓存语义。
// **[2026-02-26]** 变更说明：与测试钩子配合可稳定断言。
use sysinfo::{Disks, System};

// Volatility Tracking
// Tracks table update frequency to bypass cache for volatile tables.
#[derive(Debug, Clone)]
pub struct VolatilityStats {
    pub last_known_mtime: u64, // The actual mtime value from FS
    pub last_change_ts: u64,   // System time when we detected the change
    pub update_count: u32,     // Number of updates in current window
    pub is_volatile: bool,     // Circuit breaker status
    pub cooldown_start: u64,   // When volatility mode started
}

impl VolatilityStats {
    fn new(now: u64, mtime: u64) -> Self {
        Self {
            last_known_mtime: mtime,
            last_change_ts: now,
            update_count: 0,
            is_volatile: false,
            cooldown_start: 0,
        }
    }
}

// Global Volatility Tracker
// Key: Table Name
type VolatilityTracker = RwLock<HashMap<String, VolatilityStats>>;

static VOLATILITY_TRACKER: OnceLock<VolatilityTracker> = OnceLock::new();

fn get_volatility_tracker() -> &'static VolatilityTracker {
    VOLATILITY_TRACKER.get_or_init(|| RwLock::new(HashMap::new()))
}

// Constants for Volatility Logic
const VOLATILITY_WINDOW_MS: u64 = 10_000; // 10 seconds window
const VOLATILITY_THRESHOLD: u32 = 3; // 3 updates in window -> Volatile
const VOLATILITY_COOLDOWN_MS: u64 = 30_000; // 30 seconds cooldown

// Time-To-Idle (TTI) Constants
const PROBATION_TTL_MS: u64 = 30_000; // 30 seconds for low-access items (< 2 hits)
const PROTECTED_TTL_MS: u64 = 300_000; // 5 minutes for high-access items (>= 2 hits)
const MAINTENANCE_INTERVAL_MS: u64 = 10_000; // Check every 10 seconds
                                             // **[2026-02-26]** 变更原因：内存上限读取需要缓存以降低系统调用频率。
                                             // **[2026-02-26]** 变更目的：避免每次写入都刷新系统信息造成抖动。
                                             // **[2026-02-26]** 变更说明：默认刷新间隔 5 分钟，可在测试中覆盖。
                                             // **[2026-02-26]** 变更说明：仅影响内部阈值读取，不改变外部接口。
                                             // **[2026-02-26]** 变更说明：与 MEMORY_LIMIT_LAST_REFRESH_MS 联动使用。
const MEMORY_LIMIT_REFRESH_MS: u64 = 300_000;
// **[2026-02-26]** 变更原因：L1 驱逐检查需避免高频触发。
// **[2026-02-26]** 变更目的：降低磁盘扫描开销与日志噪音。
// **[2026-02-26]** 变更说明：默认间隔与维护周期保持一致。
// **[2026-02-26]** 变更说明：测试可通过 TEST_L1_EVICTION_INTERVAL_MS 覆盖。
// **[2026-02-26]** 变更说明：不改变驱逐阈值与策略。
const L1_EVICTION_CHECK_INTERVAL_MS: u64 = 10_000;
// **[2026-02-26]** 变更原因：L2 合并在大批次场景下成本过高。
// **[2026-02-26]** 变更目的：使用阈值限制合并，控制内存拷贝。
// **[2026-02-26]** 变更说明：阈值基于批次总大小判断。
// **[2026-02-26]** 变更说明：测试可通过 TEST_L2_COMPACTION_MAX_BYTES 覆盖。
// **[2026-02-26]** 变更说明：不改变原有缓存内容与顺序。
const L2_COMPACTION_MAX_BYTES: usize = 256 * 1024;

#[derive(Debug, PartialEq)]
pub enum CachePolicy {
    UseCache,
    Bypass,
}

use crate::datasources::excel::ExcelDataSource;
use datafusion::arrow::compute::concat_batches;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::error::{DataFusionError, Result};
use datafusion::parquet::arrow::ArrowWriter;
use datafusion::parquet::file::properties::WriterProperties;
use datafusion::prelude::*;

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

// Singleflight / Request Coalescing Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlightStatus {
    InProgress,
    Completed,
    Failed,
}

// Map of In-Flight Requests
// Key: Cache Key
// Value: Watch Sender (to notify followers)
type InFlightRegistry = std::sync::Mutex<HashMap<String, Arc<watch::Sender<FlightStatus>>>>;

static INFLIGHT_REGISTRY: OnceLock<InFlightRegistry> = OnceLock::new();

fn get_inflight_registry() -> &'static InFlightRegistry {
    INFLIGHT_REGISTRY.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

// Flight Guard: Ensures the flight is removed from registry when dropped
pub struct FlightGuard {
    key: String,
    sender: Arc<watch::Sender<FlightStatus>>,
}

impl FlightGuard {
    pub fn mark_completed(&self) {
        let _ = self.sender.send(FlightStatus::Completed);
    }

    pub fn mark_failed(&self) {
        let _ = self.sender.send(FlightStatus::Failed);
    }
}

impl Drop for FlightGuard {
    fn drop(&mut self) {
        // Remove from registry
        let mut registry = get_inflight_registry().lock().unwrap();
        registry.remove(&self.key);
        // Ensure final status is sent if not already (default to Failed if dropped unexpectedly)
        if *self.sender.borrow() == FlightStatus::InProgress {
            let _ = self.sender.send(FlightStatus::Failed);
        }
    }
}

// ==========================================
// Metrics Registry
// ==========================================

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub query_count: u64,
    pub total_query_latency_us: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub l2_read_latency_us: u64,
    pub l2_lock_wait_us: u64,
    pub l2_eviction_count: u64,
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l1_io_latency_us: u64,
    pub l1_eviction_count: u64,
    pub l0_requests: u64,
    pub l0_exec_latency_us: u64,
    pub memory_usage: usize,
}

pub struct MetricsRegistry {
    pub query_count: AtomicU64,
    pub total_query_latency_us: AtomicU64,
    pub l2_hits: AtomicU64,
    pub l2_misses: AtomicU64,
    pub l2_read_latency_us: AtomicU64,
    pub l2_lock_wait_us: AtomicU64,
    pub l2_eviction_count: AtomicU64,
    pub l1_hits: AtomicU64,
    pub l1_misses: AtomicU64,
    pub l1_io_latency_us: AtomicU64,
    pub l1_eviction_count: AtomicU64,
    pub l0_requests: AtomicU64,
    pub l0_exec_latency_us: AtomicU64,
}

impl MetricsRegistry {
    fn new() -> Self {
        Self {
            query_count: AtomicU64::new(0),
            total_query_latency_us: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l2_misses: AtomicU64::new(0),
            l2_read_latency_us: AtomicU64::new(0),
            l2_lock_wait_us: AtomicU64::new(0),
            l2_eviction_count: AtomicU64::new(0),
            l1_hits: AtomicU64::new(0),
            l1_misses: AtomicU64::new(0),
            l1_io_latency_us: AtomicU64::new(0),
            l1_eviction_count: AtomicU64::new(0),
            l0_requests: AtomicU64::new(0),
            l0_exec_latency_us: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            query_count: self.query_count.load(Ordering::Relaxed),
            total_query_latency_us: self.total_query_latency_us.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.l2_misses.load(Ordering::Relaxed),
            l2_read_latency_us: self.l2_read_latency_us.load(Ordering::Relaxed),
            l2_lock_wait_us: self.l2_lock_wait_us.load(Ordering::Relaxed),
            l2_eviction_count: self.l2_eviction_count.load(Ordering::Relaxed),
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.l1_misses.load(Ordering::Relaxed),
            l1_io_latency_us: self.l1_io_latency_us.load(Ordering::Relaxed),
            l1_eviction_count: self.l1_eviction_count.load(Ordering::Relaxed),
            l0_requests: self.l0_requests.load(Ordering::Relaxed),
            l0_exec_latency_us: self.l0_exec_latency_us.load(Ordering::Relaxed),
            memory_usage: GLOBAL_MEMORY_USAGE.load(Ordering::Relaxed),
        }
    }

    pub fn record_l2_hit(&self, latency_us: u64) {
        self.l2_hits.fetch_add(1, Ordering::Relaxed);
        self.l2_read_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    pub fn record_l2_miss(&self) {
        self.l2_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_lock_wait(&self, latency_us: u64) {
        self.l2_lock_wait_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    pub fn record_l2_eviction(&self) {
        self.l2_eviction_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l1_eviction(&self) {
        self.l1_eviction_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l1_hit(&self) {
        self.l1_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l1_miss(&self) {
        self.l1_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l1_io_latency(&self, latency_us: u64) {
        self.l1_io_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    pub fn record_l0_request(&self) {
        self.l0_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l0_latency(&self, latency_us: u64) {
        self.l0_exec_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_query_latency(&self, latency_us: u64) {
        self.total_query_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }
}

static METRICS_REGISTRY: OnceLock<MetricsRegistry> = OnceLock::new();

pub fn get_metrics_registry() -> &'static MetricsRegistry {
    METRICS_REGISTRY.get_or_init(MetricsRegistry::new)
}

pub enum FlightResult {
    IsLeader(FlightGuard),
    IsFollower(watch::Receiver<FlightStatus>),
}

// Global L2 Cache (Memory) - Sharded
// Key: MD5 hash
// Value: L2CacheEntry
// type L2Cache = RwLock<HashMap<String, L2CacheEntry>>;

const SHARD_COUNT: usize = 16;
type Shard = RwLock<HashMap<String, L2CacheEntry>>;

pub struct ShardedL2Cache {
    shards: Vec<Shard>,
}

impl ShardedL2Cache {
    fn new() -> Self {
        let mut shards = Vec::with_capacity(SHARD_COUNT);
        for _ in 0..SHARD_COUNT {
            shards.push(RwLock::new(HashMap::new()));
        }
        Self { shards }
    }

    fn get_shard(&self, key: &str) -> &Shard {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        &self.shards[(hash as usize) % SHARD_COUNT]
    }
}

// Global L1 Cache Index (Metadata for Disk Cache)
// Key: MD5 hash
// Value: L1CacheEntry (Metadata only)
type L1CacheIndex = RwLock<HashMap<String, L1CacheEntry>>;

fn get_l1_cache_index() -> &'static L1CacheIndex {
    L1_CACHE_INDEX.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Unified Score Calculation
/// Higher Score = Keep
/// Score = (ln(Cost) - ln(Size)) + 4.6 * Priority + (Now - Epoch)
/// Equivalent to: ln(Cost * (1/Size)) ...
fn calculate_static_score(cost: u64, size: usize, priority: f32) -> f32 {
    let c = (cost as f32).max(1.0);
    let s = (size as f32).max(1.0);
    // Size is in denominator (1/size), so larger size -> lower score -> evict
    c.ln() - s.ln() + 4.6 * priority
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct L1CacheEntry {
    pub file_path: PathBuf,
    pub size: u64, // bytes
    pub cost: u64, // ms
    pub priority: f32,
    pub last_access: AtomicU64,
    pub access_count: AtomicU32,
    pub score: AtomicU32,
    pub static_score: f32,
}

impl L1CacheEntry {
    pub fn new(file_path: PathBuf, size: u64, cost: u64, priority: f32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let static_score = calculate_static_score(cost, size as usize, priority);
        let now_sec = (now.saturating_sub(EPOCH_START) as f32) / 1000.0;
        let score = static_score + now_sec;

        Self {
            file_path,
            size,
            cost,
            priority,
            last_access: AtomicU64::new(now),
            access_count: AtomicU32::new(1),
            score: AtomicU32::new(score.to_bits()),
            static_score,
        }
    }

    pub fn update_access(&self, now: u64) {
        let now_sec = (now.saturating_sub(EPOCH_START) as f32) / 1000.0;
        let new_score = self.static_score + now_sec;
        self.score.store(new_score.to_bits(), Ordering::Relaxed);
        self.last_access.store(now, Ordering::Relaxed);
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_score(&self) -> f32 {
        f32::from_bits(self.score.load(Ordering::Relaxed))
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct L2CacheEntry {
    pub data: Vec<RecordBatch>,
    pub cost: u64,              // ms
    pub size: usize,            // bytes
    pub priority: f32,          // default 1.0
    pub last_access: AtomicU64, // ms timestamp
    pub access_count: AtomicU32,
    pub score: AtomicU32,  // Absolute score (f32 bits)
    pub static_score: f32, // (ln C - ln S) + 4.6 P
}

// 2025-01-01 00:00:00 UTC
const EPOCH_START: u64 = 1_735_689_600_000;

impl L2CacheEntry {
    pub fn new(data: Vec<RecordBatch>, cost: u64, priority: f32) -> Self {
        let size: usize = data.iter().map(|b| b.get_array_memory_size()).sum();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Use unified calculation
        let static_score = calculate_static_score(cost, size, priority);

        // Use relative time in seconds to fit in f32 with good precision
        // If we use raw millis, f32 precision is too low for recent updates.
        let now_sec = (now.saturating_sub(EPOCH_START) as f32) / 1000.0;

        let score = static_score + now_sec;

        Self {
            data,
            cost,
            size,
            priority,
            last_access: AtomicU64::new(now),
            access_count: AtomicU32::new(1),
            score: AtomicU32::new(score.to_bits()),
            static_score,
        }
    }

    pub fn update_access(&self, now: u64) {
        // Use relative time in seconds
        let now_sec = (now.saturating_sub(EPOCH_START) as f32) / 1000.0;
        // Update score: replace old time component with new one
        // score = static_score + now_sec
        let new_score = self.static_score + now_sec;

        self.score.store(new_score.to_bits(), Ordering::Relaxed);
        self.last_access.store(now, Ordering::Relaxed);
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_score(&self) -> f32 {
        f32::from_bits(self.score.load(Ordering::Relaxed))
    }
}

static L2_CACHE: OnceLock<ShardedL2Cache> = OnceLock::new();
static L1_CACHE_INDEX: OnceLock<L1CacheIndex> = OnceLock::new();

static GLOBAL_MEMORY_USAGE: AtomicUsize = AtomicUsize::new(0);
static IS_EVICTING: AtomicBool = AtomicBool::new(false);
static L1_EVICTION_LAST_CHECK_MS: AtomicU64 = AtomicU64::new(0);
// **[2026-02-26]** 变更原因：内存上限读取需要缓存结果。
// **[2026-02-26]** 变更目的：减少系统信息读取频率。
// **[2026-02-26]** 变更说明：0 表示未缓存，触发首次刷新。
// **[2026-02-26]** 变更说明：与刷新时间戳配套使用。
// **[2026-02-26]** 变更说明：仅用于内部阈值计算。
static MEMORY_LIMIT_CACHE: AtomicUsize = AtomicUsize::new(0);
// **[2026-02-26]** 变更原因：需要判断内存上限缓存是否过期。
// **[2026-02-26]** 变更目的：为刷新逻辑提供时间基准。
// **[2026-02-26]** 变更说明：时间单位为毫秒。
// **[2026-02-26]** 变更说明：0 表示未刷新状态。
// **[2026-02-26]** 变更说明：仅影响内部判断。
static MEMORY_LIMIT_LAST_REFRESH_MS: AtomicU64 = AtomicU64::new(0);

// Test Hooks
static TEST_MEMORY_LIMIT: RwLock<Option<usize>> = RwLock::new(None);
static TEST_DISK_USAGE: RwLock<Option<(u64, u64)>> = RwLock::new(None);
static TEST_L1_EVICTION_INTERVAL_MS: RwLock<Option<u64>> = RwLock::new(None);
// **[2026-02-26]** 变更原因：测试需要控制内存上限刷新间隔。
// **[2026-02-26]** 变更目的：避免测试等待真实时间。
// **[2026-02-26]** 变更说明：None 表示使用默认常量。
// **[2026-02-26]** 变更说明：仅测试使用。
// **[2026-02-26]** 变更说明：不影响生产逻辑。
static TEST_MEMORY_LIMIT_REFRESH_INTERVAL_MS: RwLock<Option<u64>> = RwLock::new(None);
// **[2026-02-26]** 变更原因：测试需要控制 L2 合并阈值。
// **[2026-02-26]** 变更目的：用小数据覆盖分支逻辑。
// **[2026-02-26]** 变更说明：None 表示使用默认常量。
// **[2026-02-26]** 变更说明：仅测试使用。
// **[2026-02-26]** 变更说明：不影响生产逻辑。
static TEST_L2_COMPACTION_MAX_BYTES: RwLock<Option<usize>> = RwLock::new(None);
// **[2026-02-26]** 变更原因：测试需要稳定的总内存值。
// **[2026-02-26]** 变更目的：避免环境差异导致断言波动。
// **[2026-02-26]** 变更说明：None 表示使用系统值。
// **[2026-02-26]** 变更说明：仅测试使用。
// **[2026-02-26]** 变更说明：不影响生产逻辑。
static TEST_TOTAL_MEMORY_BYTES: RwLock<Option<usize>> = RwLock::new(None);

fn get_l2_cache() -> &'static ShardedL2Cache {
    L2_CACHE.get_or_init(|| ShardedL2Cache::new())
}

pub struct CacheManager;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

static BYPASS_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn get_bypass_semaphore() -> &'static Arc<Semaphore> {
    BYPASS_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(10))) // Max 10 concurrent bypass queries
}

impl CacheManager {
    #[allow(dead_code)]
    pub fn set_test_memory_limit(limit: Option<usize>) {
        *TEST_MEMORY_LIMIT.write().unwrap() = limit;
    }

    #[allow(dead_code)]
    pub fn set_test_disk_usage(usage: Option<(u64, u64)>) {
        *TEST_DISK_USAGE.write().unwrap() = usage;
    }

    #[allow(dead_code)]
    pub fn set_test_l1_eviction_interval_ms(interval_ms: Option<u64>) {
        *TEST_L1_EVICTION_INTERVAL_MS.write().unwrap() = interval_ms;
    }

    #[allow(dead_code)]
    // **[2026-02-26]** 变更原因：测试需要控制内存上限刷新间隔。
    // **[2026-02-26]** 变更目的：避免测试等待真实时间。
    // **[2026-02-26]** 变更说明：None 回退默认常量。
    // **[2026-02-26]** 变更说明：仅测试使用。
    // **[2026-02-26]** 变更说明：不影响生产逻辑。
    pub fn set_test_memory_limit_refresh_interval_ms(interval_ms: Option<u64>) {
        *TEST_MEMORY_LIMIT_REFRESH_INTERVAL_MS.write().unwrap() = interval_ms;
    }

    #[allow(dead_code)]
    // **[2026-02-26]** 变更原因：测试需要控制 L2 合并阈值。
    // **[2026-02-26]** 变更目的：使用小数据覆盖分支逻辑。
    // **[2026-02-26]** 变更说明：None 回退默认常量。
    // **[2026-02-26]** 变更说明：仅测试使用。
    // **[2026-02-26]** 变更说明：不影响生产逻辑。
    pub fn set_test_l2_compaction_max_bytes(max_bytes: Option<usize>) {
        *TEST_L2_COMPACTION_MAX_BYTES.write().unwrap() = max_bytes;
    }

    #[allow(dead_code)]
    // **[2026-02-26]** 变更原因：测试需要固定总内存值。
    // **[2026-02-26]** 变更目的：避免环境差异导致断言波动。
    // **[2026-02-26]** 变更说明：None 回退系统值。
    // **[2026-02-26]** 变更说明：仅测试使用。
    // **[2026-02-26]** 变更说明：不影响生产逻辑。
    pub fn set_test_total_memory_bytes(total_bytes: Option<usize>) {
        *TEST_TOTAL_MEMORY_BYTES.write().unwrap() = total_bytes;
    }

    #[allow(dead_code)]
    pub fn clear_l2() {
        let cache = get_l2_cache();
        for shard in &cache.shards {
            shard.write().unwrap().clear();
        }
        GLOBAL_MEMORY_USAGE.store(0, Ordering::Release);
    }

    #[allow(dead_code)]
    pub fn clear_l1() {
        let _ = std::fs::remove_dir_all("cache/l1");
        get_l1_cache_index().write().unwrap().clear();
    }

    #[allow(dead_code)]
    pub fn reset_volatility() {
        get_volatility_tracker().write().unwrap().clear();
    }

    pub async fn acquire_bypass_permit() -> OwnedSemaphorePermit {
        get_bypass_semaphore()
            .clone()
            .acquire_owned()
            .await
            .unwrap()
    }

    // **[2026-02-26]** 变更原因：多个节流与缓存逻辑需要统一的时间来源。
    // **[2026-02-26]** 变更目的：避免重复写入时间获取逻辑。
    // **[2026-02-26]** 变更说明：单位为毫秒。
    // **[2026-02-26]** 变更说明：用于测试与生产路径一致性。
    // **[2026-02-26]** 变更说明：不改变时间精度。
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Check table volatility and determine cache policy.
    /// Returns CachePolicy::Bypass if table is updating too frequently.
    /// Optimized with Read-Before-Write lock to reduce contention.
    ///
    /// `current_mtime` can be:
    /// - File Modification Time (SQLite/Excel)
    /// - Oracle SCN (System Change Number)
    /// - MySQL/PG Last Update Timestamp
    pub fn check_volatility(table_name: &str, current_mtime: u64) -> CachePolicy {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 1. Fast Path: Read Lock
        {
            let tracker = get_volatility_tracker().read().unwrap();
            if let Some(stats) = tracker.get(table_name) {
                // If mtime matches, no update needed usually
                if stats.last_known_mtime == current_mtime {
                    // Special case: Cooldown check might need write, but we can check condition first
                    if stats.is_volatile {
                        if now - stats.cooldown_start > VOLATILITY_COOLDOWN_MS {
                            // Need to upgrade to write lock to reset
                        } else {
                            // Still volatile and in cooldown
                            return CachePolicy::Bypass;
                        }
                    } else {
                        // Stable and not volatile
                        return CachePolicy::UseCache;
                    }
                }
            }
        }

        // 2. Slow Path: Write Lock
        let mut tracker = get_volatility_tracker().write().unwrap();

        let stats = tracker
            .entry(table_name.to_string())
            .or_insert_with(|| VolatilityStats::new(now, current_mtime));

        // ... (rest of the logic is same as before)
        if stats.is_volatile {
            if now - stats.cooldown_start > VOLATILITY_COOLDOWN_MS {
                // Cooldown over, check if stable
                if current_mtime == stats.last_known_mtime {
                    // Still same mtime, looks stable. Reset.
                    println!(
                        "[CacheManager] Table '{}' stabilized. Resuming cache.",
                        table_name
                    );
                    stats.is_volatile = false;
                    stats.update_count = 0;
                    stats.last_known_mtime = current_mtime;
                    stats.last_change_ts = now;
                    return CachePolicy::UseCache;
                } else {
                    // Changed again during cooldown?
                    // If mtime changed since last check, extend cooldown
                    if current_mtime != stats.last_known_mtime {
                        println!(
                            "[CacheManager] Table '{}' still changing. Extending cooldown.",
                            table_name
                        );
                        stats.last_known_mtime = current_mtime;
                        stats.cooldown_start = now;
                    }
                    return CachePolicy::Bypass;
                }
            } else {
                // In cooldown period, update mtime if changed to keep tracking
                if current_mtime != stats.last_known_mtime {
                    stats.last_known_mtime = current_mtime;
                    stats.cooldown_start = now; // Extend cooldown
                }
                return CachePolicy::Bypass;
            }
        }

        // 2. Normal Mode: Detect Frequency
        if current_mtime != stats.last_known_mtime {
            // Data changed
            let time_since_last_change = now.saturating_sub(stats.last_change_ts);

            if time_since_last_change < VOLATILITY_WINDOW_MS {
                stats.update_count += 1;
                println!(
                    "[CacheManager] Table '{}' updated. Count: {}/{}",
                    table_name, stats.update_count, VOLATILITY_THRESHOLD
                );

                if stats.update_count >= VOLATILITY_THRESHOLD {
                    println!("[CacheManager] Table '{}' is VOLATILE (Frequent Updates). Bypassing cache for {}s.", table_name, VOLATILITY_COOLDOWN_MS / 1000);
                    stats.is_volatile = true;
                    stats.cooldown_start = now;

                    // Update state
                    stats.last_known_mtime = current_mtime;
                    stats.last_change_ts = now;
                    return CachePolicy::Bypass;
                }
            } else {
                // Reset window
                stats.update_count = 1;
            }

            // Update last known state
            stats.last_known_mtime = current_mtime;
            stats.last_change_ts = now;
        }

        CachePolicy::UseCache
    }

    /// Generates a cache key based on the table name, query parameters, projection, and source version (mtime/SCN).
    /// Key format: MD5(table_name + "|" + params + "|" + projection + "|" + mtime)
    pub fn generate_key(
        table_name: &str,
        params: Option<&str>,
        projection: Option<&Vec<usize>>,
        source_mtime: u64,
    ) -> String {
        let mut context = md5::Context::new();
        context.consume(table_name.as_bytes());
        context.consume(b"|");

        // Canonicalize params (SQL conditions) to avoid cache duplication
        if let Some(p) = params {
            let canonical_params = Self::canonicalize_sql(p);
            context.consume(canonical_params.as_bytes());
        }

        context.consume(b"|");
        if let Some(proj) = projection {
            for idx in proj {
                context.consume(idx.to_le_bytes());
            }
        }
        context.consume(b"|");
        context.consume(source_mtime.to_le_bytes()); // Include mtime in hash
        let digest = context.finalize();
        format!("{:x}", digest)
    }

    /// Canonicalize SQL condition string to ensure consistent cache keys.
    /// E.g., "a=1 AND b=2" -> "a=1 AND b=2"
    ///       "b=2 AND a=1" -> "a=1 AND b=2"
    fn canonicalize_sql(sql: &str) -> String {
        let s = sql.trim();

        // 1. Handle Parentheses Wrapper: (A) -> A
        // Only if parentheses wrap the ENTIRE string
        if s.starts_with('(') && s.ends_with(')') {
            if Self::is_balanced_wrapper(s) {
                return Self::canonicalize_sql(&s[1..s.len() - 1]);
            }
        }

        // 2. Split by OR (Lowest Precedence)
        // We must process OR first because it has lower precedence than AND.
        // E.g. "A AND B OR C AND D" is "(A AND B) OR (C AND D)"
        let or_parts = Self::split_ignore_nested(s, " OR ");
        if or_parts.len() > 1 {
            let mut parts: Vec<String> = or_parts
                .into_iter()
                .map(|p| Self::canonicalize_sql(&p))
                .collect();
            parts.sort();
            return parts.join(" OR ");
        }

        // 3. Split by AND
        let and_parts = Self::split_ignore_nested(s, " AND ");
        if and_parts.len() > 1 {
            let mut parts: Vec<String> = and_parts
                .into_iter()
                .map(|p| {
                    let c = Self::canonicalize_sql(&p);
                    // If the sub-part contains OR (lower precedence), it must be wrapped
                    // to be safe inside an AND clause.
                    // E.g. If we have "(A OR B) AND C", split by AND gives "A OR B" and "C".
                    // "A OR B" must be wrapped back to "(A OR B)".
                    if Self::split_ignore_nested(&c, " OR ").len() > 1 {
                        format!("({})", c)
                    } else {
                        c
                    }
                })
                .collect();
            parts.sort();
            return parts.join(" AND ");
        }

        // 4. Normalize Whitespace (leaf node)
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Check if the outer parentheses pair matches each other (wraps the whole content).
    fn is_balanced_wrapper(s: &str) -> bool {
        if !s.starts_with('(') || !s.ends_with(')') {
            return false;
        }
        let mut depth = 0;
        let bytes = s.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
                // If depth hits 0 before the end, it means the first '(' closed early
                // e.g. "(a) AND (b)" -> depth hits 0 at first ')'
                if depth == 0 && i < bytes.len() - 1 {
                    return false;
                }
            }
        }
        depth == 0
    }

    /// Split string by delimiter, ignoring nested parentheses
    fn split_ignore_nested(s: &str, delimiter: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        let s_lower = s.to_ascii_uppercase(); // For case-insensitive delimiter check
        let delim_len = delimiter.len();

        // Iterate through string using indices
        // Note: Simple byte scan is safe for ASCII delimiters and parenthesis.
        // Multibyte chars won't match ASCII delimiters.
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < s.len() {
            let b = bytes[i];
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                if depth > 0 {
                    depth -= 1;
                }
            } else if depth == 0 {
                // Check delimiter
                if i + delim_len <= s.len() && &s_lower[i..i + delim_len] == delimiter {
                    parts.push(s[start..i].to_string());
                    start = i + delim_len;
                    i += delim_len - 1; // Skip delimiter
                }
            }
            i += 1;
        }
        parts.push(s[start..].to_string());
        parts
    }

    /// Returns the path to the cache file for a given key.
    pub fn get_cache_file_path(key: &str) -> PathBuf {
        Path::new("cache")
            .join("l1")
            .join(format!("{}.parquet", key))
    }

    /// Retrieve data from L2 Cache (Memory)
    pub fn get_l2(key: &str) -> Option<Vec<RecordBatch>> {
        let start = std::time::Instant::now();
        let metrics = get_metrics_registry();

        // Use read lock for high concurrency (Shard Level)
        let cache = get_l2_cache();
        let shard = cache.get_shard(key).read().unwrap();
        metrics.record_lock_wait(start.elapsed().as_micros() as u64);

        if let Some(entry) = shard.get(key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            // Update metadata atomically (interior mutability) without write lock
            entry.update_access(now);

            metrics.record_l2_hit(start.elapsed().as_micros() as u64);
            Some(entry.data.clone())
        } else {
            metrics.record_l2_miss();
            None
        }
    }

    /// Retrieve cache file path (L1) and update metadata access
    pub fn get_l1_file(key: &str) -> Option<PathBuf> {
        let path = Self::get_cache_file_path(key);
        if path.exists() {
            // Update L1 metadata access
            let cache = get_l1_cache_index().read().unwrap();
            if let Some(entry) = cache.get(key) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                entry.update_access(now);
            }
            get_metrics_registry().record_l1_hit();
            Some(path)
        } else {
            get_metrics_registry().record_l1_miss();
            None
        }
    }

    /// Singleflight: Join an existing flight or start a new one
    pub fn join_or_start_flight(key: String) -> FlightResult {
        let mut registry = get_inflight_registry().lock().unwrap();

        if let Some(sender) = registry.get(&key) {
            // Flight exists -> Follower
            return FlightResult::IsFollower(sender.subscribe());
        }

        // Flight missing -> Leader
        let (tx, _rx) = watch::channel(FlightStatus::InProgress);
        let sender = Arc::new(tx);
        registry.insert(key.clone(), sender.clone());

        FlightResult::IsLeader(FlightGuard { key, sender })
    }

    /// Check if L1 entry should be promoted to L2 based on access frequency

    /// Check if L1 entry should be promoted to L2 based on access frequency
    pub fn should_promote_to_l2(key: &str) -> bool {
        if let Some(entry) = get_l1_cache_index().read().unwrap().get(key) {
            let count = entry.access_count.load(Ordering::Relaxed);
            // Promote if accessed > 2 times OR high priority
            // This prevents "One-Hit Wonders" (scans) from polluting L2 memory
            if count > 2 || entry.priority > 1.0 {
                return true;
            }
        }
        false
    }

    /// Register L1 Cache File (called after writing to disk)
    pub fn put_l1(key: String, file_path: PathBuf, size: u64, cost: u64) {
        let entry = L1CacheEntry::new(file_path, size, cost, 1.0);
        let mut cache = get_l1_cache_index().write().unwrap();
        cache.insert(key, entry);

        Self::maybe_check_l1_disk_eviction();
    }

    fn l1_eviction_check_interval_ms() -> u64 {
        if let Some(interval) = *TEST_L1_EVICTION_INTERVAL_MS.read().unwrap() {
            return interval;
        }
        L1_EVICTION_CHECK_INTERVAL_MS
    }

    // **[2026-02-26]** 变更原因：L1 驱逐检查需要节流。
    // **[2026-02-26]** 变更目的：避免频繁磁盘扫描与锁争用。
    // **[2026-02-26]** 变更说明：仅当超过间隔才触发检查。
    // **[2026-02-26]** 变更说明：通过原子时间戳记录最近检查。
    // **[2026-02-26]** 变更说明：不改变驱逐策略与阈值。
    fn maybe_check_l1_disk_eviction() {
        let now = Self::now_ms();
        let last_check = L1_EVICTION_LAST_CHECK_MS.load(Ordering::Relaxed);
        let interval = Self::l1_eviction_check_interval_ms();
        if now.saturating_sub(last_check) < interval {
            return;
        }
        L1_EVICTION_LAST_CHECK_MS.store(now, Ordering::Relaxed);
        Self::check_l1_disk_eviction();
    }

    // **[2026-02-26]** 变更原因：内存上限缓存需要明确刷新周期。
    // **[2026-02-26]** 变更目的：统一测试与生产配置来源。
    // **[2026-02-26]** 变更说明：测试值优先于默认常量。
    // **[2026-02-26]** 变更说明：返回单位为毫秒。
    // **[2026-02-26]** 变更说明：仅用于内部刷新判断。
    fn memory_limit_refresh_interval_ms() -> u64 {
        if let Some(interval) = *TEST_MEMORY_LIMIT_REFRESH_INTERVAL_MS.read().unwrap() {
            return interval;
        }
        MEMORY_LIMIT_REFRESH_MS
    }

    // **[2026-02-26]** 变更原因：需要跨平台获取系统总内存。
    // **[2026-02-26]** 变更目的：移除平台命令依赖。
    // **[2026-02-26]** 变更说明：测试优先使用固定值。
    // **[2026-02-26]** 变更说明：sysinfo 返回单位为 KB。
    // **[2026-02-26]** 变更说明：失败时回退 1GB 以保持可用性。
    fn get_total_memory_bytes() -> usize {
        if let Some(total) = *TEST_TOTAL_MEMORY_BYTES.read().unwrap() {
            return total;
        }

        let mut system = System::new_all();
        system.refresh_memory();
        let total_kb = system.total_memory() as usize;
        if total_kb > 0 {
            return total_kb.saturating_mul(1024);
        }

        println!("[CacheManager] Warning: Failed to get memory via sysinfo. Using fallback 1GB.");
        1024 * 1024 * 1024
    }

    // **[2026-02-26]** 变更原因：盘符解析仅适用于 Windows。
    // **[2026-02-26]** 变更目的：改为跨平台的挂载点匹配逻辑。
    // **[2026-02-26]** 变更说明：优先选择最长匹配挂载点。
    // **[2026-02-26]** 变更说明：获取失败则返回 0 值避免误判。
    // **[2026-02-26]** 变更说明：与 sysinfo 磁盘列表匹配。
    fn get_disk_usage_via_sysinfo(cache_dir: &Path) -> (u64, u64) {
        let disks = Disks::new_with_refreshed_list();
        let mut best_match: Option<(usize, u64, u64)> = None;

        for disk in disks.list() {
            let mount_point = disk.mount_point();
            if cache_dir.starts_with(mount_point) {
                let mount_len = mount_point.as_os_str().len();
                let candidate = (mount_len, disk.total_space(), disk.available_space());
                if best_match
                    .as_ref()
                    .map(|(len, _, _)| mount_len > *len)
                    .unwrap_or(true)
                {
                    best_match = Some(candidate);
                }
            }
        }

        if let Some((_, total, free)) = best_match {
            return (total, free);
        }

        (0, 0)
    }

    // **[2026-02-26]** 变更原因：内存上限读取需要带缓存刷新策略。
    // **[2026-02-26]** 变更目的：减少系统信息读取频率。
    // **[2026-02-26]** 变更说明：缓存命中时直接返回缓存值。
    // **[2026-02-26]** 变更说明：过期时刷新缓存并更新时间戳。
    // **[2026-02-26]** 变更说明：仅影响内部阈值计算。
    fn get_memory_limit_bytes_with_now(now_ms: u64) -> usize {
        if let Some(limit) = *TEST_MEMORY_LIMIT.read().unwrap() {
            return limit;
        }

        let last_refresh = MEMORY_LIMIT_LAST_REFRESH_MS.load(Ordering::Relaxed);
        let cached_value = MEMORY_LIMIT_CACHE.load(Ordering::Relaxed);
        let refresh_interval = Self::memory_limit_refresh_interval_ms();

        let should_refresh =
            cached_value == 0 || now_ms.saturating_sub(last_refresh) >= refresh_interval;
        if !should_refresh {
            return cached_value;
        }

        let total_memory = Self::get_total_memory_bytes();
        let limit = (total_memory as f64 * 0.70) as usize;
        MEMORY_LIMIT_CACHE.store(limit, Ordering::Relaxed);
        MEMORY_LIMIT_LAST_REFRESH_MS.store(now_ms, Ordering::Relaxed);
        limit
    }

    // **[2026-02-26]** 变更原因：常规调用需要自动注入当前时间。
    // **[2026-02-26]** 变更目的：减少调用侧传参负担。
    // **[2026-02-26]** 变更说明：内部委托带时间版本。
    // **[2026-02-26]** 变更说明：与缓存策略一致。
    // **[2026-02-26]** 变更说明：不影响测试可控性。
    fn get_memory_limit_bytes() -> usize {
        Self::get_memory_limit_bytes_with_now(Self::now_ms())
    }

    // **[2026-02-26]** 变更原因：L2 合并阈值需要测试覆盖能力。
    // **[2026-02-26]** 变更目的：允许用小数据覆盖分支逻辑。
    // **[2026-02-26]** 变更说明：测试值优先，生产使用常量。
    // **[2026-02-26]** 变更说明：不影响其它缓存逻辑。
    // **[2026-02-26]** 变更说明：阈值单位为字节。
    fn l2_compaction_max_bytes() -> usize {
        if let Some(limit) = *TEST_L2_COMPACTION_MAX_BYTES.read().unwrap() {
            return limit;
        }
        L2_COMPACTION_MAX_BYTES
    }

    /// Store data into L2 Cache (Memory) with eviction logic
    pub fn put_l2(key: String, mut batches: Vec<RecordBatch>, cost_ms: u64) {
        // Optimization: Compact small batches into larger ones for sequential read efficiency
        // This reduces fragmentation and improves CPU cache locality during scans.
        // **[2026-02-26]** 变更原因：L2 合并在大批次场景下成本过高。
        // **[2026-02-26]** 变更目的：增加阈值控制，避免大内存拷贝。
        // **[2026-02-26]** 变更说明：仅在总大小较小时才合并。
        // **[2026-02-26]** 变更说明：阈值可在测试中覆盖。
        // **[2026-02-26]** 变更说明：不影响批次内容与顺序。
        let total_batch_bytes: usize = batches.iter().map(|b| b.get_array_memory_size()).sum();
        if batches.len() > 1 && total_batch_bytes <= Self::l2_compaction_max_bytes() {
            if let Some(first) = batches.first() {
                let schema = first.schema();
                match concat_batches(&schema, &batches) {
                    Ok(merged_batch) => {
                        // println!("[CacheManager] Compacted {} batches for key {}", batches.len(), key);
                        batches = vec![merged_batch];
                    }
                    Err(e) => {
                        println!(
                            "[CacheManager] Warning: Failed to compact batches for key {}: {:?}",
                            key, e
                        );
                    }
                }
            }
        }

        // 1. Calculate limits (Read-only check)
        // **[2026-02-26]** 变更原因：内存上限计算需要缓存并跨平台。
        // **[2026-02-26]** 变更目的：减少系统信息读取频率并支持 Linux。
        // **[2026-02-26]** 变更说明：使用带缓存的内存上限计算。
        // **[2026-02-26]** 变更说明：测试可覆盖内存上限值。
        // **[2026-02-26]** 变更说明：不改变上限比例。
        let memory_limit = Self::get_memory_limit_bytes();

        // 2. Insert into Cache (Hold Shard Lock briefly)
        {
            let cache = get_l2_cache();
            let mut shard = cache.get_shard(&key).write().unwrap();

            // Create new entry
            let entry = L2CacheEntry::new(batches, cost_ms, 1.0);
            let entry_size = entry.size;

            // Hard limit check for single item
            if entry_size > memory_limit {
                println!(
                    "[CacheManager] Item too large to cache ({} > {}). Skipping L2.",
                    entry_size, memory_limit
                );
                return;
            }

            GLOBAL_MEMORY_USAGE.fetch_add(entry_size, Ordering::Relaxed);
            shard.insert(key, entry);
        } // Lock dropped here

        // 3. Trigger Async Eviction (Fire & Forget)
        // If limit exceeded, spawn background task.
        // We check IS_EVICTING to avoid spawning too many tasks.
        if !IS_EVICTING.load(Ordering::Relaxed) {
            let current_usage = GLOBAL_MEMORY_USAGE.load(Ordering::Relaxed);
            if current_usage > memory_limit {
                // Try to set flag
                if IS_EVICTING
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    println!(
                        "[CacheManager] Triggering Background Eviction (Usage: {} / Limit: {})",
                        current_usage, memory_limit
                    );
                    tokio::spawn(async move {
                        Self::run_eviction_cycle(memory_limit).await;
                        IS_EVICTING.store(false, Ordering::Release);
                    });
                }
            }
        }
    }

    /// Background Eviction Task
    /// Uses "Sampled Eviction" + "Lock Sharding" to be concurrent-friendly.
    async fn run_eviction_cycle(limit: usize) {
        // Pool configuration
        const EVICTION_POOL_SIZE: usize = 16;
        const SAMPLE_SIZE: usize = 5;

        // Eviction Pool: Stores (key, score).
        let mut pool: Vec<(String, f32)> = Vec::with_capacity(EVICTION_POOL_SIZE);
        let mut shard_index = 0;

        loop {
            // Check global usage
            let current_usage = GLOBAL_MEMORY_USAGE.load(Ordering::Relaxed);
            if current_usage <= limit {
                break;
            }

            let mut evicted_something = false;
            let cache = get_l2_cache();

            // 1. Sample from current shard (Read Lock)
            {
                if let Some(shard) = cache.shards.get(shard_index) {
                    let shard_lock = shard.read().unwrap();
                    if !shard_lock.is_empty() {
                        let sample_keys: Vec<String> =
                            shard_lock.keys().take(SAMPLE_SIZE).cloned().collect();
                        for key in sample_keys {
                            if let Some(entry) = shard_lock.get(&key) {
                                let score = entry.get_score();
                                if !pool.iter().any(|(k, _)| *k == key) {
                                    pool.push((key, score));
                                }
                            }
                        }
                    }
                }
            }

            // Move to next shard for next iteration
            shard_index = (shard_index + 1) % SHARD_COUNT;

            // 2. Sort and Truncate Pool
            pool.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            if pool.len() > EVICTION_POOL_SIZE {
                pool.truncate(EVICTION_POOL_SIZE);
            }

            // 3. Evict Best Candidate from Pool
            while let Some((victim_key, score)) = pool.first().cloned() {
                // Remove from pool first
                pool.remove(0);

                // Identify shard and lock for writing
                let victim_shard = cache.get_shard(&victim_key);
                let mut shard_lock = victim_shard.write().unwrap();

                // Double check usage before evicting? No, we are in eviction loop.
                // Try to remove from cache
                if let Some(entry) = shard_lock.remove(&victim_key) {
                    GLOBAL_MEMORY_USAGE.fetch_sub(entry.size, Ordering::Relaxed);
                    evicted_something = true;
                    let new_usage = GLOBAL_MEMORY_USAGE.load(Ordering::Relaxed);
                    println!(
                        "[CacheManager] Evicted {} (Score: {:.2}). New Usage: {}",
                        victim_key, score, new_usage
                    );
                    get_metrics_registry().record_l2_eviction();
                    break; // Successfully evicted one, yield
                } else {
                    // Key might have been removed by someone else, try next in pool
                    continue;
                }
            }

            if !evicted_something {
                // If pool is empty or all candidates invalid, and we are still over limit
                if pool.is_empty() {
                    // Prevent tight loop if we can't find anything to evict immediately
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }

            // Yield to let other tasks access the cache
            tokio::task::yield_now().await;
        }
    }

    #[cfg(test)]
    pub async fn run_eviction_cycle_for_test(limit: usize) {
        Self::run_eviction_cycle(limit).await;
    }

    /// Start Background Maintenance Task (TTI Eviction)
    pub fn start_maintenance_task() {
        tokio::spawn(async move {
            println!(
                "[CacheManager] Starting Background Maintenance Task (Interval: {} ms)",
                MAINTENANCE_INTERVAL_MS
            );
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(MAINTENANCE_INTERVAL_MS))
                    .await;
                Self::run_ttl_eviction().await;
            }
        });
    }

    /// Two-Tiered TTI Eviction Strategy
    /// - Tier 1 (Probation): If access_count < 2, eviction after 30s idle.
    /// - Tier 2 (Protected): If access_count >= 2, eviction after 5m idle.
    /// Uses "Read-Lock Scan" + "Write-Lock Purge" for minimal blocking.
    async fn run_ttl_eviction() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut victims = Vec::new();

        // 1. Fast Scan with Read Lock (All Shards)
        let cache = get_l2_cache();
        for shard in &cache.shards {
            let shard_lock = shard.read().unwrap();
            for (key, entry) in shard_lock.iter() {
                let last_access = entry.last_access.load(Ordering::Relaxed);
                let idle_time = now.saturating_sub(last_access);
                let count = entry.access_count.load(Ordering::Relaxed);

                // Determine Threshold based on Access Count
                let ttl = if count < 2 {
                    PROBATION_TTL_MS
                } else {
                    PROTECTED_TTL_MS
                };

                if idle_time > ttl {
                    victims.push(key.clone());
                }
            }
        }

        if victims.is_empty() {
            return;
        }

        // 2. Batch Eviction with Write Lock (Per Key)
        for key in victims {
            let shard = cache.get_shard(&key);
            let mut shard_lock = shard.write().unwrap();

            // Re-check condition inside write lock
            if let Some(entry) = shard_lock.get(&key) {
                let last_access = entry.last_access.load(Ordering::Relaxed);
                let idle_time = now.saturating_sub(last_access);
                let count = entry.access_count.load(Ordering::Relaxed);

                let ttl = if count < 2 {
                    PROBATION_TTL_MS
                } else {
                    PROTECTED_TTL_MS
                };

                if idle_time > ttl {
                    if let Some(removed) = shard_lock.remove(&key) {
                        GLOBAL_MEMORY_USAGE.fetch_sub(removed.size, Ordering::Relaxed);
                        println!(
                            "[CacheManager] TTI Eviction: {} (Idle: {}ms, Count: {})",
                            key, idle_time, count
                        );
                        get_metrics_registry().record_l2_eviction();
                    }
                }
            }
        }
    }

    /// Check Disk Space for L1 Cache Eviction
    /// Limit: 80% of Available Disk Space (Total - Free > 0.8 * Total)
    /// Eviction Strategy: Use L1 Metadata Index (Score-based) instead of just Mtime.
    pub fn check_l1_disk_eviction() {
        let cache_dir = Path::new("cache").join("l1");
        if !cache_dir.exists() {
            return;
        }

        // **[2026-02-26]** 变更原因：盘符解析仅适用于 Windows。
        // **[2026-02-26]** 变更目的：改为跨平台的挂载点匹配逻辑。
        // **[2026-02-26]** 变更说明：使用 canonical path 避免软链接影响匹配。
        // **[2026-02-26]** 变更说明：获取失败则直接返回，避免误判。
        // **[2026-02-26]** 变更说明：与 sysinfo 磁盘列表匹配。
        let canonical_cache_dir = match fs::canonicalize(&cache_dir) {
            Ok(path) => path,
            Err(_) => return,
        };

        let (total, free) = if let Some(usage) = *TEST_DISK_USAGE.read().unwrap() {
            usage
        } else {
            Self::get_disk_usage_via_sysinfo(&canonical_cache_dir)
        };

        if total == 0 {
            return;
        }

        let used_pct = 1.0 - (free as f64 / total as f64);

        if used_pct > 0.80 {
            println!(
                "[CacheManager] Disk usage high ({:.2}%). Triggering L1 eviction...",
                used_pct * 100.0
            );

            // 1. Get Access to L1 Metadata Index
            let mut cache_index = get_l1_cache_index().write().unwrap();

            if !cache_index.is_empty() {
                // Strategy: Evict items with lowest scores (Small size + High cost = Keep; Large size + Low cost = Evict)
                // Collect all keys to sort
                let mut entries: Vec<(String, f32)> = cache_index
                    .iter()
                    .map(|(k, v)| (k.clone(), v.get_score()))
                    .collect();

                // Sort by score ascending (lowest score first = victim)
                entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                // Evict bottom 10%
                let count_to_evict = (entries.len() as f64 * 0.10).ceil() as usize;
                println!(
                    "[CacheManager] Evicting {} L1 files (Total: {})",
                    count_to_evict,
                    entries.len()
                );

                for (key, score) in entries.iter().take(count_to_evict) {
                    if let Some(entry) = cache_index.remove(key) {
                        if let Err(e) = fs::remove_file(&entry.file_path) {
                            eprintln!(
                                "[CacheManager] Failed to delete file {:?}: {:?}",
                                entry.file_path, e
                            );
                        } else {
                            println!(
                                "[CacheManager] Evicted L1 file: {:?} (Score: {:.2})",
                                entry.file_path, score
                            );
                            get_metrics_registry().record_l1_eviction();
                        }
                    }
                }
            } else {
                // Fallback: If index is empty (e.g. restart), use directory listing (Mtime based)
                // ... existing fallback code ...
                let mut files = Vec::new();
                if let Ok(entries) = fs::read_dir(&cache_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "parquet") {
                            if let Ok(metadata) = fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    files.push((path, modified));
                                }
                            }
                        }
                    }
                }
                files.sort_by(|a, b| a.1.cmp(&b.1));
                let files_to_delete = (files.len() as f64 * 0.10).ceil() as usize;
                for (path, _) in files.iter().take(files_to_delete) {
                    let _ = fs::remove_file(path);
                    get_metrics_registry().record_l1_eviction();
                }
            }
        }
    }

    /// Debug/Inspection: Get status of all cache entries (L1 and L2)
    #[allow(dead_code)]
    pub fn get_cache_status() -> Vec<String> {
        let mut status = Vec::new();

        // L2 Status
        let cache = get_l2_cache();
        let mut total_count = 0;
        let mut items = Vec::new();

        for (i, shard) in cache.shards.iter().enumerate() {
            let shard_lock = shard.read().unwrap();
            total_count += shard_lock.len();
            for (key, entry) in shard_lock.iter() {
                items.push(format!(
                    "Shard: {}, Key: {}, Score: {:.2}, Size: {}B, Cost: {}ms, LastAccess: {}",
                    i,
                    key,
                    entry.get_score(),
                    entry.size,
                    entry.cost,
                    entry.last_access.load(Ordering::Relaxed)
                ));
            }
        }
        status.push(format!("--- L2 Cache (Memory) Count: {} ---", total_count));
        status.extend(items);

        // L1 Status
        let l1_cache = get_l1_cache_index().read().unwrap();
        status.push(format!("--- L1 Cache (Disk) Count: {} ---", l1_cache.len()));
        for (key, entry) in l1_cache.iter() {
            status.push(format!(
                "Key: {}, Score: {:.2}, Size: {}B, Cost: {}ms, Path: {:?}, LastAccess: {}",
                key,
                entry.get_score(),
                entry.size,
                entry.cost,
                entry.file_path,
                entry.last_access.load(Ordering::Relaxed)
            ));
        }

        status
    }

    /// Creates a synchronous Parquet writer for incremental caching.
    /// This is used by the "Sidecar" (Safety Airbag) to write batches to disk to prevent OOM.
    /// The file is stored in `cache/l1/{key}.parquet`.
    #[allow(dead_code)]
    pub fn create_incremental_writer(key: &str, schema: SchemaRef) -> Result<ArrowWriter<File>> {
        // Check disk space before creating new cache file
        Self::check_l1_disk_eviction();

        // Ensure cache directory exists
        let cache_dir = Path::new("cache").join("l1");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).map_err(|e| DataFusionError::IoError(e))?;
        }

        let file_path = Self::get_cache_file_path(key);
        println!("[CacheManager] Creating cache file: {:?}", file_path);

        let file = File::create(file_path).map_err(|e| DataFusionError::IoError(e))?;
        let props = WriterProperties::builder().build();
        let writer = ArrowWriter::try_new(file, schema, Some(props))
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        Ok(writer)
    }

    /// Ensures a Parquet cache exists for the given file.
    /// Returns the path to the Parquet file (either newly created or existing).
    pub async fn ensure_parquet_cache(
        file_path: &str,
        source_type: &str,
        sheet_name: Option<String>,
    ) -> Result<String> {
        let shadow_path = if let Some(sheet) = &sheet_name {
            format!("{}_{}.shadow.parquet", file_path, sheet)
        } else {
            format!("{}.shadow.parquet", file_path)
        };

        if Path::new(&shadow_path).exists() {
            println!("Cache hit: Using existing shadow file {}", shadow_path);
            return Ok(shadow_path);
        }

        // Singleflight: Prevent concurrent transcoding of the same file
        let flight_key = format!("shadow:{}", shadow_path);
        match Self::join_or_start_flight(flight_key.clone()) {
            FlightResult::IsFollower(mut rx) => {
                println!(
                    "[CacheManager] Waiting for shadow file transcoding (Follower): {}",
                    shadow_path
                );
                let _ = rx.changed().await;
                if *rx.borrow() == FlightStatus::Completed {
                    if Path::new(&shadow_path).exists() {
                        return Ok(shadow_path);
                    }
                }
                // If failed or missing after wait
                return Err(DataFusionError::Execution(
                    "Concurrent transcoding failed or file missing".to_string(),
                ));
            }
            FlightResult::IsLeader(flight_guard) => {
                println!(
                    "Cache miss: Transcoding {} to Parquet (Leader)...",
                    file_path
                );

                let transcoding_task = async {
                    let ctx = SessionContext::new();
                    let df = match source_type {
                        "csv" => ctx.read_csv(file_path, CsvReadOptions::default()).await?,
                        "excel" => {
                            let sheet = sheet_name.ok_or(DataFusionError::Execution(
                                "Sheet name required for Excel".to_string(),
                            ))?;
                            let ds = ExcelDataSource::new(
                                "temp".to_string(),
                                file_path.to_string(),
                                sheet,
                            );
                            let mem_table = ds.load_table()?;
                            ctx.read_table(mem_table)?
                        }
                        _ => return Ok(file_path.to_string()), // Should not happen given logic flow
                    };

                    // Write to Parquet
                    df.write_parquet(&shadow_path, DataFrameWriteOptions::default(), None)
                        .await?;
                    Ok(shadow_path.clone())
                };

                match transcoding_task.await {
                    Ok(path) => {
                        println!("Transcoding complete: {}", path);
                        flight_guard.mark_completed();
                        Ok(path)
                    }
                    Err(e) => {
                        eprintln!("Transcoding failed: {:?}", e);
                        flight_guard.mark_failed();
                        // Cleanup partial file
                        let _ = std::fs::remove_file(&shadow_path);
                        Err(e)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_simple_and() {
        let s1 = "a=1 AND b=2";
        let s2 = "b=2 AND a=1";
        assert_eq!(CacheManager::canonicalize_sql(s1), "a=1 AND b=2");
        assert_eq!(CacheManager::canonicalize_sql(s2), "a=1 AND b=2");
    }

    #[test]
    fn test_canonicalize_parentheses() {
        assert_eq!(CacheManager::canonicalize_sql("(a=1)"), "a=1");
        assert_eq!(CacheManager::canonicalize_sql("((a=1))"), "a=1");
        // Balanced but not wrapper
        assert_eq!(
            CacheManager::canonicalize_sql("(a=1) AND (b=2)"),
            "a=1 AND b=2"
        );
    }

    #[test]
    fn test_canonicalize_nested_or() {
        let s1 = "(a=1 OR b=2) AND c=3";
        let s2 = "c=3 AND (b=2 OR a=1)";
        // Expected: "a=1 OR b=2" is sorted part 1 (wrapped). "c=3" is part 2.
        // Sort: "(..." comes before "c..."
        assert_eq!(CacheManager::canonicalize_sql(s1), "(a=1 OR b=2) AND c=3");
        assert_eq!(CacheManager::canonicalize_sql(s2), "(a=1 OR b=2) AND c=3");
    }

    #[test]
    fn test_canonicalize_complex_precedence() {
        // A AND B OR C AND D
        // Split OR -> [A AND B], [C AND D]
        // Sorted -> A AND B OR C AND D
        let s = "a=1 AND b=2 OR c=3 AND d=4";
        let s_rev = "c=3 AND d=4 OR b=2 AND a=1";
        // s_rev split OR -> [c=3 AND d=4], [b=2 AND a=1 -> a=1 AND b=2]
        // Sort -> [a=1 AND b=2], [c=3 AND d=4]
        // Join -> a=1 AND b=2 OR c=3 AND d=4
        assert_eq!(
            CacheManager::canonicalize_sql(s),
            "a=1 AND b=2 OR c=3 AND d=4"
        );
        assert_eq!(
            CacheManager::canonicalize_sql(s_rev),
            "a=1 AND b=2 OR c=3 AND d=4"
        );
    }

    #[test]
    fn test_canonicalize_whitespace() {
        assert_eq!(CacheManager::canonicalize_sql("  a = 1  "), "a = 1");
    }

    #[test]
    fn test_canonicalize_case_insensitive_keywords() {
        assert_eq!(CacheManager::canonicalize_sql("a=1 and b=2"), "a=1 AND b=2");
        assert_eq!(CacheManager::canonicalize_sql("a=1 OR b=2"), "a=1 OR b=2");
    }

    #[test]
    fn test_is_balanced() {
        assert!(CacheManager::is_balanced_wrapper("(a)"));
        assert!(CacheManager::is_balanced_wrapper("((a))"));
        assert!(!CacheManager::is_balanced_wrapper("(a) AND (b)"));
        assert!(!CacheManager::is_balanced_wrapper("a AND (b)"));
    }

    #[test]
    // **[2026-02-26]** 变更原因：新增 L1 驱逐节流逻辑需要测试覆盖。
    // **[2026-02-26]** 变更目的：确保节流时间窗口内不重复触发检查。
    // **[2026-02-26]** 变更说明：使用测试钩子避免真实磁盘依赖。
    // **[2026-02-26]** 变更说明：覆盖节流生效与过期刷新两个分支。
    // **[2026-02-26]** 变更说明：不影响其它测试状态。
    fn test_l1_eviction_check_throttled() {
        CacheManager::set_test_l1_eviction_interval_ms(Some(10_000));
        CacheManager::set_test_disk_usage(Some((100, 0)));

        let now = CacheManager::now_ms();
        L1_EVICTION_LAST_CHECK_MS.store(now, Ordering::Relaxed);

        CacheManager::put_l1("k1".to_string(), PathBuf::from("cache/l1/a.parquet"), 1, 1);

        let last_check = L1_EVICTION_LAST_CHECK_MS.load(Ordering::Relaxed);
        assert_eq!(last_check, now);

        let expired = now.saturating_sub(20_000);
        L1_EVICTION_LAST_CHECK_MS.store(expired, Ordering::Relaxed);
        CacheManager::put_l1("k2".to_string(), PathBuf::from("cache/l1/b.parquet"), 1, 1);

        let refreshed = L1_EVICTION_LAST_CHECK_MS.load(Ordering::Relaxed);
        assert!(refreshed > expired);

        CacheManager::set_test_l1_eviction_interval_ms(None);
        CacheManager::set_test_disk_usage(None);
    }

    #[tokio::test]
    async fn test_eviction_reliability() {
        use datafusion::arrow::array::Int64Builder;
        use datafusion::arrow::datatypes::{DataType as ArrowDataType, Field, Schema};

        // 1. Setup
        CacheManager::clear_l2();
        // Set limit to 350 bytes (Each item is ~160 bytes)
        CacheManager::set_test_memory_limit(Some(350));

        // 2. Prepare Data (Mock RecordBatches)
        let schema = Arc::new(Schema::new(vec![Field::new(
            "a",
            ArrowDataType::Int64,
            false,
        )]));

        // Helper to create batch of approx specific size
        // 5 * 8 bytes = 40 bytes payload + overhead
        let make_batch = |size_bytes: usize| -> Vec<RecordBatch> {
            let num_rows = size_bytes / 8;
            let mut builder = Int64Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                builder.append_value(i as i64);
            }
            let array = builder.finish();
            let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(array)]).unwrap();
            vec![batch]
        };

        // 3. Insert Items
        // A: Cost 1000, Size 40 -> High Score -> Keep
        let batch_a = make_batch(40);
        CacheManager::put_l2("A".to_string(), batch_a, 1000);

        // B: Cost 1, Size 40 -> Low Score -> Evict victim
        let batch_b = make_batch(40);
        CacheManager::put_l2("B".to_string(), batch_b, 1);

        // C: Cost 500, Size 40 -> Medium Score -> Keep
        let batch_c = make_batch(40);
        CacheManager::put_l2("C".to_string(), batch_c, 500);

        // Total Size inserted > 100.

        // 4. Trigger Eviction
        CacheManager::run_eviction_cycle_for_test(350).await;

        // 5. Verify
        let res_b = CacheManager::get_l2("B");
        assert!(res_b.is_none(), "Item B should be evicted (Low Score)");

        let res_a = CacheManager::get_l2("A");
        assert!(res_a.is_some(), "Item A should be kept (High Score)");

        let res_c = CacheManager::get_l2("C");
        assert!(res_c.is_some(), "Item C should be kept (Medium Score)");
    }

    #[test]
    // **[2026-02-26]** 变更原因：内存上限缓存刷新逻辑新增可测试路径。
    // **[2026-02-26]** 变更目的：验证缓存命中与过期刷新行为。
    // **[2026-02-26]** 变更说明：使用固定总内存值避免环境波动。
    // **[2026-02-26]** 变更说明：覆盖间隔内不刷新与过期后刷新。
    // **[2026-02-26]** 变更说明：测试结束恢复钩子状态。
    fn test_memory_limit_cache_refresh_behavior() {
        CacheManager::set_test_memory_limit(None);
        CacheManager::set_test_total_memory_bytes(Some(1000));
        CacheManager::set_test_memory_limit_refresh_interval_ms(Some(1000));
        MEMORY_LIMIT_CACHE.store(0, Ordering::Relaxed);
        MEMORY_LIMIT_LAST_REFRESH_MS.store(0, Ordering::Relaxed);

        let first = CacheManager::get_memory_limit_bytes_with_now(1000);
        assert_eq!(first, 700);

        CacheManager::set_test_total_memory_bytes(Some(2000));
        let cached = CacheManager::get_memory_limit_bytes_with_now(1500);
        assert_eq!(cached, 700);

        let refreshed = CacheManager::get_memory_limit_bytes_with_now(3000);
        assert_eq!(refreshed, 1400);

        CacheManager::set_test_memory_limit_refresh_interval_ms(None);
        CacheManager::set_test_total_memory_bytes(None);
    }

    #[test]
    // **[2026-02-26]** 变更原因：L2 合并阈值逻辑新增条件分支。
    // **[2026-02-26]** 变更目的：验证阈值关闭与开启两条路径。
    // **[2026-02-26]** 变更说明：使用小批次数据控制内存大小。
    // **[2026-02-26]** 变更说明：分别断言批次数量变化。
    // **[2026-02-26]** 变更说明：测试结束恢复钩子状态。
    fn test_l2_compaction_threshold() {
        use datafusion::arrow::array::Int32Builder;
        use datafusion::arrow::datatypes::{DataType as ArrowDataType, Field, Schema};

        CacheManager::clear_l2();
        CacheManager::set_test_memory_limit(Some(10_000_000));

        let schema = Arc::new(Schema::new(vec![Field::new(
            "a",
            ArrowDataType::Int32,
            false,
        )]));

        let make_batch = |rows: usize| -> RecordBatch {
            let mut builder = Int32Builder::with_capacity(rows);
            for i in 0..rows {
                builder.append_value(i as i32);
            }
            let array = builder.finish();
            RecordBatch::try_new(schema.clone(), vec![Arc::new(array)]).unwrap()
        };

        CacheManager::set_test_l2_compaction_max_bytes(Some(1));
        CacheManager::put_l2(
            "compact_off".to_string(),
            vec![make_batch(1), make_batch(1)],
            1,
        );

        let cache = get_l2_cache();
        let shard = cache.get_shard("compact_off").read().unwrap();
        let entry = shard.get("compact_off").unwrap();
        assert_eq!(entry.data.len(), 2);
        drop(shard);

        CacheManager::set_test_l2_compaction_max_bytes(Some(1_000_000));
        CacheManager::put_l2(
            "compact_on".to_string(),
            vec![make_batch(1), make_batch(1)],
            1,
        );

        let shard = cache.get_shard("compact_on").read().unwrap();
        let entry = shard.get("compact_on").unwrap();
        assert_eq!(entry.data.len(), 1);

        CacheManager::set_test_l2_compaction_max_bytes(None);
        CacheManager::set_test_memory_limit(None);
    }
}

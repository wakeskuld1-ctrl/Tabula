use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const CHANGE_NOTES: &[&str] = &[
    "变更备注 2026-03-12: Sidecar日志写入迁移到.runtime/logs，原因是统一运行时文件目录并便于排障",
    "变更备注 2026-03-12: Sidecar关键状态同步写入应用日志，原因是减少查问题时跨文件定位成本",
];

// Global log buffer: stores last 1000 lines
pub static GLOBAL_LOGS: Lazy<Arc<RwLock<VecDeque<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(VecDeque::new())));

#[derive(Clone, Serialize, Debug)]
pub struct SidecarLogEntry {
    pub timestamp: String,
    pub table_name: String,
    pub row_count: u64,
    pub speed: f64,     // rows/sec
    pub status: String, // "Running", "Completed", "Failed"
    pub message: String,
}

// Sidecar Status Map: Stores the LATEST status of each table task (In-Memory Only)
pub static SIDECAR_STATUS: Lazy<Arc<RwLock<HashMap<String, SidecarLogEntry>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Sidecar Event Log: Stores significant events (Start/End) and persistence
pub static SIDECAR_LOGS: Lazy<Arc<RwLock<VecDeque<SidecarLogEntry>>>> = Lazy::new(|| {
    let mut deque = VecDeque::new();
    let sidecar_log_path = sidecar_log_file_path();

    // Attempt to load from sidecar.log on startup
    if let Ok(file) = std::fs::File::open(&sidecar_log_path) {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(file);

        // Read lines (keep last 1000)
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start_idx = if lines.len() > 1000 {
            lines.len() - 1000
        } else {
            0
        };

        for line in lines.iter().skip(start_idx) {
            // Parse: timestamp | table_name | row_count rows | speed r/s | status | message
            let parts: Vec<&str> = line.split(" | ").collect();
            if parts.len() >= 6 {
                let timestamp = parts[0].to_string();
                let table_name = parts[1].to_string();

                // Parse "100 rows" -> 100
                let row_count = parts[2]
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                // Parse "50 r/s" -> 50.0
                let speed = parts[3]
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let status = parts[4].to_string();
                let message = parts[5].to_string();

                deque.push_back(SidecarLogEntry {
                    timestamp,
                    table_name,
                    row_count,
                    speed,
                    status,
                    message,
                });
            }
        }
    }

    Arc::new(RwLock::new(deque))
});

pub fn init_logger(log_dir: &str) {
    let _ = CHANGE_NOTES;
    // Ensure log directory exists
    if !std::path::Path::new(log_dir).exists() {
        let _ = std::fs::create_dir_all(log_dir);
    }
    // Future expansion: Initialize file-based logger (tracing/log4rs) here
    // For now, we rely on println! and in-memory buffer, but we prepare the directory.
    log(&format!("Logger initialized in: {}", log_dir));
}

/// 记录应用日志
///
/// **实现方案**:
/// 1. 获取当前时间戳。
/// 2. 打印到标准输出 (`stdout`)。
/// 3. 将日志存入内存缓冲区 `GLOBAL_LOGS`（保留最近 1000 条）。
///
/// **调用链路**:
/// - 通过 `app_log!` 宏在应用各处调用。
pub fn log(msg: &str) {
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let formatted_msg = format!("[{}] {}", timestamp, msg);

    // Print to stdout
    println!("{}", msg);

    // Add to buffer
    let logs = GLOBAL_LOGS.clone();
    if let Ok(mut guard) = logs.write() {
        guard.push_back(formatted_msg);
        if guard.len() > 1000 {
            guard.pop_front();
        }
    };
}

fn sidecar_log_file_path() -> PathBuf {
    let log_dir = crate::config::AppConfig::global().runtime_dir.join("logs");
    if !log_dir.exists() {
        let _ = std::fs::create_dir_all(&log_dir);
    }
    log_dir.join("sidecar.log")
}

/// 记录 Sidecar (后台缓存任务) 日志
///
/// **实现方案**:
/// 1. **内存状态更新**: 无论状态如何，始终更新 `SIDECAR_STATUS` 内存映射，用于前端实时 Dashboard 显示（Latest Status）。
/// 2. **持久化**: 仅当状态不是 "Running" 时（即 Start, Completed, Failed 等重要事件），写入磁盘文件 `sidecar.log` 并添加到历史缓冲区 `SIDECAR_LOGS`。
/// 3. **日志轮转**: 如果日志文件超过 10MB，自动重命名为 `sidecar.log.old`。
///
/// **关键问题点**:
/// - 性能：避免高频写入 "Running" 状态导致磁盘 IO 瓶颈。
/// - 可观测性：提供实时状态和历史记录两种视图。
pub fn log_sidecar(table_name: &str, row_count: u64, speed: f64, status: &str, message: &str) {
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let entry = SidecarLogEntry {
        timestamp: timestamp.clone(),
        table_name: table_name.to_string(),
        row_count,
        speed,
        status: status.to_string(),
        message: message.to_string(),
    };

    // 1. Update In-Memory Status (Always)
    // This allows the UI to see real-time "Running" updates without disk I/O
    let status_map = SIDECAR_STATUS.clone();
    if let Ok(mut guard) = status_map.write() {
        guard.insert(table_name.to_string(), entry.clone());
    }

    // 2. Write to Disk & History Log ONLY for significant events
    // Avoid writing "Running" updates every second to save disk/resources
    if status != "Running" {
        use std::io::Write;
        let sidecar_log_path = sidecar_log_file_path();
        let log_line = format!(
            "{} | {} | {} rows | {:.0} r/s | {} | {}\n",
            timestamp, table_name, row_count, speed, status, message
        );

        // Log Rotation Logic: 10MB limit
        const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10MB
        if let Ok(metadata) = std::fs::metadata(&sidecar_log_path) {
            if metadata.len() > MAX_LOG_SIZE {
                let rotated_path = sidecar_log_path.with_extension("log.old");
                let _ = std::fs::rename(&sidecar_log_path, rotated_path);
            }
        }

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&sidecar_log_path)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
        log(&format!(
            "Sidecar status: table={}, status={}, rows={}, speed={:.0}, message={}",
            table_name, status, row_count, speed, message
        ));

        // Add to history buffer
        let logs = SIDECAR_LOGS.clone();
        if let Ok(mut guard) = logs.write() {
            guard.push_back(entry);
            if guard.len() > 1000 {
                guard.pop_front();
            }
        };
    }
}

pub fn get_logs() -> Vec<String> {
    if let Ok(guard) = GLOBAL_LOGS.read() {
        guard.iter().cloned().collect()
    } else {
        vec!["Failed to acquire log lock".to_string()]
    }
}

pub fn get_sidecar_logs() -> Vec<SidecarLogEntry> {
    // Return the LATEST status of all known tasks (Active or Recently Completed)
    // This provides a "Dashboard" view (one row per table) instead of a Log view
    if let Ok(guard) = SIDECAR_STATUS.read() {
        let mut logs: Vec<SidecarLogEntry> = guard.values().cloned().collect();
        // Sort by timestamp (newest first)
        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        logs
    } else {
        Vec::new()
    }
}

#[macro_export]
macro_rules! app_log {
    ($($arg:tt)*) => {
        $crate::logger::log(&format!($($arg)*))
    }
}

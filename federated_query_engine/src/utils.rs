use std::fs;
use std::path::PathBuf;

/// 临时文件 RAII 守卫
///
/// **实现方案**:
/// 封装临时文件路径。
/// - 在 `new` 时，如果路径已存在则删除，确保干净的开始。
/// - 在 `drop` 时，如果未调用 `commit()`，则尝试删除文件（回滚）。
/// - `commit()` 标记操作成功，放弃删除所有权。
///
/// **关键问题点**:
/// - 原子性保证：配合 `std::fs::rename` 使用，实现 "Write-to-Temp-then-Rename" 模式。
pub struct TmpFileGuard {
    path: PathBuf,
    committed: bool,
}

impl TmpFileGuard {
    /// Create a new guard for the given path.
    /// If a file already exists at the path, it is deleted immediately to ensure a clean slate.
    pub fn new(path: PathBuf) -> Self {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        Self {
            path,
            committed: false,
        }
    }

    /// Mark the file operation as successful.
    /// The file will NOT be deleted when the guard is dropped.
    pub fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for TmpFileGuard {
    fn drop(&mut self) {
        if !self.committed && self.path.exists() {
            // Attempt to delete the file, ignoring errors (best effort)
            let _ = fs::remove_file(&self.path);
        }
    }
}

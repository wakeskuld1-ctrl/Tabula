use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::Result;

/// 一个简单的键值对数据库
pub struct SimpleDb {
    map: HashMap<String, String>,
    file_path: Option<String>,
}

impl SimpleDb {
    /// 创建一个新的内存数据库
    #[allow(dead_code)]
    pub fn new() -> Self {
        SimpleDb {
            map: HashMap::new(),
            file_path: None,
        }
    }

    /// 从文件加载数据库
    pub fn open(path: &str) -> Result<Self> {
        let map = if Path::new(path).exists() {
            let content = fs::read_to_string(path)?;
            if content.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&content)?
            }
        } else {
            HashMap::new()
        };

        Ok(SimpleDb {
            map,
            file_path: Some(path.to_string()),
        })
    }

    /// 插入数据
    pub fn insert(&mut self, key: String, value: String) {
        self.map.insert(key, value);
        self.save().ok(); // 尝试自动保存，忽略错误
    }

    /// 获取数据
    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    /// 删除数据
    pub fn remove(&mut self, key: &str) {
        self.map.remove(key);
        self.save().ok();
    }

    /// 持久化到文件
    fn save(&self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let content = serde_json::to_string_pretty(&self.map)?;
            fs::write(path, content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut db = SimpleDb::new();
        db.insert("key1".to_string(), "value1".to_string());
        assert_eq!(db.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_remove() {
        let mut db = SimpleDb::new();
        db.insert("key1".to_string(), "value1".to_string());
        db.remove("key1");
        assert_eq!(db.get("key1"), None);
    }

    #[test]
    fn test_persistence() {
        let path = "test_db.json";
        // 清理旧文件
        if Path::new(path).exists() {
            fs::remove_file(path).unwrap();
        }

        {
            let mut db = SimpleDb::open(path).unwrap();
            db.insert("persist_key".to_string(), "persist_value".to_string());
        } // db 在这里 drop，数据应该已保存

        {
            let db = SimpleDb::open(path).unwrap();
            assert_eq!(db.get("persist_key"), Some(&"persist_value".to_string()));
        }

        // 清理测试文件
        fs::remove_file(path).unwrap();
    }
}

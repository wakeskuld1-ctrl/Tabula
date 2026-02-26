mod llm;
mod db;
mod memory_demo;

use llm::{LlmClient, Message};
use db::SimpleDb;
use metadata_store::MetadataStore;
use std::env;
use dotenv::dotenv;

fn main() {
    // 加载 .env 文件
    dotenv().ok();

    println!("=== Rust 环境与功能演示 ===");

    // 0. 内存管理演示
    memory_demo::run_demo();

    // 1. 元数据存储演示 (SQLite)
    println!("\n[1] 演示元数据存储 (SQLite):");
    match MetadataStore::new("metadata.db") {
        Ok(store) => {
            println!("  已连接到 SQLite 数据库: metadata.db");
            // 插入示例数据
            let res = store.add_table("datafusion", "public", "users", "s3://bucket/users.parquet", "parquet", None);
            match res {
                Ok(_) => println!("  成功注册表: users"),
                Err(e) => println!("  注册表失败: {}", e),
            }
            
            // 查询数据
            match store.list_tables() {
                Ok(tables) => {
                    println!("  当前已注册的表:");
                    for t in tables {
                        println!("    - {} ({}) -> {}", t.table_name, t.source_type, t.file_path);
                    }
                },
                Err(e) => println!("  查询表失败: {}", e),
            }
        },
        Err(e) => println!("  连接 SQLite 失败: {}", e),
    }

    // 2. 简单的 Key-Value 数据库演示 (JSON)
    println!("\n[2] 演示简单的 Key-Value 数据库 (JSON):");
    let mut db = SimpleDb::open("my_data.json").expect("无法打开数据库");
    
    println!("  插入数据: user_id = 1001");
    db.insert("user_id".to_string(), "1001".to_string());
    
    println!("  插入数据: username = rust_fan");
    db.insert("username".to_string(), "rust_fan".to_string());

    if let Some(val) = db.get("username") {
        println!("  读取数据: username = {}", val);
    }
    
    // 演示删除
    println!("  删除数据: user_id");
    db.remove("user_id");
    
    println!("  (数据已自动持久化到 my_data.json)");


    // 3. LLM 客户端演示 (Mock 模式)
    println!("\n[3] 演示 LLM 客户端 (模拟模式):");
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string());
    let base_url = env::var("OPENAI_BASE_URL").ok();

    println!("  正在初始化 LLM 客户端...");
    let client = LlmClient::new(api_key, base_url);

    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "Rust 适合做数据库吗？".to_string(),
        }
    ];

    println!("  发送请求: {:?}", messages[0].content);
    match client.chat_completion(messages) {
        Ok(response) => println!("  大模型响应: {}", response),
        Err(e) => eprintln!("  调用出错: {}", e),
    }
}

// 简单的单元测试
#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_logic() {
        assert_eq!(1 + 1, 2);
    }
}

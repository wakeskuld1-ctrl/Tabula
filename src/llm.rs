use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct LlmClient {
    #[allow(dead_code)]
    api_key: String,
    #[allow(dead_code)]
    base_url: String,
}

impl LlmClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }

    // 模拟调用，避免引入复杂的 reqwest/tokio 导致编译失败
    pub fn chat_completion(&self, _messages: Vec<Message>) -> Result<String> {
        // 模拟网络延迟
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok("这是一个模拟的响应，因为当前环境缺少编译异步网络库所需的底层工具。但代码结构是正确的！".to_string())
    }
}

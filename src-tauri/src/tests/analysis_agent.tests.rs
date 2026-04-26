/// Analysis Agent 集成测试
#[cfg(test)]
mod tests {
    use reqwest::Client;
    use serde_json::{json, Value};

    const API_KEY_ENV: &str = "DASHSCOPE_API_KEY";
    const API_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
    const MODEL: &str = "qwen-plus";

    // ─────────────────────────────────────────────────────────────────────────
    // 基础工具函数
    // ─────────────────────────────────────────────────────────────────────────

    /// 向 LLM 发送消息列表，返回模型的文本回复
    ///
    /// # 参数
    /// - `messages` : 完整的消息列表，格式：`[{"role": "...", "content": "..."}]`
    ///
    /// # 返回
    /// 模型回复的文本内容（choices[0].message.content）
    async fn call_llm(messages: Value) -> String {
        let api_key = std::env::var(API_KEY_ENV).unwrap_or_default();
        assert!(
            !api_key.trim().is_empty(),
            "未设置 {}，无法运行真实 LLM 测试",
            API_KEY_ENV
        );
        let body = json!({
            "model": MODEL,
            "messages": messages
        });

        let client = Client::new();
        let response = client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .expect("请求发送失败");

        let status = response.status();
        let text = response.text().await.expect("读取响应失败");

        assert!(
            status.is_success(),
            "HTTP 请求失败，状态码: {}，响应: {}",
            status,
            text
        );

        let json: Value = serde_json::from_str(&text).expect("响应不是合法 JSON");
        json["choices"][0]["message"]["content"]
            .as_str()
            .expect("响应中没有 choices[0].message.content")
            .to_string()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 测试
    // ─────────────────────────────────────────────────────────────────────────

    /// 验证 API 可以正常调用，并能返回非空文本
    #[tokio::test]
    #[ignore = "需要真实 LLM API"]
    async fn test_basic_llm_call() {
        let messages = json!([
            { "role": "system", "content": "你是一个数据分析助手" },
            { "role": "user",   "content": "我有些个人财务数据，你能帮我分析下吗？" }
        ]);

        let reply = call_llm(messages).await;

        println!("模型回复：{}", reply);
        assert!(!reply.is_empty(), "模型回复不应为空");
    }
}

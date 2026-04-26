pub mod quick_booking_tests;
#[path = "analysis_agent.tests.rs"]
pub mod analysis_agent_tests;
pub mod memory_tests;

// 共用的测试工具函数
use crate::database::Database;
use crate::models::*;
use crate::DatabaseState;
use std::sync::Arc;
// 测试工具模块

/// 创建测试用的内存数据库
pub async fn create_test_database() -> Result<Database, Box<dyn std::error::Error>> {
    let db = Database::new("sqlite::memory:").await?;
    
    // 插入一些测试分类数据
    let test_categories = vec![
        NewCategory {
            name: "餐饮".to_string(),
            icon: Some("🍽️".to_string()),
            color: Some("#FF6B6B".to_string()),
            r#type: "expense".to_string(),
            parent_id: None,
        },
        NewCategory {
            name: "交通".to_string(),
            icon: Some("🚗".to_string()),
            color: Some("#4ECDC4".to_string()),
            r#type: "expense".to_string(),
            parent_id: None,
        },
        NewCategory {
            name: "其他支出".to_string(),
            icon: Some("💰".to_string()),
            color: Some("#A0A0A0".to_string()),
            r#type: "expense".to_string(),
            parent_id: None,
        },
        NewCategory {
            name: "工资".to_string(),
            icon: Some("💼".to_string()),
            color: Some("#52C41A".to_string()),
            r#type: "income".to_string(),
            parent_id: None,
        },
        NewCategory {
            name: "其他收入".to_string(),
            icon: Some("💰".to_string()),
            color: Some("#52C41A".to_string()),
            r#type: "income".to_string(),
            parent_id: None,
        },
    ];
    
    for category in test_categories {
        db.create_category(&category).await?;
    }
    
    // 插入测试LLM配置
    let llm_config = NewLLMConfig {
        config_name: "Test Config".to_string(),
        provider: "Test Platform".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: "test-api-key".to_string(),
        model: "gpt-4o".to_string(),
        temperature: Some(0.3),
        max_tokens: Some(2048),
        enable_thinking: Some(false),
    };
    
    db.save_llm_config(&llm_config).await?;
    
    Ok(db)
}

/// 创建模拟的DatabaseState
pub async fn create_test_database_state() -> Result<DatabaseState, Box<dyn std::error::Error>> {
    let db = create_test_database().await?;
    let memory = Arc::new(crate::memory::MemoryFacade::new(db.pool.clone()));
    let token_recorder = Arc::new(crate::telemetry::TokenUsageRecorder::new(db.pool.clone()));
    Ok(DatabaseState { db, memory, token_recorder })
} 
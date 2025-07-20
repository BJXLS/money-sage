pub mod quick_booking_tests;

// 共用的测试工具函数
use crate::database::Database;
use crate::models::*;
use crate::DatabaseState;
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
        platform: "Test Platform".to_string(),
        app_key: "test-api-key".to_string(),
    };
    
    db.save_llm_config(&llm_config).await?;
    
    Ok(db)
}

/// 创建模拟的DatabaseState
pub async fn create_test_database_state() -> Result<DatabaseState, Box<dyn std::error::Error>> {
    let db = create_test_database().await?;
    Ok(DatabaseState { db })
} 
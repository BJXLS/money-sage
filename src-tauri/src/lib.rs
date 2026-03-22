// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Emitter, Manager, State};
use chrono::NaiveDate;
use anyhow::Result;
use std::collections::HashMap;

mod models;
mod database;
mod utils;
mod ai;
mod mcp;

#[cfg(test)]
mod tests;

use database::Database;
use models::*;

pub struct DatabaseState {
    pub db: Database,
}

pub struct McpState {
    pub manager: mcp::McpManager,
}

#[tauri::command]
async fn get_categories(state: State<'_, DatabaseState>) -> Result<Vec<Category>, String> {
    state.db.get_categories().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_categories_by_type(state: State<'_, DatabaseState>, category_type: String) -> Result<Vec<Category>, String> {
    state.db.get_categories_by_type(&category_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_category(state: State<'_, DatabaseState>, category: NewCategory) -> Result<i64, String> {
    state.db.create_category(&category).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_category(state: State<'_, DatabaseState>, id: i64, category: UpdateCategory) -> Result<(), String> {
    state.db.update_category(id, &category).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_category(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.delete_category(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_parent_categories(state: State<'_, DatabaseState>, category_type: String) -> Result<Vec<Category>, String> {
    state.db.get_parent_categories(&category_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_sub_categories(state: State<'_, DatabaseState>, parent_id: i64) -> Result<Vec<Category>, String> {
    state.db.get_sub_categories(parent_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_transactions(state: State<'_, DatabaseState>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransactionWithCategory>, String> {
    state.db.get_transactions(limit, offset).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_transactions_by_date_range(state: State<'_, DatabaseState>, start_date: String, end_date: String) -> Result<Vec<TransactionWithCategory>, String> {
    let start_date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let end_date = NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    state.db.get_transactions_by_date_range(&start_date, &end_date).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_transaction(state: State<'_, DatabaseState>, transaction: NewTransaction) -> Result<i64, String> {
    state.db.create_transaction(&transaction).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_transaction(state: State<'_, DatabaseState>, id: i64, transaction: UpdateTransaction) -> Result<(), String> {
    state.db.update_transaction(id, &transaction).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_transaction(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.delete_transaction(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_monthly_stats(state: State<'_, DatabaseState>, months: i32) -> Result<Vec<MonthlyStats>, String> {
    state.db.get_monthly_stats(months).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_category_stats(state: State<'_, DatabaseState>, start_date: String, end_date: String, transaction_type: String) -> Result<Vec<CategoryStats>, String> {
    let start_date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let end_date = NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    state.db.get_category_stats(&start_date, &end_date, &transaction_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_budgets(state: State<'_, DatabaseState>) -> Result<Vec<BudgetProgress>, String> {
    state.db.get_budgets().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_budget(state: State<'_, DatabaseState>, budget: NewBudget) -> Result<i64, String> {
    state.db.create_budget(&budget).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_budget(state: State<'_, DatabaseState>, id: i64, budget: UpdateBudget) -> Result<(), String> {
    state.db.update_budget(id, &budget).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_budget(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.delete_budget(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_transactions(
    state: State<'_, DatabaseState>,
    file_path: String,
) -> Result<usize, String> {
    let db = &state.db;
    
    let mut reader = csv::Reader::from_path(&file_path)
        .map_err(|e| format!("无法读取CSV文件: {}", e))?;
    
    let mut count = 0;
    
    for result in reader.deserialize::<HashMap<String, String>>() {
        let record: HashMap<String, String> = result
            .map_err(|e| format!("CSV格式错误: {}", e))?;
        
        // 解析日期
        let date_str = record.get("date")
            .or_else(|| record.get("日期"))
            .map_or("", |v| v);
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y/%m/%d"))
            .map_err(|_| format!("无效的日期格式: {}", date_str))?;
        
        // 解析交易类型
        let transaction_type = record.get("type")
            .or_else(|| record.get("类型"))
            .map_or("expense", |v| v);
        
        // 解析金额
        let amount_str = record.get("amount")
            .or_else(|| record.get("金额"))
            .map_or("0", |v| v);
        let amount: f64 = amount_str.parse()
            .map_err(|_| format!("无效的金额格式: {}", amount_str))?;
        
        // 解析分类ID
        let category_id_str = record.get("category_id")
            .or_else(|| record.get("分类ID"))
            .map_or("1", |v| v);
        let category_id: i64 = category_id_str.parse()
            .map_err(|_| format!("无效的分类ID格式: {}", category_id_str))?;
        
        // 获取描述和备注
        let description = record.get("description")
            .or_else(|| record.get("描述"))
            .map(|s| s.clone());
        let note = record.get("note")
            .or_else(|| record.get("备注"))
            .map(|s| s.clone());
        
        // 创建交易记录
        let new_transaction = NewTransaction {
            date,
            r#type: transaction_type.to_string(),
            amount,
            category_id,
            budget_id: None, // CSV导入时暂不关联预算
            description,
            note,
        };
        
        db.create_transaction(&new_transaction).await
            .map_err(|e| format!("创建交易记录失败: {}", e))?;
        
        count += 1;
    }
    
    Ok(count)
}

#[tauri::command]
async fn export_csv_transactions(state: State<'_, DatabaseState>, file_path: String, start_date: String, end_date: String) -> Result<(), String> {
    use std::fs::File;
    use csv::Writer;
    
    let start_date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let end_date = NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    
    let transactions = state.db.get_transactions_by_date_range(&start_date, &end_date).await.map_err(|e| e.to_string())?;
    
    let file = File::create(&file_path).map_err(|e| format!("无法创建文件: {}", e))?;
    let mut writer = Writer::from_writer(file);
    
    // 写入表头
    writer.write_record(&["日期", "类型", "金额", "分类", "描述", "备注"]).map_err(|e| e.to_string())?;
    
    for transaction in transactions {
        writer.write_record(&[
            transaction.date.to_string(),
            transaction.r#type,
            transaction.amount.to_string(),
            transaction.category_name,
            transaction.description.unwrap_or_default(),
            transaction.note.unwrap_or_default(),
        ]).map_err(|e| e.to_string())?;
    }
    
    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

// ── 大模型配置相关命令（泛化重设计）────────────────────────────────────

/// 获取所有已保存的 LLM 配置
#[tauri::command]
async fn get_llm_configs(state: State<'_, DatabaseState>) -> Result<Vec<LLMConfig>, String> {
    state.db.get_llm_configs().await.map_err(|e| e.to_string())
}

/// 获取当前活跃的 LLM 配置（快速记账等功能使用）
#[tauri::command]
async fn get_active_llm_config(state: State<'_, DatabaseState>) -> Result<Option<LLMConfig>, String> {
    state.db.get_active_llm_config().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_llm_config(state: State<'_, DatabaseState>, config: NewLLMConfig) -> Result<i64, String> {
    state.db.save_llm_config(&config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_llm_config(state: State<'_, DatabaseState>, id: i64, config: UpdateLLMConfig) -> Result<(), String> {
    state.db.update_llm_config(id, &config).await.map_err(|e| e.to_string())
}

/// 将指定配置设为活跃，同时停用其他所有配置
#[tauri::command]
async fn set_active_llm_config(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.set_active_llm_config(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_llm_config(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.delete_llm_config(id).await.map_err(|e| e.to_string())
}

/// 测试连接（不需要预先保存，直接发一条最小请求验证可达性）
#[tauri::command]
async fn test_llm_connection(config: TestConnectionRequest) -> Result<String, String> {
    use crate::utils::http_client::{AIHttpClient, ClientConfig, AIProvider, AIRequest, AIMessage};

    let client_config = ClientConfig {
        provider: AIProvider::Custom("test".to_string()),
        base_url: config.base_url.clone(),
        api_key: config.api_key.clone(),
        timeout_secs: 15,
        max_retries: 1,
        headers: std::collections::HashMap::new(),
    };

    let http_client = AIHttpClient::new(client_config)
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    let request = AIRequest {
        model: config.model.clone(),
        messages: vec![AIMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }],
        temperature: 0.1,
        max_tokens: 5,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stream: None,
        enable_thinking: false,
    };

    match http_client.chat_completion(request).await {
        Ok(resp) => {
            let model_used = resp.model;
            Ok(format!("连接成功！模型: {}", model_used))
        }
        Err(e) => Err(format!("连接失败: {}", e)),
    }
}

// 快速记账文本处理命令
#[tauri::command]
async fn process_quick_booking_text(state: State<'_, DatabaseState>, text: String) -> Result<QuickBookingResult, String> {
    use crate::ai::agent::{QuickNoteAgent, Agent};
    
    // 验证输入
    if text.trim().is_empty() {
        return Ok(QuickBookingResult {
            success: false,
            message: "输入文本不能为空".to_string(),
            parsed_transactions: vec![],
            failed_lines: vec![],
        });
    }
    
    // 1. 获取当前活跃的 LLM 配置
    let llm_config = match state.db.get_active_llm_config().await {
        Ok(Some(config)) => config,
        Ok(None) => {
            return Ok(QuickBookingResult {
                success: false,
                message: "请先在设置中配置大模型接口".to_string(),
                parsed_transactions: vec![],
                failed_lines: vec![],
            });
        }
        Err(e) => {
            return Ok(QuickBookingResult {
                success: false,
                message: format!("获取大模型配置失败: {}", e),
                parsed_transactions: vec![],
                failed_lines: vec![],
            });
        }
    };

    if llm_config.base_url.is_empty() || llm_config.model.is_empty() {
        return Ok(QuickBookingResult {
            success: false,
            message: "大模型配置不完整，请检查 Base URL 和模型名称".to_string(),
            parsed_transactions: vec![],
            failed_lines: vec![],
        });
    }

    // 2. 创建HTTP客户端和快速记账代理（使用配置中的 base_url 和 model）
    use crate::utils::http_client::{AIHttpClient, ClientConfig, AIProvider};

    let config = ClientConfig {
        provider: AIProvider::Custom(llm_config.provider.clone()),
        base_url: llm_config.base_url.clone(),
        api_key: llm_config.api_key.clone(),
        timeout_secs: 60,
        max_retries: 3,
        headers: std::collections::HashMap::new(),
    };
    
    let http_client = AIHttpClient::new(config)
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    let quick_note_agent = QuickNoteAgent::new_with_config(
        llm_config.model.clone(),
        llm_config.temperature as f32,
        llm_config.max_tokens as u32,
        llm_config.enable_thinking,
    );
    
    // 3. 获取所有分类数据用于构建动态提示词
    let all_categories = match state.db.get_categories().await {
        Ok(categories) => categories,
        Err(e) => {
            return Ok(QuickBookingResult {
                success: false,
                message: format!("获取分类数据失败: {}", e),
                parsed_transactions: vec![],
                failed_lines: vec![],
            });
        }
    };
    
    // 4. 使用AI模型解析文本（使用动态提示词）
    println!("🔥 [QuickBooking] 开始快速记账处理");
    println!("👤 [QuickBooking] 用户输入文本: {}", text);
    println!("🗂️ [QuickBooking] 当前可用分类数量: {}", all_categories.len());
    
    let parse_result = match quick_note_agent.parse_quick_note_with_categories(&text, &all_categories, &http_client).await {
        Ok(result) => {
            println!("✅ [QuickBooking] AI解析成功，共解析出{}条记录", result.transactions.len());
            result
        },
        Err(e) => {
            println!("❌ [QuickBooking] AI解析失败: {}", e);
            return Ok(QuickBookingResult {
                success: false,
                message: format!("AI解析失败: {}", e),
                parsed_transactions: vec![],
                failed_lines: vec![FailedLine {
                    line_number: 1,
                    original_text: text,
                    error_reason: e.to_string(),
                }],
            });
        }
    };
    
    // 4. 解析AI结果并准备展示给用户
    let mut parsed_transactions = Vec::new();
    let mut failed_lines = Vec::new();
    
    for (index, quick_transaction) in parse_result.transactions.iter().enumerate() {
        println!("🔄 [QuickBooking] 解析第{}条交易记录:", index + 1);
        println!("   原始分类: {}", quick_transaction.category);
        
        // 查找分类ID（但不强制要求找到）
        let category_type = if quick_transaction.transaction_type == "income" { "income" } else { "expense" };
        let categories = match state.db.get_categories_by_type(category_type).await {
            Ok(cats) => cats,
            Err(e) => {
                println!("❌ [QuickBooking] 获取{}类型分类失败: {}", category_type, e);
                failed_lines.push(FailedLine {
                    line_number: index + 1,
                    original_text: format!("{}: {} {}", 
                        quick_transaction.date, 
                        quick_transaction.amount, 
                        quick_transaction.remark),
                    error_reason: format!("获取分类失败: {}", e),
                });
                continue;
            }
        };
        
        println!("   可用的{}分类: {:?}", category_type, categories.iter().map(|c| &c.name).collect::<Vec<_>>());
        
        // 解析分类：处理"父分类-子分类"格式，但允许没有匹配到
        let (parent_name, child_name) = if quick_transaction.category.contains('-') {
            let parts: Vec<&str> = quick_transaction.category.split('-').collect();
            if parts.len() >= 2 {
                (parts[0].trim(), Some(parts[1].trim()))
            } else {
                (quick_transaction.category.as_str(), None)
            }
        } else {
            (quick_transaction.category.as_str(), None)
        };
        
        println!("   解析后 - 父分类: {}, 子分类: {:?}", parent_name, child_name);
        
        // 查找匹配的分类（允许没有找到）
        let category = if let Some(child) = child_name {
            // 优先查找子分类
            categories.iter()
                .find(|cat| cat.name == child)
                .or_else(|| {
                    println!("   未找到子分类'{}'，尝试查找父分类'{}'", child, parent_name);
                    categories.iter().find(|cat| cat.name == parent_name)
                })
        } else {
            // 直接查找分类名称
            categories.iter().find(|cat| cat.name == parent_name)
        }.or_else(|| {
            // 如果都找不到，尝试找到默认分类
            println!("   未找到匹配分类，查找默认分类");
            if category_type == "income" {
                categories.iter().find(|cat| cat.name == "其他收入")
            } else {
                categories.iter().find(|cat| cat.name == "其他支出")
            }
        });
        
        let category_id = if let Some(cat) = category {
            println!("✅ [QuickBooking] 找到匹配分类: {} (ID: {})", cat.name, cat.id);
            Some(cat.id)
        } else {
            println!("⚠️ [QuickBooking] 未找到匹配的分类: {}，将在前端让用户选择", quick_transaction.category);
            None
        };
        
        // 验证日期格式
        println!("📅 [QuickBooking] 验证日期: {}", quick_transaction.date);
        if let Err(e) = chrono::NaiveDate::parse_from_str(&quick_transaction.date, "%Y-%m-%d") {
            println!("❌ [QuickBooking] 日期格式无效: {}", e);
            failed_lines.push(FailedLine {
                line_number: index + 1,
                original_text: format!("{}: {} {}", 
                    quick_transaction.date, 
                    quick_transaction.amount, 
                    quick_transaction.remark),
                error_reason: format!("日期格式无效: {}", e),
            });
            continue;
        }
        
        // 创建解析后的交易记录（供前端展示和编辑）
        let parsed_transaction = models::ParsedTransaction {
            original_text: format!("{}: {} {}", 
                quick_transaction.date, 
                quick_transaction.amount, 
                quick_transaction.remark),
            date: quick_transaction.date.clone(),
            amount: quick_transaction.amount,
            transaction_type: quick_transaction.transaction_type.clone(),
            category_name: quick_transaction.category.clone(),
            category_id,
            description: quick_transaction.remark.clone(),
            confidence: 0.9, // AI识别的置信度
        };
        
        parsed_transactions.push(parsed_transaction);
        println!("✅ [QuickBooking] 交易记录解析完成");
    }
    
    // 5. 返回解析结果（不保存到数据库）
    let success = !parsed_transactions.is_empty();
    let message = if success {
        if failed_lines.is_empty() {
            format!("成功解析出{}条记录，请确认后保存", parsed_transactions.len())
        } else {
            format!("成功解析{}条记录，{}条失败，请确认后保存", parsed_transactions.len(), failed_lines.len())
        }
    } else {
        "AI解析失败，没有识别到有效的记账记录".to_string()
    };
    
    println!("🎊 [QuickBooking] AI解析完成!");
    println!("   解析成功: {}", parsed_transactions.len());
    println!("   解析失败: {}", failed_lines.len());
    println!("   处理结果: {}", if success { "成功" } else { "失败" });
    println!("   返回消息: {}", message);
    
    if !failed_lines.is_empty() {
        println!("❌ [QuickBooking] 失败详情:");
        for failed in &failed_lines {
            println!("   第{}行: {} - {}", failed.line_number, failed.original_text, failed.error_reason);
        }
    }
    
    if !parsed_transactions.is_empty() {
        println!("✅ [QuickBooking] 解析成功详情:");
        for (index, parsed) in parsed_transactions.iter().enumerate() {
            println!("   交易{}: {} {} {} {}", 
                index + 1, 
                parsed.date, 
                parsed.amount, 
                parsed.transaction_type, 
                parsed.category_name
            );
        }
    }
    
    Ok(QuickBookingResult {
        success,
        message,
        parsed_transactions,
        failed_lines,
    })
}

// 保存用户确认后的交易记录
#[tauri::command]
async fn save_confirmed_transactions(
    state: State<'_, DatabaseState>, 
    request: SaveTransactionsRequest
) -> Result<SaveTransactionsResult, String> {
    println!("💾 [SaveTransactions] 开始保存用户确认的交易记录");
    println!("   待保存交易数量: {}", request.transactions.len());
    
    let mut saved_count = 0;
    let mut failed_count = 0;
    let mut error_messages = Vec::new();
    
    for (index, confirmed_transaction) in request.transactions.iter().enumerate() {
        println!("💾 [SaveTransactions] 保存第{}条交易:", index + 1);
        println!("   日期: {}", confirmed_transaction.date);
        println!("   金额: {}", confirmed_transaction.amount);
        println!("   类型: {}", confirmed_transaction.transaction_type);
        println!("   分类ID: {}", confirmed_transaction.category_id);
        println!("   描述: {}", confirmed_transaction.description);
        
        // 解析日期
        let date = match chrono::NaiveDate::parse_from_str(&confirmed_transaction.date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(e) => {
                let error_msg = format!("第{}条记录日期格式错误: {}", index + 1, e);
                println!("❌ [SaveTransactions] {}", error_msg);
                error_messages.push(error_msg);
                failed_count += 1;
                continue;
            }
        };
        
        // 验证金额
        if confirmed_transaction.amount <= 0.0 {
            let error_msg = format!("第{}条记录金额必须大于0", index + 1);
            println!("❌ [SaveTransactions] {}", error_msg);
            error_messages.push(error_msg);
            failed_count += 1;
            continue;
        }
        
        // 验证交易类型
        if confirmed_transaction.transaction_type != "income" && confirmed_transaction.transaction_type != "expense" {
            let error_msg = format!("第{}条记录交易类型无效: {}", index + 1, confirmed_transaction.transaction_type);
            println!("❌ [SaveTransactions] {}", error_msg);
            error_messages.push(error_msg);
            failed_count += 1;
            continue;
        }
        
        // 验证分类是否存在
        let category_exists = match state.db.get_categories_by_type(&confirmed_transaction.transaction_type).await {
            Ok(categories) => categories.iter().any(|cat| cat.id == confirmed_transaction.category_id),
            Err(e) => {
                let error_msg = format!("第{}条记录验证分类失败: {}", index + 1, e);
                println!("❌ [SaveTransactions] {}", error_msg);
                error_messages.push(error_msg);
                failed_count += 1;
                continue;
            }
        };
        
        if !category_exists {
            let error_msg = format!("第{}条记录分类ID不存在: {}", index + 1, confirmed_transaction.category_id);
            println!("❌ [SaveTransactions] {}", error_msg);
            error_messages.push(error_msg);
            failed_count += 1;
            continue;
        }
        
        // 创建交易记录
        let transaction_data = models::NewTransaction {
            date,
            r#type: confirmed_transaction.transaction_type.clone(),
            amount: confirmed_transaction.amount,
            category_id: confirmed_transaction.category_id,
            budget_id: confirmed_transaction.budget_id,
            description: Some(confirmed_transaction.description.clone()),
            note: Some(confirmed_transaction.description.clone()),
        };
        
        // 保存到数据库
        match state.db.create_transaction(&transaction_data).await {
            Ok(transaction_id) => {
                println!("✅ [SaveTransactions] 第{}条交易保存成功，ID: {}", index + 1, transaction_id);
                saved_count += 1;
            }
            Err(e) => {
                let error_msg = format!("第{}条记录保存失败: {}", index + 1, e);
                println!("❌ [SaveTransactions] {}", error_msg);
                error_messages.push(error_msg);
                failed_count += 1;
            }
        }
    }
    
    // 返回保存结果
    let success = saved_count > 0;
    let message = if failed_count == 0 {
        format!("成功保存{}条交易记录", saved_count)
    } else if saved_count == 0 {
        format!("所有{}条记录保存失败", failed_count)
    } else {
        format!("成功保存{}条记录，{}条失败", saved_count, failed_count)
    };
    
    println!("🎊 [SaveTransactions] 保存完成!");
    println!("   成功保存: {}", saved_count);
    println!("   保存失败: {}", failed_count);
    println!("   结果消息: {}", message);
    
    if !error_messages.is_empty() {
        println!("❌ [SaveTransactions] 错误详情:");
        for error in &error_messages {
            println!("   {}", error);
        }
    }
    
    Ok(SaveTransactionsResult {
        success,
        message,
        saved_count,
        failed_count,
    })
}

// 辅助函数：解析单行文本为交易记录（供您实现时参考）
async fn parse_text_line_to_transaction(
    line: &str, 
    line_number: usize,
    _db: &Database
) -> Result<ProcessedTransaction, FailedLine> {
    // TODO: 实现文本解析逻辑
    // 建议使用AI模型来解析以下信息：
    // - 交易类型（收入/支出）
    // - 金额
    // - 日期（相对或绝对）
    // - 分类
    // - 描述
    
    Err(FailedLine {
        line_number,
        original_text: line.to_string(),
        error_reason: "解析功能待实现".to_string(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 智能分析 (ChatBI) 命令
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_analysis_sessions(
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<AnalysisSession>, String> {
    db_state.db.get_analysis_sessions().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_analysis_messages(
    db_state: State<'_, DatabaseState>,
    session_id: String,
) -> Result<Vec<AnalysisMessageRecord>, String> {
    db_state.db.get_analysis_messages(&session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_analysis_session(
    db_state: State<'_, DatabaseState>,
    session_id: String,
) -> Result<(), String> {
    db_state.db.delete_analysis_session(&session_id).await.map_err(|e| e.to_string())
}

/// 流式分析：接收用户消息 → 构建上下文 → 调用 LLM（SSE）→ 通过事件推送 chunk → 持久化
#[tauri::command]
async fn send_analysis_message_stream(
    app: tauri::AppHandle,
    db_state: State<'_, DatabaseState>,
    mcp_state: State<'_, McpState>,
    request: AnalysisStreamRequest,
) -> Result<(), String> {
    use crate::ai::agent::analysis::{AnalysisAgent, FinancialContext, McpToolsContext};
    use crate::utils::http_client::{AIHttpClient, ClientConfig, AIProvider, AIMessage};
    use chrono::{Local, Datelike};

    let sid = request.session_id.clone();

    // ── 辅助：发出 done / error 事件 ───────────────────────────────────
    let emit_err = |app: &tauri::AppHandle, sid: &str, msg: String| {
        let _ = app.emit("analysis-stream-chunk", StreamChunkPayload {
            session_id: sid.to_string(),
            chunk: String::new(),
            done: true,
            error: Some(msg),
        });
    };

    if request.message.trim().is_empty() {
        emit_err(&app, &sid, "请输入您的问题".into());
        return Ok(());
    }

    // 1. 获取 LLM 配置（按 config_id 优先，否则 fallback 到 active）
    let llm_config = match request.config_id {
        Some(cid) => {
            match db_state.db.get_llm_config_by_id(cid).await {
                Ok(Some(c)) => c,
                _ => match db_state.db.get_active_llm_config().await {
                    Ok(Some(c)) => c,
                    _ => { emit_err(&app, &sid, "请先在设置中配置大模型接口".into()); return Ok(()); }
                },
            }
        }
        None => match db_state.db.get_active_llm_config().await {
            Ok(Some(c)) => c,
            _ => { emit_err(&app, &sid, "请先在设置中配置大模型接口".into()); return Ok(()); }
        },
    };

    // 2. 读取历史（在保存本轮 user 消息之前，20 条 = 10 轮）
    let history_records = db_state.db
        .get_recent_analysis_messages(&sid, 20)
        .await
        .unwrap_or_default();
    let history: Vec<AIMessage> = history_records
        .iter()
        .map(|r| AIMessage { role: r.role.clone(), content: r.content.clone() })
        .collect();

    // 3. 确保会话存在 + 持久化 user 消息
    let title: String = request.message.chars().take(30).collect();
    if let Err(e) = db_state.db.ensure_analysis_session(&sid, &title, request.config_id).await {
        emit_err(&app, &sid, format!("创建会话失败: {}", e));
        return Ok(());
    }
    let _ = db_state.db.save_analysis_message(&sid, "user", &request.message).await;

    // 4. 构建 HTTP 客户端
    let client_config = ClientConfig {
        provider: AIProvider::Custom(llm_config.provider.clone()),
        base_url: llm_config.base_url.clone(),
        api_key: llm_config.api_key.clone(),
        timeout_secs: 600,
        max_retries: 1,
        headers: HashMap::new(),
    };
    let http_client = match AIHttpClient::new(client_config) {
        Ok(c) => c,
        Err(e) => { emit_err(&app, &sid, format!("创建 HTTP 客户端失败: {}", e)); return Ok(()); }
    };

    // 5. 构建财务上下文
    let today = Local::now();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    let month_end = {
        let next = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
        };
        next.and_then(|d| d.pred_opt()).unwrap_or(month_start)
    };

    let financial_context = FinancialContext {
        monthly_stats: db_state.db.get_monthly_stats(3).await.unwrap_or_default(),
        expense_category_stats: db_state.db.get_category_stats(&month_start, &month_end, "expense").await.unwrap_or_default(),
        income_category_stats: db_state.db.get_category_stats(&month_start, &month_end, "income").await.unwrap_or_default(),
    };

    // 6. 收集 MCP 工具上下文
    let mcp_tools = mcp_state.manager.get_all_tools().await;
    let mcp_ctx = if mcp_tools.is_empty() {
        None
    } else {
        Some(McpToolsContext { tools: mcp_tools })
    };

    // 7. 组装 AI 请求
    let agent = AnalysisAgent::new(
        llm_config.model.clone(),
        llm_config.temperature as f32,
        llm_config.max_tokens as u32,
        llm_config.enable_thinking,
    );

    let system_prompt = agent.build_system_prompt_with_tools(&financial_context, mcp_ctx.as_ref());
    let mut messages = vec![crate::utils::http_client::AIMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];
    let max_history = 20;
    let start = if history.len() > max_history { history.len() - max_history } else { 0 };
    messages.extend_from_slice(&history[start..]);
    messages.push(crate::utils::http_client::AIMessage {
        role: "user".to_string(),
        content: request.message.clone(),
    });
    let ai_request = crate::utils::http_client::AIRequest {
        model: llm_config.model.clone(),
        messages,
        temperature: llm_config.temperature as f32,
        max_tokens: llm_config.max_tokens as u32,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stream: None,
        enable_thinking: llm_config.enable_thinking,
    };

    // 7. 发起流式请求
    let mut response = match http_client.chat_completion_stream(ai_request).await {
        Ok(r) => r,
        Err(e) => { emit_err(&app, &sid, format!("请求失败: {}", e)); return Ok(()); }
    };

    // 8. 解析 SSE 并逐 chunk 推送
    let mut full_content = String::new();
    let mut buffer = String::new();

    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim_end_matches('\r').to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if !line.starts_with("data: ") { continue; }
                    let data = line[6..].trim();
                    if data == "[DONE]" { continue; }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                full_content.push_str(content);
                                let _ = app.emit("analysis-stream-chunk", StreamChunkPayload {
                                    session_id: sid.clone(),
                                    chunk: content.to_string(),
                                    done: false,
                                    error: None,
                                });
                            }
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                emit_err(&app, &sid, format!("流式读取失败: {}", e));
                return Ok(());
            }
        }
    }

    // 9. 流结束
    let _ = app.emit("analysis-stream-chunk", StreamChunkPayload {
        session_id: sid.clone(),
        chunk: String::new(),
        done: true,
        error: None,
    });

    // 10. 持久化 assistant 回复 + 更新会话时间戳
    if !full_content.is_empty() {
        let _ = db_state.db.save_analysis_message(&sid, "assistant", &full_content).await;
    }
    let _ = db_state.db.touch_analysis_session(&sid).await;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP 服务器管理命令
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_mcp_servers(db_state: State<'_, DatabaseState>) -> Result<Vec<mcp::McpServerConfig>, String> {
    db_state.db.get_mcp_servers().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_mcp_server(
    db_state: State<'_, DatabaseState>,
    config: mcp::NewMcpServerConfig,
) -> Result<i64, String> {
    db_state.db.create_mcp_server(&config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_mcp_server(
    db_state: State<'_, DatabaseState>,
    id: i64,
    config: mcp::UpdateMcpServerConfig,
) -> Result<(), String> {
    db_state.db.update_mcp_server(id, &config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_mcp_server(
    db_state: State<'_, DatabaseState>,
    mcp_state: State<'_, McpState>,
    id: i64,
) -> Result<(), String> {
    // 先停止运行中的服务器
    mcp_state.manager.stop_server(id).await.ok();
    db_state.db.delete_mcp_server(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_mcp_server(
    db_state: State<'_, DatabaseState>,
    mcp_state: State<'_, McpState>,
    id: i64,
) -> Result<(), String> {
    let config = db_state.db.get_mcp_server_by_id(id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP 服务器配置不存在".to_string())?;

    mcp_state.manager.start_server(&config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_mcp_server(
    mcp_state: State<'_, McpState>,
    id: i64,
) -> Result<(), String> {
    mcp_state.manager.stop_server(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_mcp_server_status(
    db_state: State<'_, DatabaseState>,
    mcp_state: State<'_, McpState>,
) -> Result<Vec<mcp::McpServerStatus>, String> {
    let configs = db_state.db.get_mcp_servers().await.map_err(|e| e.to_string())?;
    Ok(mcp_state.manager.get_all_status(&configs).await)
}

#[tauri::command]
async fn get_mcp_tools(
    mcp_state: State<'_, McpState>,
    server_id: i64,
) -> Result<Vec<mcp::McpTool>, String> {
    mcp_state.manager.get_tools(server_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_all_mcp_tools(
    mcp_state: State<'_, McpState>,
) -> Result<Vec<serde_json::Value>, String> {
    let tools = mcp_state.manager.get_all_tools().await;
    let result: Vec<serde_json::Value> = tools.iter().map(|(server, tool)| {
        serde_json::json!({
            "server": server,
            "name": tool.name,
            "description": tool.description,
            "inputSchema": tool.input_schema,
        })
    }).collect();
    Ok(result)
}

#[tauri::command]
async fn call_mcp_tool(
    mcp_state: State<'_, McpState>,
    server_id: i64,
    tool_name: String,
    arguments: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<mcp::ToolCallResult, String> {
    mcp_state.manager.call_tool(server_id, &tool_name, arguments).await.map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 初始化 MCP 管理器（同步，无需 async）
            app.manage(McpState {
                manager: mcp::McpManager::new(),
            });

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_data_dir = app_handle.path().app_data_dir()
                    .expect("无法获取应用数据目录");
                    
                println!("应用数据目录: {}", app_data_dir.display());
                
                if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                    eprintln!("创建应用数据目录失败: {}", e);
                    return;
                }
                
                let db_path = app_data_dir.join("money_note.db");
                println!("数据库路径: {}", db_path.display());
                
                let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
                println!("尝试数据库URL: {}", database_url);
                
                match Database::new(&database_url).await {
                    Ok(db) => {
                        println!("数据库初始化成功");
                        app_handle.manage(DatabaseState { db });
                    }
                    Err(e) => {
                        eprintln!("数据库初始化失败: {}", e);
                        let simple_url = format!("sqlite:{}", db_path.to_string_lossy());
                        println!("尝试简单URL格式: {}", simple_url);
                        
                        match Database::new(&simple_url).await {
                            Ok(db) => {
                                println!("使用简单URL格式成功");
                                app_handle.manage(DatabaseState { db });
                            }
                            Err(e) => {
                                eprintln!("简单URL格式也失败: {}", e);
                                println!("尝试内存数据库作为备选...");
                                match Database::new("sqlite::memory:").await {
                                    Ok(db) => {
                                        println!("内存数据库初始化成功");
                                        app_handle.manage(DatabaseState { db });
                                    }
                                    Err(e) => {
                                        eprintln!("内存数据库也失败: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_categories,
            get_categories_by_type,
            create_category,
            update_category,
            delete_category,
            get_parent_categories,
            get_sub_categories,
            get_transactions,
            get_transactions_by_date_range,
            create_transaction,
            update_transaction,
            delete_transaction,
            get_monthly_stats,
            get_category_stats,
            get_budgets,
            create_budget,
            update_budget,
            delete_budget,
            import_transactions,
            export_csv_transactions,
            get_llm_configs,
            get_active_llm_config,
            save_llm_config,
            update_llm_config,
            set_active_llm_config,
            delete_llm_config,
            test_llm_connection,
            process_quick_booking_text,
            save_confirmed_transactions,
            get_analysis_sessions,
            get_analysis_messages,
            delete_analysis_session,
            send_analysis_message_stream,
            // MCP 命令
            get_mcp_servers,
            create_mcp_server,
            update_mcp_server,
            delete_mcp_server,
            start_mcp_server,
            stop_mcp_server,
            get_mcp_server_status,
            get_mcp_tools,
            get_all_mcp_tools,
            call_mcp_tool
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

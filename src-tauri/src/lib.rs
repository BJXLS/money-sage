// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Manager, State};
use chrono::NaiveDate;
use anyhow::Result;
use std::collections::HashMap;

mod models;
mod database;
mod utils;
mod ai;

#[cfg(test)]
mod tests;

use database::Database;
use models::*;

// 定义应用状态
pub struct DatabaseState {
    pub db: Database,
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

// 大模型配置相关命令
#[tauri::command]
async fn get_llm_config(state: State<'_, DatabaseState>) -> Result<Option<LLMConfig>, String> {
    state.db.get_llm_config().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_llm_config(state: State<'_, DatabaseState>, config: NewLLMConfig) -> Result<i64, String> {
    state.db.save_llm_config(&config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_llm_config(state: State<'_, DatabaseState>, id: i64, config: UpdateLLMConfig) -> Result<(), String> {
    state.db.update_llm_config(id, &config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_llm_config(state: State<'_, DatabaseState>, id: i64) -> Result<(), String> {
    state.db.delete_llm_config(id).await.map_err(|e| e.to_string())
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
    
    // 1. 获取当前的LLM配置
    let llm_config = match state.db.get_llm_config().await {
        Ok(Some(config)) => config,
        Ok(None) => {
            return Ok(QuickBookingResult {
                success: false,
                message: "请先配置大模型平台和API密钥".to_string(),
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
    
    // 2. 创建HTTP客户端和快速记账代理
    use crate::utils::http_client::{AIHttpClient, ClientConfig, AIProvider};
    
    // 根据平台创建客户端配置
    let provider = match llm_config.platform.as_str() {
        "Alibaba Bailian" => AIProvider::Custom("Alibaba".to_string()),
        "OpenAI" => AIProvider::OpenAI,
        "Claude" => AIProvider::Claude,
        "Gemini" => AIProvider::Gemini,
        _ => AIProvider::Custom(llm_config.platform.clone()),
    };


    // 创建自定义headers
    // let mut headers = std::collections::HashMap::new();
    // headers.insert(
    //     "Authorization".to_string(), 
    //     format!("Bearer {}", llm_config.app_key)
    // );
    // headers.insert(
    //     "Content-Type".to_string(), 
    //     "application/json".to_string()
    // );

    let config = ClientConfig {
        provider,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(), // 阿里云百炼兼容模式API地址
        api_key: llm_config.app_key.clone(),
        timeout_secs: 30,
        max_retries: 3,
        headers: std::collections::HashMap::new()
    };
    
    let http_client = AIHttpClient::new(config)
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    let quick_note_agent = QuickNoteAgent::new();
    
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
            budget_id: None,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 初始化数据库
                let app_data_dir = app_handle.path().app_data_dir()
                    .expect("无法获取应用数据目录");
                    
                println!("应用数据目录: {}", app_data_dir.display());
                
                // 确保应用数据目录存在
                if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                    eprintln!("创建应用数据目录失败: {}", e);
                    return;
                }
                
                let db_path = app_data_dir.join("money_note.db");
                println!("数据库路径: {}", db_path.display());
                
                // 尝试不同的数据库URL格式
                let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
                println!("尝试数据库URL: {}", database_url);
                
                match Database::new(&database_url).await {
                    Ok(db) => {
                        println!("数据库初始化成功");
                        app_handle.manage(DatabaseState { db });
                    }
                    Err(e) => {
                        eprintln!("数据库初始化失败: {}", e);
                        // 尝试使用更简单的URL格式
                        let simple_url = format!("sqlite:{}", db_path.to_string_lossy());
                        println!("尝试简单URL格式: {}", simple_url);
                        
                        match Database::new(&simple_url).await {
                            Ok(db) => {
                                println!("使用简单URL格式成功");
                                app_handle.manage(DatabaseState { db });
                            }
                            Err(e) => {
                                eprintln!("简单URL格式也失败: {}", e);
                                // 最后尝试内存数据库
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
            delete_budget,
            import_transactions,
            export_csv_transactions,
            get_llm_config,
            save_llm_config,
            update_llm_config,
            delete_llm_config,
            process_quick_booking_text,
            save_confirmed_transactions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

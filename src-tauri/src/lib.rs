// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Manager, State};
use chrono::NaiveDate;
use anyhow::Result;
use std::collections::HashMap;

mod models;
mod database;

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
            export_csv_transactions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

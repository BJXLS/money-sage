use anyhow::{anyhow, Result};
use calamine::{open_workbook_auto, Data, Reader};
use chrono::{Duration, NaiveDate};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::models::{
    DataExportFormat, ExportDataRequest, ExportDataResult, ImportConflictStrategy, ImportDataRequest,
    ImportDataResult, ImportPreviewItem, ImportPreviewResult,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CategoryIo {
    id: Option<i64>,
    name: String,
    r#type: String,
    parent_id: Option<i64>,
    icon: Option<String>,
    color: Option<String>,
    is_system: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BudgetIo {
    id: Option<i64>,
    name: String,
    category_id: i64,
    amount: f64,
    budget_type: String,
    period_type: String,
    start_date: String,
    end_date: Option<String>,
    is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TransactionIo {
    id: Option<i64>,
    date: String,
    r#type: String,
    amount: f64,
    category_id: i64,
    budget_id: Option<i64>,
    description: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MemoryFactIo {
    id: Option<i64>,
    fact_type: String,
    key: Option<String>,
    value_json: String,
    source: Option<String>,
    confidence: Option<f64>,
    status: Option<String>,
    supersedes_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DataBundle {
    categories: Vec<CategoryIo>,
    budgets: Vec<BudgetIo>,
    transactions: Vec<TransactionIo>,
    memory_facts: Vec<MemoryFactIo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MoneySageExport {
    format: String,
    version: i32,
    schema_version: i32,
    exported_at: String,
    data_checksum: String,
    data: DataBundle,
}

struct ParsedBundle {
    data: DataBundle,
    file_type: String,
    warnings: Vec<String>,
    schema_version: Option<i32>,
    checksum_valid: Option<bool>,
}

pub async fn export_data(pool: &SqlitePool, req: ExportDataRequest) -> Result<ExportDataResult> {
    let data = load_bundle_from_db(pool).await?;
    match req.format {
        DataExportFormat::Excel => export_excel(&req.file_path, &data)?,
        DataExportFormat::MoneySage => export_moneysage(&req.file_path, &data)?,
    }
    Ok(ExportDataResult {
        file_path: req.file_path,
        categories: data.categories.len(),
        budgets: data.budgets.len(),
        transactions: data.transactions.len(),
        memory_facts: data.memory_facts.len(),
    })
}

pub async fn preview_import(_pool: &SqlitePool, file_path: &str) -> Result<ImportPreviewResult> {
    let parsed = parse_bundle_from_file(file_path)?;
    let data = parsed.data;
    let mut warnings = parsed.warnings;
    if data.categories.is_empty() && data.transactions.is_empty() && data.budgets.is_empty() && data.memory_facts.is_empty() {
        warnings.push("文件解析成功，但没有检测到可导入数据".to_string());
    }
    let category_ids = collect_existing_ids(_pool, "categories").await?;
    let budget_ids = collect_existing_ids(_pool, "budgets").await?;
    let transaction_ids = collect_existing_ids(_pool, "transactions").await?;
    let memory_fact_ids = collect_existing_ids(_pool, "memory_facts").await?;

    let (c_ins, c_upd, c_skip) = estimate_categories(&data.categories, &category_ids);
    let (b_ins, b_upd, b_skip) = estimate_budgets(&data.budgets, &budget_ids);
    let (t_ins, t_upd, t_skip) = estimate_transactions(&data.transactions, &transaction_ids);
    let (m_ins, m_upd, m_skip) = estimate_memory_facts(&data.memory_facts, &memory_fact_ids);

    Ok(ImportPreviewResult {
        file_type: parsed.file_type,
        schema_version: parsed.schema_version,
        checksum_valid: parsed.checksum_valid,
        items: vec![
            ImportPreviewItem {
                table: "categories".to_string(),
                rows: data.categories.len(),
                estimated_insert: c_ins,
                estimated_update: c_upd,
                estimated_skip: c_skip,
            },
            ImportPreviewItem {
                table: "budgets".to_string(),
                rows: data.budgets.len(),
                estimated_insert: b_ins,
                estimated_update: b_upd,
                estimated_skip: b_skip,
            },
            ImportPreviewItem {
                table: "transactions".to_string(),
                rows: data.transactions.len(),
                estimated_insert: t_ins,
                estimated_update: t_upd,
                estimated_skip: t_skip,
            },
            ImportPreviewItem {
                table: "memory_facts".to_string(),
                rows: data.memory_facts.len(),
                estimated_insert: m_ins,
                estimated_update: m_upd,
                estimated_skip: m_skip,
            },
        ],
        warnings,
    })
}

pub async fn import_data(pool: &SqlitePool, req: ImportDataRequest) -> Result<ImportDataResult> {
    let parsed = parse_bundle_from_file(&req.file_path)?;
    let bundle = parsed.data;
    let mut warnings = parsed.warnings;
    if let Some(false) = parsed.checksum_valid {
        warnings.push("检测到备份校验和不一致，已拒绝导入".to_string());
        return Err(anyhow!("导入失败：备份文件校验和不一致"));
    }
    let mut tx = pool.begin().await?;
    let mut result = ImportDataResult {
        categories: 0,
        budgets: 0,
        transactions: 0,
        memory_facts: 0,
        inserted: 0,
        updated: 0,
        skipped: 0,
        warnings: vec![],
    };

    if req.strategy == ImportConflictStrategy::ReplaceAll {
        sqlx::query("DELETE FROM transactions").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM budgets").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM categories").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM memory_facts_history").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM memory_facts").execute(&mut *tx).await?;
    }

    for item in bundle.categories {
        if item.name.trim().is_empty() {
            result.skipped += 1;
            warnings.push("发现空分类名称，已跳过".to_string());
            continue;
        }
        let existed = if let Some(id) = item.id {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM categories WHERE id = ?1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
                > 0
        } else {
            false
        };
        let q = match req.strategy {
            ImportConflictStrategy::Skip => {
                "INSERT OR IGNORE INTO categories(id, name, icon, color, type, parent_id, is_system, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE(?7, 0), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
            }
            ImportConflictStrategy::Upsert | ImportConflictStrategy::ReplaceAll => {
                "INSERT INTO categories(id, name, icon, color, type, parent_id, is_system, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE(?7, 0), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name,
                   icon=excluded.icon,
                   color=excluded.color,
                   type=excluded.type,
                   parent_id=excluded.parent_id,
                   is_system=excluded.is_system,
                   updated_at=CURRENT_TIMESTAMP"
            }
        };
        let affected = sqlx::query(q)
            .bind(item.id)
            .bind(item.name)
            .bind(item.icon)
            .bind(item.color)
            .bind(item.r#type)
            .bind(item.parent_id)
            .bind(item.is_system.map(i64::from))
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if affected > 0 {
            result.categories += 1;
            if existed && req.strategy != ImportConflictStrategy::Skip {
                result.updated += 1;
            } else {
                result.inserted += 1;
            }
        } else {
            result.skipped += 1;
        }
    }

    for item in bundle.budgets {
        let start_date = match parse_date(&item.start_date) {
            Some(d) => d,
            None => {
                result.skipped += 1;
                warnings.push(format!("预算 '{}' 的 start_date 无效，已跳过", item.name));
                continue;
            }
        };
        let end_date = item.end_date.as_deref().and_then(parse_date);
        let existed = if let Some(id) = item.id {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM budgets WHERE id = ?1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
                > 0
        } else {
            false
        };
        let q = match req.strategy {
            ImportConflictStrategy::Skip => {
                "INSERT OR IGNORE INTO budgets(id, name, category_id, amount, budget_type, period_type, start_date, end_date, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, COALESCE(?9,1), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
            }
            ImportConflictStrategy::Upsert | ImportConflictStrategy::ReplaceAll => {
                "INSERT INTO budgets(id, name, category_id, amount, budget_type, period_type, start_date, end_date, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, COALESCE(?9,1), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name,
                   category_id=excluded.category_id,
                   amount=excluded.amount,
                   budget_type=excluded.budget_type,
                   period_type=excluded.period_type,
                   start_date=excluded.start_date,
                   end_date=excluded.end_date,
                   is_active=excluded.is_active,
                   updated_at=CURRENT_TIMESTAMP"
            }
        };
        let affected = sqlx::query(q)
            .bind(item.id)
            .bind(item.name)
            .bind(item.category_id)
            .bind(item.amount)
            .bind(item.budget_type)
            .bind(item.period_type)
            .bind(start_date)
            .bind(end_date)
            .bind(item.is_active.map(i64::from))
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if affected > 0 {
            result.budgets += 1;
            if existed && req.strategy != ImportConflictStrategy::Skip {
                result.updated += 1;
            } else {
                result.inserted += 1;
            }
        } else {
            result.skipped += 1;
        }
    }

    for item in bundle.transactions {
        let date = match parse_date(&item.date) {
            Some(d) => d,
            None => {
                result.skipped += 1;
                warnings.push("发现无效交易日期，已跳过".to_string());
                continue;
            }
        };
        let existed = if let Some(id) = item.id {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM transactions WHERE id = ?1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
                > 0
        } else {
            false
        };
        let q = match req.strategy {
            ImportConflictStrategy::Skip => {
                "INSERT OR IGNORE INTO transactions(id, date, type, amount, category_id, budget_id, description, note, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
            }
            ImportConflictStrategy::Upsert | ImportConflictStrategy::ReplaceAll => {
                "INSERT INTO transactions(id, date, type, amount, category_id, budget_id, description, note, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                   date=excluded.date,
                   type=excluded.type,
                   amount=excluded.amount,
                   category_id=excluded.category_id,
                   budget_id=excluded.budget_id,
                   description=excluded.description,
                   note=excluded.note,
                   updated_at=CURRENT_TIMESTAMP"
            }
        };
        let affected = sqlx::query(q)
            .bind(item.id)
            .bind(date)
            .bind(item.r#type)
            .bind(item.amount)
            .bind(item.category_id)
            .bind(item.budget_id)
            .bind(item.description)
            .bind(item.note)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if affected > 0 {
            result.transactions += 1;
            if existed && req.strategy != ImportConflictStrategy::Skip {
                result.updated += 1;
            } else {
                result.inserted += 1;
            }
        } else {
            result.skipped += 1;
        }
    }

    for item in bundle.memory_facts {
        if item.fact_type.trim().is_empty() || item.value_json.trim().is_empty() {
            result.skipped += 1;
            continue;
        }
        let existed = if let Some(id) = item.id {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memory_facts WHERE id = ?1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
                > 0
        } else {
            false
        };
        let q = match req.strategy {
            ImportConflictStrategy::Skip => {
                "INSERT OR IGNORE INTO memory_facts(id, fact_type, key, value_json, source, confidence, status, supersedes_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, COALESCE(?5,'import'), COALESCE(?6,0.7), COALESCE(?7,'active'), ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
            }
            ImportConflictStrategy::Upsert | ImportConflictStrategy::ReplaceAll => {
                "INSERT INTO memory_facts(id, fact_type, key, value_json, source, confidence, status, supersedes_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, COALESCE(?5,'import'), COALESCE(?6,0.7), COALESCE(?7,'active'), ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                   fact_type=excluded.fact_type,
                   key=excluded.key,
                   value_json=excluded.value_json,
                   source=excluded.source,
                   confidence=excluded.confidence,
                   status=excluded.status,
                   supersedes_id=excluded.supersedes_id,
                   updated_at=CURRENT_TIMESTAMP"
            }
        };
        let affected = sqlx::query(q)
            .bind(item.id)
            .bind(item.fact_type)
            .bind(item.key)
            .bind(item.value_json)
            .bind(item.source)
            .bind(item.confidence)
            .bind(item.status)
            .bind(item.supersedes_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if affected > 0 {
            result.memory_facts += 1;
            if existed && req.strategy != ImportConflictStrategy::Skip {
                result.updated += 1;
            } else {
                result.inserted += 1;
            }
        } else {
            result.skipped += 1;
        }
    }

    tx.commit().await?;
    result.warnings = warnings;
    Ok(result)
}

fn parse_bundle_from_file(file_path: &str) -> Result<ParsedBundle> {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".xlsx") {
        let (bundle, warnings) = parse_excel_bundle(file_path)?;
        Ok(ParsedBundle {
            data: bundle,
            file_type: "excel".to_string(),
            warnings,
            schema_version: None,
            checksum_valid: None,
        })
    } else if lower.ends_with(".moneysage") || lower.ends_with(".json") {
        let txt = std::fs::read_to_string(file_path)?;
        let env: MoneySageExport = serde_json::from_str(&txt)?;
        let computed = checksum_for_bundle(&env.data)?;
        let checksum_valid = computed == env.data_checksum;
        let mut warnings = vec![];
        if env.format != "money-sage-export" {
            warnings.push("文件格式标识异常，已按 moneysage 尝试解析".to_string());
        }
        if !checksum_valid {
            warnings.push("数据校验和不匹配，文件可能已被修改".to_string());
        }
        Ok(ParsedBundle {
            data: env.data,
            file_type: "moneysage".to_string(),
            warnings,
            schema_version: Some(env.schema_version),
            checksum_valid: Some(checksum_valid),
        })
    } else {
        Err(anyhow!("不支持的文件类型，仅支持 .xlsx / .moneysage / .json"))
    }
}

fn parse_excel_bundle(file_path: &str) -> Result<(DataBundle, Vec<String>)> {
    let mut workbook = open_workbook_auto(file_path)?;
    let mut warnings = vec![];
    let categories = parse_categories_sheet(&mut workbook, "categories", &mut warnings)?;
    let budgets = parse_budgets_sheet(&mut workbook, "budgets", &mut warnings)?;
    let transactions = parse_transactions_sheet(&mut workbook, "transactions", &mut warnings)?;
    let memory_facts = parse_memory_facts_sheet(&mut workbook, "memory_facts", &mut warnings)?;
    Ok((
        DataBundle {
            categories,
            budgets,
            transactions,
            memory_facts,
        },
        warnings,
    ))
}

fn parse_categories_sheet(
    workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
    name: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<CategoryIo>> {
    let Ok(range) = workbook.worksheet_range(name) else {
        return Ok(vec![]);
    };
    let mut rows = range.rows();
    let headers = rows.next().map(header_index).unwrap_or_default();
    let mut out = vec![];
    for row in rows {
        let name_v = get_cell(row, &headers, "name").unwrap_or_default();
        if name_v.trim().is_empty() {
            continue;
        }
        out.push(CategoryIo {
            id: get_cell(row, &headers, "id").and_then(|x| x.parse::<i64>().ok()),
            name: name_v,
            r#type: get_cell(row, &headers, "type").unwrap_or_else(|| "expense".to_string()),
            parent_id: get_cell(row, &headers, "parent_id").and_then(|x| x.parse::<i64>().ok()),
            icon: get_cell(row, &headers, "icon").filter(|x| !x.is_empty()),
            color: get_cell(row, &headers, "color").filter(|x| !x.is_empty()),
            is_system: get_cell(row, &headers, "is_system").and_then(parse_bool),
        });
    }
    if out.is_empty() {
        warnings.push("Excel 中 categories sheet 为空或不存在".to_string());
    }
    Ok(out)
}

fn parse_budgets_sheet(
    workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
    name: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<BudgetIo>> {
    let Ok(range) = workbook.worksheet_range(name) else {
        return Ok(vec![]);
    };
    let mut rows = range.rows();
    let headers = rows.next().map(header_index).unwrap_or_default();
    let mut out = vec![];
    for row in rows {
        let name_v = get_cell(row, &headers, "name").unwrap_or_default();
        if name_v.trim().is_empty() {
            continue;
        }
        out.push(BudgetIo {
            id: get_cell(row, &headers, "id").and_then(|x| x.parse::<i64>().ok()),
            name: name_v,
            category_id: get_cell(row, &headers, "category_id")
                .and_then(|x| x.parse::<i64>().ok())
                .unwrap_or(0),
            amount: get_cell(row, &headers, "amount")
                .and_then(|x| x.parse::<f64>().ok())
                .unwrap_or(0.0),
            budget_type: get_cell(row, &headers, "budget_type").unwrap_or_else(|| "time".to_string()),
            period_type: get_cell(row, &headers, "period_type").unwrap_or_else(|| "monthly".to_string()),
            start_date: get_cell(row, &headers, "start_date").unwrap_or_default(),
            end_date: get_cell(row, &headers, "end_date").filter(|x| !x.is_empty()),
            is_active: get_cell(row, &headers, "is_active").and_then(parse_bool),
        });
    }
    if out.is_empty() {
        warnings.push("Excel 中 budgets sheet 为空或不存在".to_string());
    }
    Ok(out)
}

fn parse_transactions_sheet(
    workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
    name: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<TransactionIo>> {
    let Ok(range) = workbook.worksheet_range(name) else {
        return Ok(vec![]);
    };
    let mut rows = range.rows();
    let headers = rows.next().map(header_index).unwrap_or_default();
    let mut out = vec![];
    for row in rows {
        let date_v = get_cell(row, &headers, "date").unwrap_or_default();
        if date_v.trim().is_empty() {
            continue;
        }
        out.push(TransactionIo {
            id: get_cell(row, &headers, "id").and_then(|x| x.parse::<i64>().ok()),
            date: date_v,
            r#type: get_cell(row, &headers, "type").unwrap_or_else(|| "expense".to_string()),
            amount: get_cell(row, &headers, "amount")
                .and_then(|x| x.parse::<f64>().ok())
                .unwrap_or(0.0),
            category_id: get_cell(row, &headers, "category_id")
                .and_then(|x| x.parse::<i64>().ok())
                .unwrap_or(0),
            budget_id: get_cell(row, &headers, "budget_id").and_then(|x| x.parse::<i64>().ok()),
            description: get_cell(row, &headers, "description").filter(|x| !x.is_empty()),
            note: get_cell(row, &headers, "note").filter(|x| !x.is_empty()),
        });
    }
    if out.is_empty() {
        warnings.push("Excel 中 transactions sheet 为空或不存在".to_string());
    }
    Ok(out)
}

fn parse_memory_facts_sheet(
    workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
    name: &str,
    _warnings: &mut Vec<String>,
) -> Result<Vec<MemoryFactIo>> {
    let Ok(range) = workbook.worksheet_range(name) else {
        return Ok(vec![]);
    };
    let mut rows = range.rows();
    let headers = rows.next().map(header_index).unwrap_or_default();
    let mut out = vec![];
    for row in rows {
        let fact_type = get_cell(row, &headers, "fact_type").unwrap_or_default();
        if fact_type.trim().is_empty() {
            continue;
        }
        out.push(MemoryFactIo {
            id: get_cell(row, &headers, "id").and_then(|x| x.parse::<i64>().ok()),
            fact_type,
            key: get_cell(row, &headers, "key").filter(|x| !x.is_empty()),
            value_json: get_cell(row, &headers, "value_json").unwrap_or_else(|| "{}".to_string()),
            source: get_cell(row, &headers, "source").filter(|x| !x.is_empty()),
            confidence: get_cell(row, &headers, "confidence").and_then(|x| x.parse::<f64>().ok()),
            status: get_cell(row, &headers, "status").filter(|x| !x.is_empty()),
            supersedes_id: get_cell(row, &headers, "supersedes_id").and_then(|x| x.parse::<i64>().ok()),
        });
    }
    Ok(out)
}

fn header_index(row: &[Data]) -> std::collections::HashMap<String, usize> {
    row.iter()
        .enumerate()
        .map(|(idx, c)| (cell_to_string(c).to_lowercase(), idx))
        .collect()
}

fn get_cell(row: &[Data], headers: &std::collections::HashMap<String, usize>, key: &str) -> Option<String> {
    headers
        .get(&key.to_lowercase())
        .and_then(|idx| row.get(*idx))
        .map(cell_to_string)
        .map(|x| x.trim().to_string())
}

fn cell_to_string(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(s) => s.to_string(),
        Data::Float(f) => {
            if f.fract().abs() < f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        other => other.to_string(),
    }
}

fn parse_bool(s: String) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let trimmed = s.trim();
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(trimmed, "%Y/%m/%d").ok())
        .or_else(|| {
            trimmed.parse::<f64>().ok().and_then(|serial| {
                let base = NaiveDate::from_ymd_opt(1899, 12, 30)?;
                Some(base + Duration::days(serial as i64))
            })
        })
}

async fn load_bundle_from_db(pool: &SqlitePool) -> Result<DataBundle> {
    let category_rows = sqlx::query(
        "SELECT id, name, type, parent_id, icon, color, is_system FROM categories ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    let categories = category_rows
        .into_iter()
        .map(|r| CategoryIo {
            id: Some(r.get("id")),
            name: r.get("name"),
            r#type: r.get("type"),
            parent_id: r.try_get("parent_id").ok(),
            icon: r.try_get("icon").ok(),
            color: r.try_get("color").ok(),
            is_system: Some(r.try_get::<i64, _>("is_system").unwrap_or(0) != 0),
        })
        .collect();

    let budget_rows = sqlx::query(
        "SELECT id, name, category_id, amount, budget_type, period_type, start_date, end_date, is_active FROM budgets ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    let budgets = budget_rows
        .into_iter()
        .map(|r| BudgetIo {
            id: Some(r.get("id")),
            name: r.get("name"),
            category_id: r.get("category_id"),
            amount: r.get("amount"),
            budget_type: r.get("budget_type"),
            period_type: r.get("period_type"),
            start_date: r.get::<String, _>("start_date"),
            end_date: r.try_get("end_date").ok(),
            is_active: Some(r.try_get::<i64, _>("is_active").unwrap_or(1) != 0),
        })
        .collect();

    let tx_rows = sqlx::query(
        "SELECT id, date, type, amount, category_id, budget_id, description, note FROM transactions ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    let transactions = tx_rows
        .into_iter()
        .map(|r| TransactionIo {
            id: Some(r.get("id")),
            date: r.get::<String, _>("date"),
            r#type: r.get("type"),
            amount: r.get("amount"),
            category_id: r.get("category_id"),
            budget_id: r.try_get("budget_id").ok(),
            description: r.try_get("description").ok(),
            note: r.try_get("note").ok(),
        })
        .collect();

    let fact_rows = sqlx::query(
        "SELECT id, fact_type, key, value_json, source, confidence, status, supersedes_id FROM memory_facts ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    let memory_facts = fact_rows
        .into_iter()
        .map(|r| MemoryFactIo {
            id: Some(r.get("id")),
            fact_type: r.get("fact_type"),
            key: r.try_get("key").ok(),
            value_json: r.get("value_json"),
            source: r.try_get("source").ok(),
            confidence: r.try_get("confidence").ok(),
            status: r.try_get("status").ok(),
            supersedes_id: r.try_get("supersedes_id").ok(),
        })
        .collect();

    Ok(DataBundle {
        categories,
        budgets,
        transactions,
        memory_facts,
    })
}

async fn collect_existing_ids(pool: &SqlitePool, table: &str) -> Result<std::collections::HashSet<i64>> {
    let sql = match table {
        "categories" => "SELECT id FROM categories",
        "budgets" => "SELECT id FROM budgets",
        "transactions" => "SELECT id FROM transactions",
        "memory_facts" => "SELECT id FROM memory_facts",
        _ => return Err(anyhow!("unsupported table for id collection")),
    };
    let rows = sqlx::query(sql).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.get::<i64, _>("id")).collect())
}

fn estimate_categories(
    rows: &[CategoryIo],
    existing: &std::collections::HashSet<i64>,
) -> (usize, usize, usize) {
    let mut ins = 0;
    let mut upd = 0;
    let mut skip = 0;
    for r in rows {
        if r.name.trim().is_empty() {
            skip += 1;
            continue;
        }
        match r.id {
            Some(id) if existing.contains(&id) => upd += 1,
            _ => ins += 1,
        }
    }
    (ins, upd, skip)
}

fn estimate_budgets(rows: &[BudgetIo], existing: &std::collections::HashSet<i64>) -> (usize, usize, usize) {
    let mut ins = 0;
    let mut upd = 0;
    let mut skip = 0;
    for r in rows {
        if r.name.trim().is_empty() || parse_date(&r.start_date).is_none() {
            skip += 1;
            continue;
        }
        match r.id {
            Some(id) if existing.contains(&id) => upd += 1,
            _ => ins += 1,
        }
    }
    (ins, upd, skip)
}

fn estimate_transactions(
    rows: &[TransactionIo],
    existing: &std::collections::HashSet<i64>,
) -> (usize, usize, usize) {
    let mut ins = 0;
    let mut upd = 0;
    let mut skip = 0;
    for r in rows {
        if parse_date(&r.date).is_none() {
            skip += 1;
            continue;
        }
        match r.id {
            Some(id) if existing.contains(&id) => upd += 1,
            _ => ins += 1,
        }
    }
    (ins, upd, skip)
}

fn estimate_memory_facts(
    rows: &[MemoryFactIo],
    existing: &std::collections::HashSet<i64>,
) -> (usize, usize, usize) {
    let mut ins = 0;
    let mut upd = 0;
    let mut skip = 0;
    for r in rows {
        if r.fact_type.trim().is_empty() || r.value_json.trim().is_empty() {
            skip += 1;
            continue;
        }
        match r.id {
            Some(id) if existing.contains(&id) => upd += 1,
            _ => ins += 1,
        }
    }
    (ins, upd, skip)
}

fn checksum_for_bundle(bundle: &DataBundle) -> Result<String> {
    let json = serde_json::to_string(bundle)?;
    Ok(sha256_hex(json.as_bytes()))
}

fn sha256_hex(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

fn export_moneysage(file_path: &str, data: &DataBundle) -> Result<()> {
    let data_checksum = checksum_for_bundle(data)?;
    let env = MoneySageExport {
        format: "money-sage-export".to_string(),
        version: 1,
        schema_version: 1,
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        data_checksum,
        data: data.clone(),
    };
    let body = serde_json::to_string_pretty(&env)?;
    std::fs::write(file_path, body)?;
    Ok(())
}

fn export_excel(file_path: &str, data: &DataBundle) -> Result<()> {
    let mut workbook = Workbook::new();

    let readme = workbook.add_worksheet();
    readme.write_string(0, 0, "MoneySage 导出说明")?;
    readme.write_string(1, 0, "可编辑后再导入。支持 sheets: categories / budgets / transactions / memory_facts")?;

    let categories = workbook.add_worksheet();
    categories.set_name("categories")?;
    write_headers(categories, &["id", "name", "type", "parent_id", "icon", "color", "is_system"])?;
    for (idx, c) in data.categories.iter().enumerate() {
        let r = (idx + 1) as u32;
        write_opt_i64(categories, r, 0, c.id)?;
        categories.write_string(r, 1, &c.name)?;
        categories.write_string(r, 2, &c.r#type)?;
        write_opt_i64(categories, r, 3, c.parent_id)?;
        write_opt_str(categories, r, 4, c.icon.as_deref())?;
        write_opt_str(categories, r, 5, c.color.as_deref())?;
        write_opt_bool(categories, r, 6, c.is_system)?;
    }

    let budgets = workbook.add_worksheet();
    budgets.set_name("budgets")?;
    write_headers(
        budgets,
        &[
            "id",
            "name",
            "category_id",
            "amount",
            "budget_type",
            "period_type",
            "start_date",
            "end_date",
            "is_active",
        ],
    )?;
    for (idx, b) in data.budgets.iter().enumerate() {
        let r = (idx + 1) as u32;
        write_opt_i64(budgets, r, 0, b.id)?;
        budgets.write_string(r, 1, &b.name)?;
        budgets.write_number(r, 2, b.category_id as f64)?;
        budgets.write_number(r, 3, b.amount)?;
        budgets.write_string(r, 4, &b.budget_type)?;
        budgets.write_string(r, 5, &b.period_type)?;
        budgets.write_string(r, 6, &b.start_date)?;
        write_opt_str(budgets, r, 7, b.end_date.as_deref())?;
        write_opt_bool(budgets, r, 8, b.is_active)?;
    }

    let transactions = workbook.add_worksheet();
    transactions.set_name("transactions")?;
    write_headers(
        transactions,
        &[
            "id",
            "date",
            "type",
            "amount",
            "category_id",
            "budget_id",
            "description",
            "note",
        ],
    )?;
    for (idx, t) in data.transactions.iter().enumerate() {
        let r = (idx + 1) as u32;
        write_opt_i64(transactions, r, 0, t.id)?;
        transactions.write_string(r, 1, &t.date)?;
        transactions.write_string(r, 2, &t.r#type)?;
        transactions.write_number(r, 3, t.amount)?;
        transactions.write_number(r, 4, t.category_id as f64)?;
        write_opt_i64(transactions, r, 5, t.budget_id)?;
        write_opt_str(transactions, r, 6, t.description.as_deref())?;
        write_opt_str(transactions, r, 7, t.note.as_deref())?;
    }

    let memory_facts = workbook.add_worksheet();
    memory_facts.set_name("memory_facts")?;
    write_headers(
        memory_facts,
        &[
            "id",
            "fact_type",
            "key",
            "value_json",
            "source",
            "confidence",
            "status",
            "supersedes_id",
        ],
    )?;
    for (idx, m) in data.memory_facts.iter().enumerate() {
        let r = (idx + 1) as u32;
        write_opt_i64(memory_facts, r, 0, m.id)?;
        memory_facts.write_string(r, 1, &m.fact_type)?;
        write_opt_str(memory_facts, r, 2, m.key.as_deref())?;
        memory_facts.write_string(r, 3, &m.value_json)?;
        write_opt_str(memory_facts, r, 4, m.source.as_deref())?;
        if let Some(conf) = m.confidence {
            memory_facts.write_number(r, 5, conf)?;
        }
        write_opt_str(memory_facts, r, 6, m.status.as_deref())?;
        write_opt_i64(memory_facts, r, 7, m.supersedes_id)?;
    }

    workbook.save(file_path)?;
    Ok(())
}

fn write_headers(sheet: &mut rust_xlsxwriter::Worksheet, headers: &[&str]) -> Result<()> {
    for (idx, h) in headers.iter().enumerate() {
        sheet.write_string(0, idx as u16, *h)?;
    }
    Ok(())
}

fn write_opt_str(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: Option<&str>,
) -> Result<()> {
    if let Some(v) = value {
        sheet.write_string(row, col, v)?;
    }
    Ok(())
}

fn write_opt_i64(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: Option<i64>,
) -> Result<()> {
    if let Some(v) = value {
        sheet.write_number(row, col, v as f64)?;
    }
    Ok(())
}

fn write_opt_bool(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: Option<bool>,
) -> Result<()> {
    if let Some(v) = value {
        sheet.write_boolean(row, col, v)?;
    }
    Ok(())
}

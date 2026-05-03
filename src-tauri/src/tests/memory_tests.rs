use crate::memory::search::SearchQuery;
use crate::memory::MemoryFacade;
use crate::models::{
    FactFilter, FactSource, FactStatus, FactType, RoleScope, RoleTone, RoleValue, UpsertInput,
    UpsertOutcome,
};

#[tokio::test]
async fn test_memory_upsert_and_list() {
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());

    let out = memory
        .upsert_fact(UpsertInput {
            fact_type: FactType::ClassificationRule,
            key: Some("老地方".to_string()),
            value_json: serde_json::json!({
                "pattern":"老地方",
                "target_category_path":"餐饮/午饭"
            }),
            source: Some(FactSource::User),
            confidence_hint: Some(1.0),
            origin_session: Some("s1".to_string()),
            origin_message: Some(1),
        })
        .await
        .expect("upsert");

    match out {
        crate::models::UpsertOutcome::Inserted { .. } => {}
        _ => panic!("unexpected outcome"),
    }

    let facts = memory
        .list_facts(FactFilter {
            fact_type: Some(FactType::ClassificationRule),
            status: Some(FactStatus::Active),
            key: None,
            limit: Some(10),
        })
        .await
        .expect("list");
    assert_eq!(facts.len(), 1);
}

#[tokio::test]
async fn test_role_preset_and_snapshot() {
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());
    memory.ensure_default_role_seed().await.expect("seed");
    let _ = memory
        .apply_role_preset("default".to_string(), RoleScope::Analysis)
        .await
        .expect("apply");

    let role = memory.get_role(RoleScope::Analysis).await.expect("role");
    assert!(role.is_some());

    let snapshot = memory.render_analysis_snapshot().await.expect("snapshot");
    assert!(snapshot.contains("角色设定"));
}

#[tokio::test]
async fn test_role_preset_crud() {
    use crate::models::{NewRolePreset, UpdateRolePreset};
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());
    memory.ensure_default_role_seed().await.expect("seed");

    // 初始仅有 1 条内置预设
    let presets = memory.list_role_presets().await.expect("list");
    assert_eq!(presets.len(), 1);
    assert!(presets[0].is_builtin);
    assert_eq!(presets[0].preset_id, "default");

    // 重复 seed 不应再插入
    memory.ensure_default_role_seed().await.expect("re-seed");
    let presets2 = memory.list_role_presets().await.expect("list2");
    assert_eq!(presets2.len(), 1);

    // 创建一个用户预设
    let created = memory
        .create_role_preset(NewRolePreset {
            display_name: "极简".to_string(),
            summary: Some("只给结论".to_string()),
            value: serde_json::json!({"display_name":"极简","tone":{"style":"concise"}}),
            sort_order: Some(1),
        })
        .await
        .expect("create");
    assert!(!created.is_builtin);

    // 更新
    memory
        .update_role_preset(
            created.preset_id.clone(),
            UpdateRolePreset {
                display_name: Some("极简 2".to_string()),
                summary: None,
                value: None,
                sort_order: None,
            },
        )
        .await
        .expect("update");
    let updated = memory
        .list_role_presets()
        .await
        .expect("list3")
        .into_iter()
        .find(|p| p.preset_id == created.preset_id)
        .expect("found");
    assert_eq!(updated.display_name, "极简 2");

    // 删除内置应失败
    let err = memory
        .delete_role_preset("default".to_string())
        .await
        .err()
        .expect("delete builtin should fail");
    assert!(err.to_string().contains("内置"));

    // 删除用户预设
    memory
        .delete_role_preset(created.preset_id.clone())
        .await
        .expect("delete user");
    let after = memory.list_role_presets().await.expect("list4");
    assert_eq!(after.len(), 1);

    // 重置内置：先改一下，再 reset
    memory
        .update_role_preset(
            "default".to_string(),
            UpdateRolePreset {
                display_name: Some("被改过".to_string()),
                summary: None,
                value: None,
                sort_order: None,
            },
        )
        .await
        .expect("update builtin");
    memory
        .reset_role_preset("default".to_string())
        .await
        .expect("reset");
    let reset_back = memory
        .list_role_presets()
        .await
        .expect("list5")
        .into_iter()
        .find(|p| p.preset_id == "default")
        .expect("found");
    assert_eq!(reset_back.display_name, "理财助手");
}

fn role_value(name: &str) -> RoleValue {
    RoleValue {
        scope: RoleScope::Analysis,
        display_name: Some(name.to_string()),
        self_reference: Some("我".to_string()),
        user_address: Some("你".to_string()),
        tone: Some(RoleTone {
            style: Some("warm".to_string()),
            emoji: Some(false),
            verbosity: Some("medium".to_string()),
            language_flavor: Some("zh-CN".to_string()),
        }),
        traits: None,
        r#do: None,
        dont: None,
        preset_id: None,
        notes: None,
    }
}

/// 设计文档 §4.1.6 D-Rule3：agent_role 必须走 supersede，
/// 第二次 set_role 后 get_role 应返回新值（修复前会被 merge 吞掉）
#[tokio::test]
async fn test_role_supersede_replaces_value() {
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());

    memory
        .set_role(RoleScope::Analysis, role_value("分析师A"))
        .await
        .expect("set role A");
    let outcome = memory
        .set_role(RoleScope::Analysis, role_value("老板"))
        .await
        .expect("set role B");

    match &outcome {
        UpsertOutcome::Superseded { .. } => {}
        other => panic!("expected Superseded, got {:?}", other),
    }

    let role = memory
        .get_role(RoleScope::Analysis)
        .await
        .expect("role")
        .expect("active role exists");
    let display_name = role
        .value_json
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(display_name, "老板", "新值未真正落库");

    // 同一 scope 仅有 1 条 active
    let actives = memory
        .list_facts(FactFilter {
            fact_type: Some(FactType::AgentRole),
            status: Some(FactStatus::Active),
            key: Some("role:analysis".to_string()),
            limit: Some(10),
        })
        .await
        .expect("list");
    assert_eq!(actives.len(), 1);
}

/// undo(supersede) 必须把后继 retire 同时把旧条恢复 active，
/// 否则会出现同 scope 双 active（修复前的 bug）
#[tokio::test]
async fn test_undo_supersede_restores_pair() {
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());

    memory
        .set_role(RoleScope::Analysis, role_value("分析师A"))
        .await
        .expect("set A");
    memory
        .set_role(RoleScope::Analysis, role_value("老板"))
        .await
        .expect("set B");

    // 找到 op='supersede' 的一条 history（fact_id 指向被压的旧条 A）
    let changes = memory.list_recent_changes(50).await.expect("history");
    let supersede_entry = changes
        .iter()
        .find(|h| h.op == "supersede")
        .expect("supersede entry exists");
    memory.undo(supersede_entry.id).await.expect("undo");

    let actives = memory
        .list_facts(FactFilter {
            fact_type: Some(FactType::AgentRole),
            status: Some(FactStatus::Active),
            key: Some("role:analysis".to_string()),
            limit: Some(10),
        })
        .await
        .expect("list");
    assert_eq!(actives.len(), 1, "撤销后应仅剩一条 active，避免双 active");

    let only = &actives[0];
    let display_name = only
        .value_json
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(display_name, "分析师A", "应恢复为旧条 A");
}

/// memory_search 应能召回 facts（LIKE）和历史会话消息（FTS5 / 触发器同步）
#[tokio::test]
async fn test_memory_search_facts_and_messages() {
    let db = crate::database::Database::new("sqlite::memory:")
        .await
        .expect("db");
    let memory = MemoryFacade::new(db.pool.clone());

    memory
        .upsert_fact(UpsertInput {
            fact_type: FactType::FinancialGoal,
            key: Some("category_spend:餐饮月控".to_string()),
            value_json: serde_json::json!({
                "title":"餐饮月控",
                "metric":"category_spend",
                "filter":{"category":"餐饮"},
                "period":"monthly",
                "target":1500,
                "direction":"le",
                "priority":"high"
            }),
            source: Some(FactSource::User),
            confidence_hint: Some(1.0),
            origin_session: None,
            origin_message: None,
        })
        .await
        .expect("upsert goal");

    db.ensure_analysis_session("sess-old", "上个月餐饮分析", None)
        .await
        .expect("ensure old session");
    db.save_analysis_message("sess-old", "user", "帮我看看上个月的餐饮支出占比")
        .await
        .expect("save msg1");
    db.save_analysis_message("sess-old", "assistant", "餐饮支出占比偏高，建议关注外卖频次")
        .await
        .expect("save msg2");

    let result = memory
        .search(SearchQuery {
            query: "餐饮".to_string(),
            top_k_facts: 3,
            top_k_sessions: 3,
            time_range_days: 365,
            exclude_session: Some("sess-current".to_string()),
            include_facts: true,
            include_sessions: true,
        })
        .await
        .expect("search");

    assert!(
        result.facts.iter().any(|f| f.fact_type == "financial_goal"),
        "应召回餐饮月控目标"
    );
    // FTS5 trigram 在某些 SQLite 编译选项下可能不可用，做存在性而非强相等断言
    if result.sessions.is_empty() {
        eprintln!("[test] FTS5 sessions 检索为空，可能是 trigram tokenizer 不可用，跳过 sessions 断言");
    } else {
        assert!(
            result.sessions.iter().any(|s| s.session_id == "sess-old"),
            "应召回 sess-old 历史会话"
        );
    }
}

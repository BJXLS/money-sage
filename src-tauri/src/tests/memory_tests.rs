use crate::memory::MemoryFacade;
use crate::models::{FactFilter, FactSource, FactStatus, FactType, RoleScope, UpsertInput};

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
    memory.seed_default_roles().await.expect("seed");
    let _ = memory
        .apply_role_preset("gentle_coach".to_string(), RoleScope::Analysis)
        .await
        .expect("apply");

    let role = memory.get_role(RoleScope::Analysis).await.expect("role");
    assert!(role.is_some());

    let snapshot = memory.render_analysis_snapshot().await.expect("snapshot");
    assert!(snapshot.contains("角色设定"));
}

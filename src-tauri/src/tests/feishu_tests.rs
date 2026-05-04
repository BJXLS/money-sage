//! 飞书集成的数据库层测试（M2 范围）
//!
//! 只覆盖**不依赖外网**的层：UPSERT 幂等性 + 空表读取行为。
//! `identity::probe` 的端到端测试由用户用真飞书凭据手测，不在 CI 跑。

use crate::feishu::FeishuConfigInput;
use crate::tests::create_test_database_state;

/// 调 `upsert_feishu_config` 两次（同一个 `name='default'`）
/// 应只产生一行，且字段以最后一次为准。
#[tokio::test]
async fn feishu_config_upsert_idempotent() {
    let state = create_test_database_state().await.expect("test state");
    let db = &state.db;

    // 第一次 upsert
    let input1 = FeishuConfigInput {
        app_id: "cli_first".into(),
        app_secret: "secret_first".into(),
        domain: "feishu".into(),
        bind_llm_config_id: None,
        bind_role_scope: "analysis".into(),
        enabled: false,
    };
    let id1 = db.upsert_feishu_config(&input1).await.expect("first upsert");
    assert!(id1 > 0);

    // 第二次 upsert，同 name='default'，字段全部翻新
    let input2 = FeishuConfigInput {
        app_id: "cli_second".into(),
        app_secret: "secret_second".into(),
        domain: "lark".into(),
        bind_llm_config_id: Some(7),
        bind_role_scope: "analysis".into(),
        enabled: true,
    };
    let id2 = db.upsert_feishu_config(&input2).await.expect("second upsert");
    assert_eq!(id1, id2, "UPSERT 应保留同一行");

    // 行数仍为 1
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM feishu_configs")
        .fetch_one(&db.pool)
        .await
        .expect("count");
    assert_eq!(count.0, 1, "feishu_configs 单行约束被破坏");

    // get_feishu_config 返回最新字段
    let got = db
        .get_feishu_config()
        .await
        .expect("get")
        .expect("应有一条");
    assert_eq!(got.id, id2);
    assert_eq!(got.name, "default");
    assert_eq!(got.app_id, "cli_second");
    assert_eq!(got.app_secret, "secret_second");
    assert_eq!(got.domain, "lark");
    assert_eq!(got.bind_llm_config_id, Some(7));
    assert!(got.enabled);
}

/// 空 feishu_configs 表 → `get_feishu_config` 返回 `Ok(None)`，前端可展示空表单。
#[tokio::test]
async fn feishu_config_get_returns_none_when_empty() {
    let state = create_test_database_state().await.expect("test state");
    let db = &state.db;

    // 不调 upsert，直接 get
    let got = db.get_feishu_config().await.expect("get");
    assert!(got.is_none(), "空表应返回 None, 实际: {:?}", got);
}

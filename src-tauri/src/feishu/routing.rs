//! 飞书入站消息 → analysis_session 路由
//!
//! 设计文档 §6.2 / §8.1：每个 `user_open_id` 在 `feishu_user_sessions` 表里有一行，
//! `current_session_id` 指向 `analysis_sessions.id`。流水线写消息时拿到的 session_id 就来自这。
//!
//! 两种入口：
//! - [`resolve_or_create`]：常规进站消息走这里。命中已有映射 → 复用；否则新建一个 uuid，
//!   写入 `feishu_user_sessions` + `analysis_sessions`（后者由 [`analysis::run`] 内部 ensure）。
//! - [`new_session`]：处理 `/new` 命令时调用，强制创建一个新 uuid 并覆盖。
//!
//! 这一层故意不调 LLM、也不写 messages，只负责"把 open_id 翻译成 session_id"。

use uuid::Uuid;

use crate::database::Database;

use super::error::FeishuError;

/// 命中已有映射 → 复用；否则新建并写入。
pub async fn resolve_or_create(
    db: &Database,
    open_id: &str,
    user_name: Option<&str>,
) -> Result<String, FeishuError> {
    if let Some(row) = db.get_feishu_user_session(open_id).await? {
        if !row.current_session_id.is_empty() {
            return Ok(row.current_session_id);
        }
    }
    let new_sid = Uuid::new_v4().to_string();
    db.set_feishu_user_session(open_id, &new_sid, user_name).await?;
    Ok(new_sid)
}

/// 强制创建新会话并覆盖映射，用于 `/new` 命令。
pub async fn new_session(
    db: &Database,
    open_id: &str,
    user_name: Option<&str>,
) -> Result<String, FeishuError> {
    let new_sid = Uuid::new_v4().to_string();
    db.set_feishu_user_session(open_id, &new_sid, user_name).await?;
    Ok(new_sid)
}

/// 进站消息处理完毕后更新 `last_message_at`。
pub async fn touch(db: &Database, open_id: &str) -> Result<(), FeishuError> {
    db.touch_feishu_user_session(open_id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_database_state;

    #[tokio::test]
    async fn resolve_creates_new_when_absent() {
        let state = create_test_database_state().await.expect("test state");
        let sid = resolve_or_create(&state.db, "ou_alice", Some("Alice"))
            .await
            .expect("resolve");
        assert!(!sid.is_empty());

        // 同一个 open_id 第二次调用应当返回同一个 session_id
        let sid2 = resolve_or_create(&state.db, "ou_alice", Some("Alice"))
            .await
            .expect("resolve again");
        assert_eq!(sid, sid2, "已有映射应被复用");
    }

    #[tokio::test]
    async fn new_session_overrides_existing_mapping() {
        let state = create_test_database_state().await.expect("test state");
        let sid1 = resolve_or_create(&state.db, "ou_bob", None).await.unwrap();
        let sid2 = new_session(&state.db, "ou_bob", None).await.unwrap();
        assert_ne!(sid1, sid2, "/new 应当切换到新 session_id");

        // 后续 resolve_or_create 应返回最新 sid
        let sid3 = resolve_or_create(&state.db, "ou_bob", None).await.unwrap();
        assert_eq!(sid2, sid3);
    }

    #[tokio::test]
    async fn touch_updates_last_message_without_changing_session() {
        let state = create_test_database_state().await.expect("test state");
        let sid = resolve_or_create(&state.db, "ou_carl", None).await.unwrap();
        touch(&state.db, "ou_carl").await.unwrap();
        let row = state
            .db
            .get_feishu_user_session("ou_carl")
            .await
            .unwrap()
            .expect("row");
        assert_eq!(row.current_session_id, sid);
        assert!(row.last_message_at.is_some(), "touch 应当写入 last_message_at");
    }
}

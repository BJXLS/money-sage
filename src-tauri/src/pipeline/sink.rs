//! `MessageSink` trait + 内置实现
//!
//! 这一层把"流水线"与"传输通道"解耦：
//! - `pipeline::analysis::run(...)` 只负责跑 LLM + 工具循环 + 落库；
//! - 流式片段 / 工具状态 / 结束 / 错误事件都通过 `MessageSink` 推给具体通道。
//!
//! MVP 提供：
//! - `VueSink`：把事件 emit 到 `analysis-stream-chunk` 给 Vue 端（保持向后兼容）；
//! - 后续 M3 会加 `FeishuSink`，把同一组事件桥接到飞书 IM。

use async_trait::async_trait;
use tauri::{AppHandle, Emitter};

use crate::models::{StreamChunkPayload, ToolStatusPayload};

/// 工具调用状态阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatusKind {
    /// 工具开始调用（携带 tool_input）
    Calling,
    /// 工具调用结果（携带 tool_output）
    Result,
}

/// 工具状态事件载荷
#[derive(Debug, Clone)]
pub struct ToolStatus {
    pub tool_name: String,
    pub kind: ToolStatusKind,
    /// 用于 UI 的可读描述（例如 `"正在调用: query_database"`）
    pub description: Option<String>,
    /// 仅在 `Calling` 阶段填充
    pub tool_input: Option<String>,
    /// 仅在 `Result` 阶段填充
    pub tool_output: Option<String>,
}

/// 通用消息接收器
///
/// 实现者负责把 4 类事件转换成自身通道的具体动作（emit / 发消息 / 写日志 ...）。
/// 调用方约定：`on_done` 与 `on_error` **互斥**且每次 run 至多触发一次。
#[async_trait]
pub trait MessageSink: Send + Sync {
    /// 流式文本增量（assistant 回复的一段）
    async fn on_chunk(&self, chunk: &str);
    /// 工具调用阶段状态
    async fn on_tool_status(&self, status: ToolStatus);
    /// 整轮成功结束，`full_text` 为本轮 assistant 完整文本
    async fn on_done(&self, full_text: &str);
    /// 致命错误（请求失败 / 流读取失败 / 配置缺失等），运行将提前返回
    async fn on_error(&self, message: &str);
}

/// `VueSink`：把流水线事件发回桌面端 Vue UI
///
/// 与改造前的行为保持等价：
/// - chunk → `analysis-stream-chunk` `{ chunk, done=false }`
/// - tool_status → `analysis-stream-chunk` `{ tool_status, done=false }`
/// - done → `analysis-stream-chunk` `{ chunk:"", done=true }`
/// - error → `analysis-stream-chunk` `{ chunk:"", done=true, error:Some(_) }`
pub struct VueSink {
    app: AppHandle,
    session_id: String,
}

impl VueSink {
    pub fn new(app: AppHandle, session_id: String) -> Self {
        Self { app, session_id }
    }

    fn emit(&self, payload: StreamChunkPayload) {
        let _ = self.app.emit("analysis-stream-chunk", payload);
    }
}

#[async_trait]
impl MessageSink for VueSink {
    async fn on_chunk(&self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        self.emit(StreamChunkPayload {
            session_id: self.session_id.clone(),
            chunk: chunk.to_string(),
            done: false,
            error: None,
            tool_status: None,
        });
    }

    async fn on_tool_status(&self, status: ToolStatus) {
        let payload = ToolStatusPayload {
            tool_name: status.tool_name,
            status: match status.kind {
                ToolStatusKind::Calling => "calling".to_string(),
                ToolStatusKind::Result => "result".to_string(),
            },
            description: status.description,
            tool_input: status.tool_input,
            tool_output: status.tool_output,
        };
        self.emit(StreamChunkPayload {
            session_id: self.session_id.clone(),
            chunk: String::new(),
            done: false,
            error: None,
            tool_status: Some(payload),
        });
    }

    async fn on_done(&self, _full_text: &str) {
        self.emit(StreamChunkPayload {
            session_id: self.session_id.clone(),
            chunk: String::new(),
            done: true,
            error: None,
            tool_status: None,
        });
    }

    async fn on_error(&self, message: &str) {
        self.emit(StreamChunkPayload {
            session_id: self.session_id.clone(),
            chunk: String::new(),
            done: true,
            error: Some(message.to_string()),
            tool_status: None,
        });
    }
}

#[cfg(test)]
pub mod test_support {
    //! 用于 pipeline 单元测试的内存 sink
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    pub enum SinkEvent {
        Chunk(String),
        ToolStatus {
            tool_name: String,
            kind: ToolStatusKind,
            tool_input: Option<String>,
            tool_output: Option<String>,
        },
        Done(String),
        Error(String),
    }

    /// 把所有事件按时序记录到内存中，供测试断言
    pub struct RecordingSink {
        events: Mutex<Vec<SinkEvent>>,
    }

    impl RecordingSink {
        pub fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        pub fn snapshot(&self) -> Vec<SinkEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MessageSink for RecordingSink {
        async fn on_chunk(&self, chunk: &str) {
            self.events
                .lock()
                .unwrap()
                .push(SinkEvent::Chunk(chunk.to_string()));
        }
        async fn on_tool_status(&self, status: ToolStatus) {
            self.events.lock().unwrap().push(SinkEvent::ToolStatus {
                tool_name: status.tool_name,
                kind: status.kind,
                tool_input: status.tool_input,
                tool_output: status.tool_output,
            });
        }
        async fn on_done(&self, full_text: &str) {
            self.events
                .lock()
                .unwrap()
                .push(SinkEvent::Done(full_text.to_string()));
        }
        async fn on_error(&self, message: &str) {
            self.events
                .lock()
                .unwrap()
                .push(SinkEvent::Error(message.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{RecordingSink, SinkEvent};
    use super::*;

    #[tokio::test]
    async fn recording_sink_captures_event_order() {
        let sink = RecordingSink::new();
        sink.on_chunk("hello").await;
        sink.on_chunk(" world").await;
        sink.on_tool_status(ToolStatus {
            tool_name: "query_database".into(),
            kind: ToolStatusKind::Calling,
            description: None,
            tool_input: Some("{\"sql\":\"SELECT 1\"}".into()),
            tool_output: None,
        })
        .await;
        sink.on_tool_status(ToolStatus {
            tool_name: "query_database".into(),
            kind: ToolStatusKind::Result,
            description: None,
            tool_input: None,
            tool_output: Some("[{\"1\":1}]".into()),
        })
        .await;
        sink.on_done("hello world").await;

        let events = sink.snapshot();
        assert_eq!(events.len(), 5);
        match &events[0] {
            SinkEvent::Chunk(s) => assert_eq!(s, "hello"),
            other => panic!("unexpected first event: {:?}", other),
        }
        match &events[2] {
            SinkEvent::ToolStatus { tool_name, kind, tool_input, tool_output } => {
                assert_eq!(tool_name, "query_database");
                assert_eq!(*kind, ToolStatusKind::Calling);
                assert!(tool_input.as_deref().unwrap().contains("SELECT 1"));
                assert!(tool_output.is_none());
            }
            other => panic!("unexpected event: {:?}", other),
        }
        match &events[4] {
            SinkEvent::Done(s) => assert_eq!(s, "hello world"),
            other => panic!("unexpected last event: {:?}", other),
        }
    }

    #[tokio::test]
    async fn recording_sink_captures_error_path() {
        let sink = RecordingSink::new();
        sink.on_chunk("partial").await;
        sink.on_error("network broken").await;
        let events = sink.snapshot();
        assert_eq!(events.len(), 2);
        match &events[1] {
            SinkEvent::Error(s) => assert_eq!(s, "network broken"),
            other => panic!("unexpected event: {:?}", other),
        }
    }
}

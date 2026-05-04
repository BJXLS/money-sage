//! 流水线（pipeline）模块
//!
//! 把"跑一次智能分析对话"的内核从 Tauri 命令里抽出来，做成可被多个入站通道复用的 service：
//! - 桌面 Vue UI 通过 [`sink::VueSink`] 接事件；
//! - 后续 M3 接入飞书会通过 `FeishuSink`（暂未实现）接同一组事件。
//!
//! 入口：[`analysis::run`]。

pub mod analysis;
pub mod sink;

pub use analysis::{run as run_analysis, AnalysisInput};
#[allow(unused_imports)]
pub use sink::{MessageSink, ToolStatus, ToolStatusKind, VueSink};

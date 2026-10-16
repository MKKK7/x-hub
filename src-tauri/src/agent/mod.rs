//! Agent 底座（计划 §4/§5，#t44）：
//! - `context`：按 turn 截断 + 历史工具消息成对剪枝
//! - `approval`：权限三档判定 + 会话级授权（只存内存）
//! - `prompt`：system prompt 四段组装
//! - `tools`：工具注册表契约（ToolRegistry / ToolCtx / schema 组装）
//! - `builtin`：31 个内置工具实现（#t45 全量 30 + create_subcategory：读 12 / 写 14 / 危险 5）
//! - `turn`：循环编排（模型调用 ↔ 工具执行，10 轮上限、审批、中断）
//! - `state`：运行时状态（审批挂起表 / 会话授权 / 中断旗标，tauri managed）
pub mod approval;
pub mod builtin;
pub mod context;
pub mod prompt;
pub mod state;
pub mod tools;
pub mod turn;

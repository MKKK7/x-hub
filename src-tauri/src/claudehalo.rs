// ClaudeHalo 进度监控（Q2）
//
// ClaudeHaloManage 是另一个本地工具，提供 HTTP /status 端点反馈 Claude CLI 当前状态
// (idle / thinking / working / waiting_input)。x-hub 主动探测, 不与 ClaudeHalo 进程耦合。
//
// 多 session 区分：ClaudeHalo 启动时占 7700 端口, 多 session 各占 7700/7701/...
// 当前实现最多探测 8 个端口 (7700..7707), 超时 200ms / 端口, 总探测耗时上限 ~1.6s

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// ClaudeHalo /status JSON 响应子集
/// 完整字段见 ClaudeHaloManage/win/README-task.md, 这里只读需要的
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ClaudeHaloStatus {
    /// 监听的本地端口（7700 起）
    pub port: u16,
    /// Claude Code session id
    pub session_id: Option<String>,
    /// 当前状态: idle / thinking / working / waiting_input / unknown
    pub state: String,
    /// 主机名（多 session 时与 port 一起用于区分）
    pub hostname: Option<String>,
    /// 工作目录（ClaudeHaloManage 暂未暴露, 留作 Option）
    pub cwd: Option<String>,
    /// 探测时戳（毫秒）
    pub detected_at: i64,
}

const PROBE_PORTS: std::ops::RangeInclusive<u16> = 7700..=7707;
const PROBE_TIMEOUT: Duration = Duration::from_millis(200);

/// 探测单个端口: 200ms 内能连上 + 拿到 /status 即视为该端口有 ClaudeHalo 实例
async fn probe_port(port: u16) -> Option<ClaudeHaloStatus> {
    let url = format!("http://127.0.0.1:{}/status", port);
    let client = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let session_id = v.get("session_id").and_then(|x| x.as_str()).map(String::from);
    let state = v
        .get("state")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let hostname = v.get("hostname").and_then(|x| x.as_str()).map(String::from);
    let cwd = v.get("cwd").and_then(|x| x.as_str()).map(String::from);
    Some(ClaudeHaloStatus {
        port,
        session_id,
        state,
        hostname,
        cwd,
        detected_at: chrono::Utc::now().timestamp_millis(),
    })
}

/// 探测 7700..7707 所有端口, 返回当前在跑的 ClaudeHalo 实例列表
pub async fn discover_sessions() -> Vec<ClaudeHaloStatus> {
    let mut tasks = Vec::new();
    for port in PROBE_PORTS {
        tasks.push(tokio::spawn(probe_port(port)));
    }
    let mut results = Vec::new();
    for t in tasks {
        if let Ok(Some(s)) = t.await {
            results.push(s);
        }
    }
    results.sort_by_key(|s| s.port);
    results
}

/// Tauri 命令: 获取所有在跑的 ClaudeHalo session 状态
#[tauri::command]
pub async fn claudehalo_get_status() -> Vec<ClaudeHaloStatus> {
    log::trace!("探测 ClaudeHalo 状态 (端口 7700..7707)");
    discover_sessions().await
}
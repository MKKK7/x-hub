// CC Switch 集成（Q5）
//
// CC Switch 是本地独立 GUI 工具（GitHub 开源），用于管理 Claude Code / Codex / Gemini
// 多供应商配置。x-hub 集成原则：**只读探测**，不修改 cc-switch 配置。
//
// 探测策略：
// - 进程名 `cc-switch.exe` (Windows) / `cc-switch` (Linux/macOS)
// - 配置目录优先尝试:
//   1. ~/.cc-switch/  (Windows / Linux)
//   2. ~/Library/Application Support/cc-switch/  (macOS, 不探测, 该函数逻辑未提供)
//   3. ~/.config/cc-switch/  (Linux 备用)
//   4. %APPDATA%/cc-switch/  (Windows 备用)
// - settings.json 解析: 读 providers 数组统计当前活跃 provider
// - cc-switch.db (SQLite) 不打开 (避免与 cc-switch 自身锁冲突), 只探测存在性
//
// 不做的:
// - 启动/停止 cc-switch 进程
// - 修改 cc-switch 配置 (用户期望从 CC Switch 自己的 GUI 改)
// - 共享 API key (CC Switch 没有 IPC 暴露)
// - iframe 嵌入 (没有 HTTP 端点)

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchStatus {
    /// 进程是否在跑
    pub running: bool,
    /// 进程 PID（若 running）
    pub pid: Option<u32>,
    /// 配置文件目录绝对路径（若找到）
    pub config_dir: Option<String>,
    /// 当前活跃 provider 数（从 settings.json 的 providers 数组取）
    pub provider_count: u32,
    /// 当前选中 provider 名（settings.json 的 currentProvider 字段）
    pub current_provider: Option<String>,
    /// 探测时戳
    pub detected_at: i64,
}

/// 探测 CC Switch 进程名（跨平台）
#[cfg(target_os = "windows")]
fn process_names() -> &'static [&'static str] {
    &["cc-switch.exe", "CCSwitch.exe"]
}

#[cfg(not(target_os = "windows"))]
fn process_names() -> &'static [&'static str] {
    &["cc-switch", "ccswitch"]
}

/// 探测进程是否在跑（Windows 用 tasklist 拿 PID；其他平台退化为不探测进程只探测配置目录）
#[cfg(target_os = "windows")]
fn find_running_pid() -> Option<u32> {
    let names = process_names();
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", names[0])])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // 解析第二列（PID）；过滤表头行
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 2 && cols[0].eq_ignore_ascii_case(names[0]) {
            if let Ok(pid) = cols[1].parse::<u32>() {
                return Some(pid);
            }
        }
    }
    // 也试第二个候选名
    if names.len() > 1 {
        let output2 = std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", names[1])])
            .output()
            .ok()?;
        let text2 = String::from_utf8_lossy(&output2.stdout);
        for line in text2.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 2 && cols[0].eq_ignore_ascii_case(names[1]) {
                if let Ok(pid) = cols[1].parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_running_pid() -> Option<u32> {
    // Linux/macOS 用 pgrep 探测
    for name in process_names() {
        let output = std::process::Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output()
            .ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = text.lines().next() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }
    None
}

/// 候选配置目录列表（按优先级排序，第一个存在即用）
fn candidate_config_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".cc-switch"));
        dirs.push(home.join(".config").join("cc-switch"));
        #[cfg(target_os = "windows")]
        if let Some(roaming) = std::env::var_os("APPDATA") {
            let p = PathBuf::from(roaming).join("cc-switch");
            dirs.push(p);
        }
    }
    dirs
}

/// 读取 settings.json 解析 provider 数 + 当前 provider
fn read_settings(config_dir: &std::path::Path) -> (u32, Option<String>) {
    let p = config_dir.join("settings.json");
    let Ok(text) = std::fs::read_to_string(&p) else {
        return (0, None);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (0, None);
    };
    let count = v
        .get("providers")
        .and_then(|x| x.as_array())
        .map(|a| a.len() as u32)
        .unwrap_or(0);
    let cur = v
        .get("currentProvider")
        .or_else(|| v.get("current_provider"))
        .and_then(|x| x.as_str())
        .map(String::from);
    (count, cur)
}

#[tauri::command]
pub fn ccswitch_get_status() -> CcSwitchStatus {
    let pid = find_running_pid();
    let config_dir = candidate_config_dirs()
        .into_iter()
        .find(|d| d.is_dir());
    let (provider_count, current_provider) = match config_dir.as_ref() {
        Some(d) => read_settings(d),
        None => (0, None),
    };
    log::trace!(
        "探测 CC Switch: running={} pid={:?} providers={} current={:?}",
        pid.is_some(),
        pid,
        provider_count,
        current_provider
    );
    CcSwitchStatus {
        running: pid.is_some(),
        pid,
        config_dir: config_dir.map(|p| p.to_string_lossy().to_string()),
        provider_count,
        current_provider,
        detected_at: chrono::Utc::now().timestamp_millis(),
    }
}
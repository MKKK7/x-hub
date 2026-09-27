// ClaudeHalo 进度监控（Q2 重做：CLAUDEHALO 抄过来 + 适配多个）
//
// 参考: D:\Work\Project\ClaudeHaloManage\win\scripts\halo-hook.ps1
//   (Claude Code 配置 hook, 调用此 PS1 写状态文件)
//
// 状态文件:
//   - 位置: %TEMP%\claude-halo-state-{claude_pid}.txt (per-session)
//   - 旧版兼容: %TEMP%\claude-halo-state.txt
//   - 内容: 纯文本 (UTF-8 no BOM), 一行
//   - 取值: idle / thinking / working / waiting_input / compacting / completed
//
// x-hub 改造点 (相对原 CLAUDEHALO 单实例):
//   - 枚举所有 claude.exe (CLAUDEHALO 只返第一个, 用户原话 "要多个")
//   - 每个 PID 独立读状态文件 → 多个 session 独立状态
//
// 不依赖外部进程 (不需要 ClaudeHaloManage.exe):
//   - Toolhelp32 直接枚举
//   - 直接读状态文件
//   - 状态文件由 Claude Code hook (用户配在 ~/.claude/settings.json) 写入

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeHaloSession {
    /// Claude.exe 进程 PID
    pub pid: u32,
    /// 状态: idle / thinking / working / waiting_input / compacting / completed / unknown
    /// unknown = 进程在跑但找不到状态文件 (hook 未配)
    pub state: String,
    /// 状态文件绝对路径 (若找到, 否则 null)
    pub state_file: Option<String>,
    /// 探测时间戳 (毫秒)
    pub detected_at: i64,
}

// ── Win32 FFI (抄自 CLAUDEHALO platform.rs) ─────────────────────────
extern "system" {
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn CloseHandle(hObject: isize) -> i32;
}
const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const INVALID_HANDLE_VALUE: isize = -1;

#[repr(C)]
#[allow(non_snake_case)]
struct PROCESSENTRY32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

/// 枚举所有 claude.exe 进程 (返回 PID 列表)
fn find_claude_pids() -> Vec<u32> {
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/macOS 暂不实现 (CLAUDEHALO 仅 Windows)
        return Vec::new();
    }
    #[cfg(target_os = "windows")]
    {
        let mut pids = Vec::new();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == 0 || snapshot == INVALID_HANDLE_VALUE {
                return pids;
            }
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                cntUsage: 0,
                th32ProcessID: 0,
                th32DefaultHeapID: 0,
                th32ModuleID: 0,
                cntThreads: 0,
                th32ParentProcessID: 0,
                pcPriClassBase: 0,
                dwFlags: 0,
                szExeFile: [0u16; 260],
            };
            let target: Vec<u16> = "claude.exe\0".encode_utf16().collect();
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    // case-insensitive 前缀匹配 "claude.exe"
                    let mut matches = true;
                    for (i, &c) in target.iter().enumerate() {
                        if c == 0 {
                            break;
                        }
                        let ec = entry.szExeFile[i] as u8 as char;
                        let ec_l = ec.to_ascii_lowercase() as u16;
                        let c_l = (c as u8 as char).to_ascii_lowercase() as u16;
                        if ec_l != c_l {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        pids.push(entry.th32ProcessID);
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
        pids.sort_unstable();
        pids
    }
}

/// 读取单个 PID 的状态文件
fn read_state_for_pid(pid: u32) -> (String, Option<String>) {
    let tmp = std::env::temp_dir();
    let path = tmp.join(format!("claude-halo-state-{}.txt", pid));
    let (state, found) = read_state_file(&path);
    if found {
        return (state, Some(path.to_string_lossy().to_string()));
    }
    // fallback: 旧版单文件
    let legacy = tmp.join("claude-halo-state.txt");
    let (state2, found2) = read_state_file(&legacy);
    (state2, if found2 { Some(legacy.to_string_lossy().to_string()) } else { None })
}

fn read_state_file(path: &PathBuf) -> (String, bool) {
    let Ok(text) = fs::read_to_string(path) else {
        return ("unknown".to_string(), false);
    };
    let trimmed = text.trim().to_lowercase();
    // 取值白名单 (CLAUDEHALO 实际值), 其它归为 unknown
    let valid = matches!(
        trimmed.as_str(),
        "idle" | "thinking" | "working" | "waiting_input" | "compacting" | "completed"
    );
    if valid {
        (trimmed, true)
    } else {
        ("unknown".to_string(), true)
    }
}

#[tauri::command]
pub async fn claudehalo_get_status() -> Vec<ClaudeHaloSession> {
    // 探测是同步 (Toolhelp32 + 文件读), 不需要 async — 但保留 async 签名以备未来扩展
    let pids = find_claude_pids();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    let sessions: Vec<ClaudeHaloSession> = pids
        .into_iter()
        .map(|pid| {
            let (state, file) = read_state_for_pid(pid);
            ClaudeHaloSession {
                pid,
                state,
                state_file: file,
                detected_at: now,
            }
        })
        .collect();
    log::trace!("ClaudeHalo 探测: {} 个 claude.exe", sessions.len());
    sessions
}
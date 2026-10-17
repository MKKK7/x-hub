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
//   - 拿窗口标题 (EnumWindows + GetWindowText) 替代 PID 作为会话标识
//   - 自动清理死会话状态文件 (会话累积修复)
//   - 一键装 hook 到 ~/.claude/settings.json (含幂等)
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
    /// 会话标题 (窗口标题, 拿不到时 None)
    pub title: Option<String>,
    /// 探测时间戳 (毫秒)
    pub detected_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallHookResult {
    /// settings.json 绝对路径
    pub settings_path: String,
    /// 是否已配置 (幂等命中, 跳过注入)
    pub already_configured: bool,
    /// 实际使用的 hook 脚本路径
    pub script_path: String,
}

// ── Win32 FFI (抄自 CLAUDEHALO platform.rs) ─────────────────────────
extern "system" {
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn CloseHandle(hObject: isize) -> i32;
    fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
    fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
    fn GetWindowThreadProcessId(hWnd: isize, lpdwProcessId: *mut u32) -> u32;
    fn IsWindowVisible(hWnd: isize) -> i32;
    fn GetParent(hWnd: isize) -> isize;
}
type EnumWindowsProc = unsafe extern "system" fn(hWnd: isize, lParam: isize) -> i32;
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

struct EnumCtx {
    target_pid: u32,
    found_hwnd: isize,
    found_title: String,
}

unsafe extern "system" fn enum_proc(hWnd: isize, lParam: isize) -> i32 {
    unsafe {
        if IsWindowVisible(hWnd) == 0 {
            return 1;
        }
        if GetParent(hWnd) != 0 {
            return 1;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hWnd, &mut pid);
        if pid != (*(lParam as *const EnumCtx)).target_pid {
            return 1;
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hWnd, buf.as_mut_ptr(), buf.len() as i32);
        let ctx = &mut *(lParam as *mut EnumCtx);
        if ctx.found_hwnd != 0 {
            return 0;
        }
        ctx.found_hwnd = hWnd;
        if len > 0 {
            ctx.found_title = String::from_utf16_lossy(&buf[..len as usize])
                .trim()
                .to_string();
        }
        1
    }
}

fn find_claude_pids() -> Vec<u32> {
    #[cfg(not(target_os = "windows"))]
    {
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
                dwSize: std::mem::size_of::<PROCESSENTRY32W> as u32,
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

fn read_window_title(pid: u32) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let mut ctx = EnumCtx {
            target_pid: pid,
            found_hwnd: 0,
            found_title: String::new(),
        };
        unsafe {
            EnumWindows(enum_proc, (&mut ctx as *mut EnumCtx) as isize);
        }
        if ctx.found_title.is_empty() { None } else { Some(ctx.found_title) }
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn read_state_for_pid(pid: u32) -> (String, Option<String>) {
    let tmp = std::env::temp_dir();
    let path = tmp.join(format!("claude-halo-state-{}.txt", pid));
    let (state, found) = read_state_file(&path);
    if found {
        return (state, Some(path.to_string_lossy().to_string()));
    }
    let legacy = tmp.join("claude-halo-state.txt");
    let (state2, found2) = read_state_file(&legacy);
    (state2, if found2 { Some(legacy.to_string_lossy().to_string()) } else { None })
}

fn read_state_file(path: &PathBuf) -> (String, bool) {
    let Ok(text) = fs::read_to_string(path) else {
        return ("unknown".to_string(), false);
    };
    let trimmed = text.trim().to_lowercase();
    let valid = matches!(
        trimmed.as_str(),
        "idle" | "thinking" | "working" | "waiting_input" | "compacting" | "completed"
    );
    if valid { (trimmed, true) } else { ("unknown".to_string(), true) }
}

/// 清理死会话状态文件 (磁盘上有但当前没在跑的 PID 对应文件)
/// 返回被清理的 PID 列表
fn cleanup_dead_sessions(active_pids: &[u32]) -> Vec<u32> {
    #[cfg(not(target_os = "windows"))]
    return Vec::new();
    #[cfg(target_os = "windows")]
    {
        let mut cleaned = Vec::new();
        let tmp = std::env::temp_dir();
        let Ok(entries) = fs::read_dir(&tmp) else { return cleaned; };
        let prefix = "claude-halo-state-";
        let suffix = ".txt";
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue; };
            if !name.starts_with(prefix) || !name.ends_with(suffix) { continue; }
            let pid_str = &name[prefix.len()..name.len() - suffix.len()];
            let Ok(pid) = pid_str.parse::<u32>() else { continue; };
            if active_pids.contains(&pid) { continue; }
            if fs::remove_file(entry.path()).is_ok() {
                cleaned.push(pid);
            }
        }
        if !cleaned.is_empty() {
            log::info!("ClaudeHalo 清理 {} 个死会话状态文件", cleaned.len());
        }
        cleaned
    }
}

#[tauri::command]
pub async fn claudehalo_get_status() -> Vec<ClaudeHaloSession> {
    let pids = find_claude_pids();
    let _cleaned = cleanup_dead_sessions(&pids);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    let sessions: Vec<ClaudeHaloSession> = pids
        .into_iter()
        .map(|pid| {
            let (state, file) = read_state_for_pid(pid);
            let title = read_window_title(pid);
            ClaudeHaloSession {
                pid,
                state,
                state_file: file,
                title,
                detected_at: now,
            }
        })
        .collect();
    log::trace!("ClaudeHalo 探测: {} 个 claude.exe", sessions.len());
    sessions
}

/// 一键安装 ClaudeHalo hook 到 `~/.claude/settings.json`
///
/// 合并逻辑: 读用户现有 settings, 在 hooks 字段下注入 6 个事件
/// (SessionStart / UserPromptSubmit / PreToolUse / PostToolUse / Stop / Notification),
/// 每个事件指向 ClaudeHaloManage 的 halo-hook.ps1 + 传 -State 参数。
/// 已配置则幂等跳过, 不覆盖。
#[tauri::command]
pub async fn claudehalo_install_hook(
    hook_script_path: Option<String>,
) -> Result<InstallHookResult, String> {
    let default_script = r"D:\Work\Project\ClaudeHaloManage\win\scripts\halo-hook.ps1";
    let script = hook_script_path
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_script.to_string());

    if !PathBuf::from(&script).is_file() {
        return Err(format!("Hook 脚本不存在: {}", script));
    }

    let home = dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    let settings_path = home.join(".claude").join("settings.json");
    fs::create_dir_all(settings_path.parent().unwrap())
        .map_err(|e| format!("创建 .claude 目录失败: {}", e))?;

    let mut settings: serde_json::Value = if settings_path.is_file() {
        let text = fs::read_to_string(&settings_path).map_err(|e| format!("读 settings.json 失败: {}", e))?;
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let already = settings
        .get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .and_then(|s| s.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if already {
        return Ok(InstallHookResult {
            settings_path: settings_path.to_string_lossy().to_string(),
            already_configured: true,
            script_path: script,
        });
    }

    let ps_cmd = format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
        script.replace('\\', "\\\\")
    );

    let make_event = |state: &str| -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "hooks": [{
                "type": "command",
                "command": format!("{} -State {}", ps_cmd, state)
            }]
        })]
    };

    let mut hooks = std::collections::HashMap::new();
    hooks.insert("SessionStart",       make_event("idle"));
    hooks.insert("UserPromptSubmit",   make_event("thinking"));
    hooks.insert("PreToolUse",         make_event("working"));
    hooks.insert("PostToolUse",        make_event("waiting_input"));
    hooks.insert("Stop",               make_event("idle"));
    hooks.insert("Notification",       make_event("waiting_input"));

    let hooks_obj = serde_json::Value::Object(
        hooks.into_iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::Array(v)))
            .collect()
    );

    if let Some(obj) = settings.as_object_mut() {
        obj.insert("hooks".to_string(), hooks_obj);
    }

    let tmp = settings_path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&tmp, &text).map_err(|e| format!("写临时文件失败: {}", e))?;
    fs::rename(&tmp, &settings_path).map_err(|e| format!("rename 失败: {}", e))?;

    log::info!("ClaudeHalo hook 已安装到 {}", settings_path.display());
    Ok(InstallHookResult {
        settings_path: settings_path.to_string_lossy().to_string(),
        already_configured: false,
        script_path: script,
    })
}
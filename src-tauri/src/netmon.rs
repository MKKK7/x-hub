// 网速监视（Q1）
//
// 数据源：sysinfo::Networks —— 内部调 Windows iphlpapi!GetIfTable2，
// 跨进程通过全局 Mutex 复用同一 Networks 实例（避免每帧重新枚举适配器）。
//
// 后端只返回当前各接口的「累计字节数」绝对值；速率（KB/s）由前端 1s 差值计算。
// 理由：sysinfo 每次 refresh 也需 30-100ms，与前端 1s 采样错峰能在 idle 时更省 CPU。

use serde::Serialize;
use std::sync::Mutex;
use sysinfo::Networks;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetStats {
    pub interfaces: Vec<NetInterface>,
}

/// 全局 Networks 实例：复用避免重复枚举适配器（sysinfo::Networks::new_with_refreshed_list 不便宜）
static NETS: Mutex<Option<Networks>> = Mutex::new(None);

/// 读取各网络接口累计收发字节数
#[tauri::command]
pub fn get_net_stats() -> NetStats {
    let mut guard = NETS.lock().unwrap_or_else(|p| p.into_inner());
    let nets = guard.get_or_insert_with(Networks::new_with_refreshed_list);
    nets.refresh();

    let interfaces: Vec<NetInterface> = nets
        .iter()
        .map(|(name, data)| NetInterface {
            name: name.clone(),
            rx_bytes: data.received(),
            tx_bytes: data.transmitted(),
        })
        .collect();

    log::trace!("读取网络接口: {} 个", interfaces.len());
    NetStats { interfaces }
}
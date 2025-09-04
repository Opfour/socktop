//! Types that represent data from the socktop agent.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub mem_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkInfo {
    pub name: String,
    pub received: u64,
    pub transmitted: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GpuInfo {
    pub name: Option<String>,
    pub vendor: Option<String>,

    // Accept both the new and legacy keys
    #[serde(
        default,
        alias = "utilization_gpu_pct",
        alias = "gpu_util_pct",
        alias = "gpu_utilization"
    )]
    pub utilization: Option<f32>,

    #[serde(default, alias = "mem_used_bytes", alias = "vram_used_bytes")]
    pub mem_used: Option<u64>,

    #[serde(default, alias = "mem_total_bytes", alias = "vram_total_bytes")]
    pub mem_total: Option<u64>,

    #[serde(default, alias = "temp_c", alias = "temperature_c")]
    pub temp: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metrics {
    pub cpu_total: f32,
    pub cpu_per_core: Vec<f32>,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub hostname: String,
    pub cpu_temp_c: Option<f32>,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetworkInfo>,
    pub top_processes: Vec<ProcessInfo>,
    pub gpus: Option<Vec<GpuInfo>>,
    // New: keep the last reported total process count
    #[serde(default)]
    pub process_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessesPayload {
    pub process_count: usize,
    pub top_processes: Vec<ProcessInfo>,
}

/// Request types that can be sent to the agent
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AgentRequest {
    #[serde(rename = "metrics")]
    Metrics,
    #[serde(rename = "disks")]
    Disks,
    #[serde(rename = "processes")]
    Processes,
}

impl AgentRequest {
    /// Convert to the legacy string format used by the agent
    pub fn to_legacy_string(&self) -> String {
        match self {
            AgentRequest::Metrics => "get_metrics".to_string(),
            AgentRequest::Disks => "get_disks".to_string(),
            AgentRequest::Processes => "get_processes".to_string(),
        }
    }
}

/// Response types that can be received from the agent
#[derive(Debug, Clone)]
pub enum AgentResponse {
    Metrics(Metrics),
    Disks(Vec<DiskInfo>),
    Processes(ProcessesPayload),
}

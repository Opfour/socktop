//! WebSocket request handlers for native (non-WASM) environments.

use crate::networking::WsStream;
use crate::types::{JournalResponse, ProcessMetricsResponse};
use crate::utils::{gunzip_to_string, gunzip_to_vec, is_gzip};
use crate::{DiskInfo, Metrics, ProcessInfo, ProcessesPayload, pb};

use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use tokio_tungstenite::tungstenite::Message;

/// Send a "get_metrics" request and await a single JSON reply
pub async fn request_metrics(ws: &mut WsStream) -> Option<Metrics> {
    if ws.send(Message::Text("get_metrics".into())).await.is_err() {
        return None;
    }
    match ws.next().await {
        Some(Ok(Message::Binary(b))) => gunzip_to_string(&b)
            .ok()
            .and_then(|s| serde_json::from_str::<Metrics>(&s).ok()),
        Some(Ok(Message::Text(json))) => serde_json::from_str::<Metrics>(&json).ok(),
        _ => None,
    }
}

/// Send a "get_disks" request and await a JSON Vec<DiskInfo>
pub async fn request_disks(ws: &mut WsStream) -> Option<Vec<DiskInfo>> {
    if ws.send(Message::Text("get_disks".into())).await.is_err() {
        return None;
    }
    match ws.next().await {
        Some(Ok(Message::Binary(b))) => gunzip_to_string(&b)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<DiskInfo>>(&s).ok()),
        Some(Ok(Message::Text(json))) => serde_json::from_str::<Vec<DiskInfo>>(&json).ok(),
        _ => None,
    }
}

/// Send a "get_processes" request and await a ProcessesPayload decoded from protobuf (binary, may be gzipped)
pub async fn request_processes(ws: &mut WsStream) -> Option<ProcessesPayload> {
    if ws
        .send(Message::Text("get_processes".into()))
        .await
        .is_err()
    {
        return None;
    }
    match ws.next().await {
        Some(Ok(Message::Binary(b))) => {
            let gz = is_gzip(&b);
            let data = if gz { gunzip_to_vec(&b).ok()? } else { b };
            match pb::Processes::decode(data.as_slice()) {
                Ok(pb) => {
                    let rows: Vec<ProcessInfo> = pb
                        .rows
                        .into_iter()
                        .map(|p: pb::Process| ProcessInfo {
                            pid: p.pid,
                            name: p.name,
                            cpu_usage: p.cpu_usage,
                            mem_bytes: p.mem_bytes,
                        })
                        .collect();
                    Some(ProcessesPayload {
                        process_count: pb.process_count as usize,
                        top_processes: rows,
                    })
                }
                Err(e) => {
                    if std::env::var("SOCKTOP_DEBUG").ok().as_deref() == Some("1") {
                        eprintln!("protobuf decode failed: {e}");
                    }
                    // Fallback: maybe it's JSON (bytes already decompressed if gz)
                    match String::from_utf8(data) {
                        Ok(s) => serde_json::from_str::<ProcessesPayload>(&s).ok(),
                        Err(_) => None,
                    }
                }
            }
        }
        Some(Ok(Message::Text(json))) => serde_json::from_str::<ProcessesPayload>(&json).ok(),
        _ => None,
    }
}

/// Send a "get_process_metrics:{pid}" request and await a JSON ProcessMetricsResponse
pub async fn request_process_metrics(
    ws: &mut WsStream,
    pid: u32,
) -> Option<ProcessMetricsResponse> {
    let request = format!("get_process_metrics:{pid}");
    if ws.send(Message::Text(request)).await.is_err() {
        return None;
    }
    match ws.next().await {
        Some(Ok(Message::Binary(b))) => gunzip_to_string(&b)
            .ok()
            .and_then(|s| serde_json::from_str::<ProcessMetricsResponse>(&s).ok()),
        Some(Ok(Message::Text(json))) => serde_json::from_str::<ProcessMetricsResponse>(&json).ok(),
        _ => None,
    }
}

/// Send a "get_journal_entries:{pid}" request and await a JSON JournalResponse
pub async fn request_journal_entries(ws: &mut WsStream, pid: u32) -> Option<JournalResponse> {
    let request = format!("get_journal_entries:{pid}");
    if ws.send(Message::Text(request)).await.is_err() {
        return None;
    }
    match ws.next().await {
        Some(Ok(Message::Binary(b))) => gunzip_to_string(&b)
            .ok()
            .and_then(|s| serde_json::from_str::<JournalResponse>(&s).ok()),
        Some(Ok(Message::Text(json))) => serde_json::from_str::<JournalResponse>(&json).ok(),
        _ => None,
    }
}

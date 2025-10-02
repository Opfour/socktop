//! Caching for process metrics and journal entries

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::types::{ProcessMetricsResponse, JournalResponse};

#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    cached_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

#[derive(Debug)]
pub struct ProcessCache {
    process_metrics: RwLock<HashMap<u32, CacheEntry<ProcessMetricsResponse>>>,
    journal_entries: RwLock<HashMap<u32, CacheEntry<JournalResponse>>>,
}

impl ProcessCache {
    pub fn new() -> Self {
        Self {
            process_metrics: RwLock::new(HashMap::new()),
            journal_entries: RwLock::new(HashMap::new()),
        }
    }

    /// Get cached process metrics if available and not expired (250ms TTL)
    pub async fn get_process_metrics(&self, pid: u32) -> Option<ProcessMetricsResponse> {
        let cache = self.process_metrics.read().await;
        if let Some(entry) = cache.get(&pid) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Cache process metrics with 250ms TTL
    pub async fn set_process_metrics(&self, pid: u32, data: ProcessMetricsResponse) {
        let mut cache = self.process_metrics.write().await;
        cache.insert(pid, CacheEntry {
            data,
            cached_at: Instant::now(),
            ttl: Duration::from_millis(250),
        });
    }

    /// Get cached journal entries if available and not expired (1s TTL)
    pub async fn get_journal_entries(&self, pid: u32) -> Option<JournalResponse> {
        let cache = self.journal_entries.read().await;
        if let Some(entry) = cache.get(&pid) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Cache journal entries with 1s TTL
    pub async fn set_journal_entries(&self, pid: u32, data: JournalResponse) {
        let mut cache = self.journal_entries.write().await;
        cache.insert(pid, CacheEntry {
            data,
            cached_at: Instant::now(),
            ttl: Duration::from_secs(1),
        });
    }

    /// Clean up expired entries periodically
    pub async fn cleanup_expired(&self) {
        {
            let mut cache = self.process_metrics.write().await;
            cache.retain(|_, entry| !entry.is_expired());
        }
        {
            let mut cache = self.journal_entries.write().await;
            cache.retain(|_, entry| !entry.is_expired());
        }
    }
}

impl Default for ProcessCache {
    fn default() -> Self {
        Self::new()
    }
}
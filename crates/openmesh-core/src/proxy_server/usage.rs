use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_USAGE_LOGS: usize = 2_048;

#[derive(Clone, Default)]
pub(crate) struct ProxyUsageTracker {
    inner: Arc<Mutex<UsageInner>>,
}

pub trait ProxyUsageSink: Send + Sync {
    fn record(&self, log: &ProxyUsageLog);
}

struct UsageInner {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    logs: VecDeque<ProxyUsageLog>,
    sink: Option<Arc<dyn ProxyUsageSink>>,
}

impl Default for UsageInner {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            logs: VecDeque::new(),
            sink: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyUsageLog {
    pub request_id: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub upstream_id: Option<String>,
    pub account_id: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyUsageSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub logs: Vec<ProxyUsageLog>,
}

impl ProxyUsageTracker {
    pub(crate) fn with_sink(sink: Arc<dyn ProxyUsageSink>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(UsageInner {
                sink: Some(sink),
                ..UsageInner::default()
            })),
        }
    }

    pub(crate) fn record(
        &self,
        request_id: String,
        path: String,
        status: u16,
        latency_ms: u64,
        model: Option<String>,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        total_tokens: Option<u64>,
        upstream_id: Option<String>,
        account_id: Option<String>,
    ) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.total_requests = inner.total_requests.saturating_add(1);
        if (200..400).contains(&status) {
            inner.successful_requests = inner.successful_requests.saturating_add(1);
        } else {
            inner.failed_requests = inner.failed_requests.saturating_add(1);
        }
        let log = ProxyUsageLog {
            request_id,
            path,
            status,
            latency_ms,
            model,
            input_tokens,
            output_tokens,
            total_tokens,
            upstream_id,
            account_id,
            observed_at: chrono::Utc::now().to_rfc3339(),
        };
        inner.logs.push_back(log.clone());
        while inner.logs.len() > MAX_USAGE_LOGS {
            inner.logs.pop_front();
        }
        let sink = inner.sink.clone();
        drop(inner);
        if let Some(sink) = sink {
            sink.record(&log);
        }
    }

    pub(crate) fn snapshot(&self, limit: usize) -> ProxyUsageSnapshot {
        let Ok(inner) = self.inner.lock() else {
            return ProxyUsageSnapshot {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                logs: Vec::new(),
            };
        };
        let limit = limit.min(MAX_USAGE_LOGS);
        let logs = inner.logs.iter().rev().take(limit).cloned().collect();
        ProxyUsageSnapshot {
            total_requests: inner.total_requests,
            successful_requests: inner.successful_requests,
            failed_requests: inner.failed_requests,
            logs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct TestSink {
        records: Arc<Mutex<Vec<ProxyUsageLog>>>,
    }

    impl ProxyUsageSink for TestSink {
        fn record(&self, log: &ProxyUsageLog) {
            self.records.lock().expect("sink lock").push(log.clone());
        }
    }

    #[test]
    fn tracker_counts_success_and_failure_and_returns_newest_first() {
        let tracker = ProxyUsageTracker::default();
        tracker.record(
            "one".into(),
            "/v1/models".into(),
            200,
            3,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        tracker.record(
            "two".into(),
            "/v1/chat/completions".into(),
            502,
            8,
            Some("model".into()),
            Some(3),
            Some(4),
            Some(7),
            Some("account".into()),
            Some("acct-1".into()),
        );
        let snapshot = tracker.snapshot(10);
        assert_eq!(snapshot.total_requests, 2);
        assert_eq!(snapshot.successful_requests, 1);
        assert_eq!(snapshot.failed_requests, 1);
        assert_eq!(snapshot.logs[0].request_id, "two");
        assert_eq!(snapshot.logs[0].account_id.as_deref(), Some("acct-1"));
        assert_eq!(snapshot.logs[0].model.as_deref(), Some("model"));
        assert_eq!(snapshot.logs[0].total_tokens, Some(7));
    }

    #[test]
    fn tracker_forwards_records_to_optional_sink() {
        let sink = TestSink::default();
        let tracker = ProxyUsageTracker::with_sink(Arc::new(sink.clone()));
        tracker.record(
            "request".into(),
            "/v1/chat/completions".into(),
            200,
            12,
            Some("model".into()),
            Some(1),
            Some(2),
            Some(3),
            Some("upstream".into()),
            None,
        );
        let records = sink.records.lock().expect("sink lock");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].total_tokens, Some(3));
    }
}

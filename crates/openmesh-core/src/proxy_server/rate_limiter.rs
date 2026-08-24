use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct ProxyRateLimitTracker {
    inner: Arc<Mutex<HashMap<String, Instant>>>,
    cooldown: Duration,
}

impl Default for ProxyRateLimitTracker {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            cooldown: DEFAULT_COOLDOWN,
        }
    }
}

impl ProxyRateLimitTracker {
    pub(crate) fn mark_limited(&self, upstream_id: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.insert(upstream_id.to_string(), Instant::now());
        }
    }

    pub(crate) fn is_limited(&self, upstream_id: &str) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        let Some(marked_at) = inner.get(upstream_id).copied() else {
            return false;
        };
        if marked_at.elapsed() >= self.cooldown {
            inner.remove(upstream_id);
            false
        } else {
            true
        }
    }

    #[cfg(test)]
    pub(crate) fn with_cooldown(cooldown: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            cooldown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_expires_without_external_cleanup() {
        let tracker = ProxyRateLimitTracker::with_cooldown(Duration::from_millis(20));
        tracker.mark_limited("account-a");
        assert!(tracker.is_limited("account-a"));
        std::thread::sleep(Duration::from_millis(30));
        assert!(!tracker.is_limited("account-a"));
    }
}

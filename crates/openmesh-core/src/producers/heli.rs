//! Read-only Heli harness state reader (0.1.3.6 Checkpoint C).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::domain::{HeliProducerError, HeliProducerResult, HeliSnapshot, ProducerSkipReason};

const CURRENT_TASK_REL: &str = ".heli-harness/state/current-task.md";
const DECISIONS_REL: &str = ".heli-harness/state/decisions.md";
const REPORTS_REL: &str = ".heli-harness/state/reports";
const MAX_HELI_FILE_BYTES: u64 = 256 * 1024;
const MAX_HELI_DECISIONS_TAIL_BYTES: usize = 32 * 1024;
const MAX_HELI_EXCERPT_CHARS: usize = 4096;

/// Collect a bounded Heli harness snapshot from `workspace_path`.
///
/// Never writes Heli files. Missing `.heli-harness/` is a graceful skip, not an
/// error.
pub fn read_heli_snapshot(workspace_path: &Path) -> HeliProducerResult {
    let heli_root = workspace_path.join(".heli-harness");
    if !heli_root.is_dir() {
        return HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent);
    }

    let observed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut read_errors = Vec::new();

    let current_task_excerpt = match read_bounded_text(workspace_path, CURRENT_TASK_REL) {
        Ok(text) => Some(bound_excerpt(&text)),
        Err(HeliProducerError::ReadFailed(msg)) if msg.contains("not found") => None,
        Err(err) => {
            read_errors.push(err);
            None
        }
    };

    let decisions_tail_excerpt = match read_bounded_text(workspace_path, DECISIONS_REL) {
        Ok(text) => Some(bound_tail_excerpt(&text, MAX_HELI_DECISIONS_TAIL_BYTES)),
        Err(HeliProducerError::ReadFailed(msg)) if msg.contains("not found") => None,
        Err(err) => {
            read_errors.push(err);
            None
        }
    };

    let latest_report_path = find_latest_report_path(workspace_path);

    if current_task_excerpt.is_none()
        && decisions_tail_excerpt.is_none()
        && latest_report_path.is_none()
    {
        if let Some(err) = read_errors.into_iter().next() {
            return HeliProducerResult::Err(err);
        }
        return HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent);
    }

    HeliProducerResult::Snapshot(HeliSnapshot {
        current_task_excerpt,
        decisions_tail_excerpt,
        latest_report_path,
        observed_at,
    })
}

fn read_bounded_text(workspace_path: &Path, rel: &str) -> Result<String, HeliProducerError> {
    let path = workspace_path.join(rel);
    if !path.is_file() {
        return Err(HeliProducerError::ReadFailed(format!("not found: {rel}")));
    }
    let metadata = fs::metadata(&path).map_err(|e| io_error(rel, e))?;
    if metadata.len() > MAX_HELI_FILE_BYTES {
        return Err(HeliProducerError::ReadFailed(format!(
            "{rel} exceeds {MAX_HELI_FILE_BYTES} bytes"
        )));
    }
    let mut file = fs::File::open(&path).map_err(|e| io_error(rel, e))?;
    let mut buf = vec![0u8; (MAX_HELI_FILE_BYTES as usize) + 1];
    let n = file.read(&mut buf).map_err(|e| io_error(rel, e))?;
    if n > MAX_HELI_FILE_BYTES as usize {
        return Err(HeliProducerError::ReadFailed(format!(
            "{rel} exceeds {MAX_HELI_FILE_BYTES} bytes"
        )));
    }
    buf.truncate(n);
    String::from_utf8(buf)
        .map_err(|_| HeliProducerError::ReadFailed(format!("{rel}: invalid utf-8")))
}

fn find_latest_report_path(workspace_path: &Path) -> Option<String> {
    let reports_dir = workspace_path.join(REPORTS_REL);
    if !reports_dir.is_dir() {
        return None;
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = fs::read_dir(&reports_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name()?.to_string_lossy();
        if !name.starts_with("openmesh-") || !name.ends_with(".md") {
            continue;
        }
        let mtime = entry.metadata().ok()?.modified().ok()?;
        match &best {
            Some((best_mtime, _)) if mtime <= *best_mtime => {}
            _ => best = Some((mtime, path)),
        }
    }
    best.and_then(|(_, path)| {
        path.strip_prefix(workspace_path)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
    })
}

fn bound_excerpt(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= MAX_HELI_EXCERPT_CHARS {
        return trimmed.to_string();
    }
    trimmed.chars().take(MAX_HELI_EXCERPT_CHARS).collect()
}

fn bound_tail_excerpt(text: &str, max_bytes: usize) -> String {
    let bytes = text.as_bytes();
    if bytes.len() <= max_bytes {
        return text.trim().to_string();
    }
    let tail = &bytes[bytes.len() - max_bytes..];
    let lossy = String::from_utf8_lossy(tail);
    bound_excerpt(lossy.trim())
}

fn io_error(rel: &str, err: std::io::Error) -> HeliProducerError {
    HeliProducerError::ReadFailed(format!("{rel}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_workspace(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-heli-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    #[test]
    fn heli_absent_when_harness_missing() {
        let ws = temp_workspace("absent");
        assert!(matches!(
            read_heli_snapshot(&ws),
            HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent)
        ));
    }

    #[test]
    fn heli_reads_current_task_and_decisions() {
        let ws = temp_workspace("present");
        let state = ws.join(".heli-harness/state");
        fs::create_dir_all(&state).unwrap();
        fs::write(
            state.join("current-task.md"),
            "# Dev Track 0.1.3.6\n\nActive checkpoint C\n",
        )
        .unwrap();
        fs::write(
            state.join("decisions.md"),
            "## 2026-07-16\n\nDecision entry\n",
        )
        .unwrap();

        let snapshot = match read_heli_snapshot(&ws) {
            HeliProducerResult::Snapshot(s) => s,
            other => panic!("expected snapshot, got {other:?}"),
        };
        assert!(snapshot
            .current_task_excerpt
            .as_deref()
            .unwrap()
            .contains("0.1.3.6"));
        assert!(snapshot
            .decisions_tail_excerpt
            .as_deref()
            .unwrap()
            .contains("Decision entry"));
    }

    #[test]
    fn heli_selects_latest_openmesh_report_by_mtime() {
        let ws = temp_workspace("reports");
        let reports = ws.join(REPORTS_REL);
        fs::create_dir_all(&reports).unwrap();
        fs::write(reports.join("openmesh-old.md"), "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(reports.join("openmesh-new.md"), "new").unwrap();

        let snapshot = match read_heli_snapshot(&ws) {
            HeliProducerResult::Snapshot(s) => s,
            other => panic!("expected snapshot, got {other:?}"),
        };
        assert_eq!(
            snapshot.latest_report_path.as_deref(),
            Some(".heli-harness/state/reports/openmesh-new.md")
        );
    }

    #[test]
    fn heli_tail_excerpt_is_bounded() {
        let huge = "x".repeat(40_000);
        let excerpt = bound_tail_excerpt(&huge, 1024);
        assert!(excerpt.len() <= MAX_HELI_EXCERPT_CHARS);
    }
}

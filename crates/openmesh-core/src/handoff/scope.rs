//! Dev Track 0.1.8 Checkpoint B — handoff recipient and catch-up window helpers (pure).

use crate::domain::CatchUpWindow;
use crate::handoff::contract::{
    validate_recipient_fields, HandoffRecipient, HandoffValidationError,
};

const DEFAULT_WINDOW_DAYS: i64 = 7;

/// Builds a validated handoff recipient from label and optional role.
pub fn build_handoff_recipient(
    label: &str,
    role_label: Option<&str>,
) -> Result<HandoffRecipient, HandoffValidationError> {
    let recipient = HandoffRecipient {
        label: label.to_string(),
        role_label: role_label.map(str::to_string),
    };
    validate_recipient_fields(&recipient)?;
    Ok(recipient)
}

/// Resolves a catch-up window for handoff generation.
///
/// When both `since` and `until` are omitted, defaults to the last seven days ending at `now`.
/// When only one bound is provided, fails closed.
pub fn resolve_handoff_window(
    since: Option<&str>,
    until: Option<&str>,
    now: &str,
) -> Result<CatchUpWindow, HandoffValidationError> {
    match (since, until) {
        (None, None) => default_window(now),
        (Some(since), Some(until)) => Ok(CatchUpWindow {
            since: since.to_string(),
            until: until.to_string(),
        }),
        _ => Err(HandoffValidationError::InvalidWindow(
            "since and until must both be provided".into(),
        )),
    }
    .and_then(validate_resolved_window)
}

fn format_utc_rfc3339(dt: chrono::DateTime<chrono::FixedOffset>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn default_window(now: &str) -> Result<CatchUpWindow, HandoffValidationError> {
    let end = chrono::DateTime::parse_from_rfc3339(now)
        .map_err(|err| HandoffValidationError::InvalidTimestamp(err.to_string()))?;
    let start = end - chrono::Duration::days(DEFAULT_WINDOW_DAYS);
    Ok(CatchUpWindow {
        since: format_utc_rfc3339(start),
        until: format_utc_rfc3339(end),
    })
}

fn validate_resolved_window(
    window: CatchUpWindow,
) -> Result<CatchUpWindow, HandoffValidationError> {
    crate::handoff::contract::validate_window_fields(&window)?;
    Ok(window)
}

//! Dev Track 0.1.8 Checkpoint E — deterministic markdown projection for handoff notes.

use crate::handoff::contract::{HandoffNote, HandoffSection, HandoffSectionItem, HandoffStatus};

/// Renders a deterministic markdown document for a handoff note.
pub fn render_handoff_markdown(note: &HandoffNote) -> String {
    let mut out = String::new();
    out.push_str("# Handoff Note\n\n");
    out.push_str(&format!("**Handoff ID:** {}\n", note.handoff_id));
    out.push_str(&format!("**Workspace:** {}\n", note.workspace_id));
    out.push_str(&format!("**Status:** {}\n", status_label(note.status)));
    out.push_str(&format!(
        "**Recipient:** {}\n",
        recipient_line(&note.recipient)
    ));
    out.push_str(&format!(
        "**Window:** {} → {}\n",
        note.window.since, note.window.until
    ));
    out.push_str(&format!(
        "**Freshness:** generated_at={} age_seconds={}\n",
        note.freshness.generated_at, note.freshness.age_seconds
    ));
    if !note.freshness.warnings.is_empty() {
        out.push_str("**Freshness warnings:**\n");
        for warning in &note.freshness.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }
    out.push('\n');

    render_section(&mut out, "What Changed", &note.what_changed);
    render_section(&mut out, "What Is Complete", &note.what_is_complete);
    render_section(&mut out, "What Is Blocked", &note.what_is_blocked);
    render_section(&mut out, "What Needs Review", &note.what_needs_review);
    render_section(&mut out, "Open Questions", &note.open_questions);
    render_section(
        &mut out,
        "Safe To Answer Context",
        &note.safe_to_answer_context,
    );
    render_section(&mut out, "Next Suggested Step", &note.next_suggested_step);

    if !note.limitations.is_empty() {
        out.push_str("## Limitations\n");
        for limitation in &note.limitations {
            out.push_str(&format!("- {limitation}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Metadata\n");
    out.push_str(&format!("- created_at: {}\n", note.created_at));
    out.push_str(&format!("- updated_at: {}\n", note.updated_at));
    if let Some(approved_at) = &note.approved_at {
        out.push_str(&format!("- approved_at: {approved_at}\n"));
    }
    if let Some(work_event_id) = &note.work_event_id {
        out.push_str(&format!("- work_event_id: {work_event_id}\n"));
    }

    out
}

fn status_label(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Draft => "draft",
        HandoffStatus::Approved => "approved",
    }
}

fn recipient_line(recipient: &crate::handoff::contract::HandoffRecipient) -> String {
    match &recipient.role_label {
        Some(role) => format!("{} ({role})", recipient.label),
        None => recipient.label.clone(),
    }
}

fn render_section(out: &mut String, title: &str, section: &HandoffSection) {
    out.push_str(&format!("## {title}\n"));
    if section.items.is_empty() {
        out.push_str("- (none)\n\n");
        return;
    }
    for item in &section.items {
        render_item(out, item);
    }
    out.push('\n');
}

fn render_item(out: &mut String, item: &HandoffSectionItem) {
    out.push_str(&format!("- {}\n", item.summary));
    if !item.evidence_refs.is_empty() {
        let refs = item
            .evidence_refs
            .iter()
            .map(evidence_ref_label)
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  - evidence: {refs}\n"));
    }
    if !item.source_event_ids.is_empty() {
        out.push_str(&format!(
            "  - source_event_ids: {}\n",
            item.source_event_ids.join(", ")
        ));
    }
}

fn evidence_ref_label(evidence: &crate::domain::EvidenceRef) -> String {
    match evidence {
        crate::domain::EvidenceRef::FilePath(path) => format!("file:{path}"),
        crate::domain::EvidenceRef::ProducerSignal(signal_id) => format!("signal:{signal_id}"),
        crate::domain::EvidenceRef::GitState(state) => format!("git:{}", state.repo_id),
    }
}

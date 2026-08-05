use serde::{Deserialize, Serialize};

pub const CANVAS_SCHEMA: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasNode {
    pub id: String,
    pub label: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub x: f64,
    pub y: f64,
}

fn default_kind() -> String {
    "default".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasEdge {
    pub id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasRevision {
    pub rev: u32,
    pub at: u64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasDocument {
    pub id: String,
    pub title: String,
    pub schema_version: String,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
    pub revisions: Vec<CanvasRevision>,
    pub updated_at: u64,
}

impl CanvasDocument {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = now_ms();
        Self {
            id: id.into(),
            title: title.into(),
            schema_version: CANVAS_SCHEMA.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            revisions: vec![CanvasRevision {
                rev: 1,
                at: now,
                summary: "created".into(),
            }],
            updated_at: now,
        }
    }

    pub fn bump(&mut self, summary: impl Into<String>) {
        let rev = self.revisions.last().map(|r| r.rev + 1).unwrap_or(1);
        let at = now_ms();
        self.revisions.push(CanvasRevision {
            rev,
            at,
            summary: summary.into(),
        });
        if self.revisions.len() > 50 {
            let drain = self.revisions.len() - 50;
            self.revisions.drain(0..drain);
        }
        self.updated_at = at;
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let doc = CanvasDocument::new("c1", "Network");
        let json = serde_json::to_string(&doc).unwrap();
        let back: CanvasDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "c1");
        assert_eq!(back.schema_version, CANVAS_SCHEMA);
    }
}

use super::model::{CanvasDocument, CanvasEdge, CanvasNode};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    #[error("io: {0}")]
    Io(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

fn canvases_dir(project_path: &Path) -> PathBuf {
    project_path.join(".openmesh").join("canvases")
}

fn canvas_file(project_path: &Path, id: &str) -> PathBuf {
    canvases_dir(project_path).join(format!("{id}.json"))
}

pub fn list_canvases(project_path: &Path) -> Result<Vec<CanvasDocument>, CanvasError> {
    let dir = canvases_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| CanvasError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| CanvasError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(doc) = load_canvas_path(&path) {
            out.push(doc);
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

pub fn load_canvas(project_path: &Path, id: &str) -> Result<CanvasDocument, CanvasError> {
    load_canvas_path(&canvas_file(project_path, id))
}

fn load_canvas_path(path: &Path) -> Result<CanvasDocument, CanvasError> {
    let raw = fs::read_to_string(path).map_err(|e| CanvasError::NotFound(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| CanvasError::Invalid(e.to_string()))
}

pub fn save_canvas(project_path: &Path, doc: &CanvasDocument) -> Result<(), CanvasError> {
    let dir = canvases_dir(project_path);
    fs::create_dir_all(&dir).map_err(|e| CanvasError::Io(e.to_string()))?;
    let path = canvas_file(project_path, &doc.id);
    let raw = serde_json::to_string_pretty(doc).map_err(|e| CanvasError::Invalid(e.to_string()))?;
    fs::write(path, raw).map_err(|e| CanvasError::Io(e.to_string()))
}

pub fn create_canvas(
    project_path: &Path,
    title: impl Into<String>,
) -> Result<CanvasDocument, CanvasError> {
    let id = format!("canvas-{}", now_ms());
    let doc = CanvasDocument::new(id, title);
    save_canvas(project_path, &doc)?;
    Ok(doc)
}

pub fn add_node(
    project_path: &Path,
    canvas_id: &str,
    label: impl Into<String>,
    kind: Option<String>,
) -> Result<CanvasDocument, CanvasError> {
    let mut doc = load_canvas(project_path, canvas_id)?;
    let n = doc.nodes.len() as f64;
    let node = CanvasNode {
        id: format!("n-{}", now_ms()),
        label: label.into(),
        kind: kind.unwrap_or_else(|| "machine".into()),
        x: 80.0 + (n % 4.0) * 180.0,
        y: 80.0 + (n / 4.0).floor() * 120.0,
    };
    let label_clone = node.label.clone();
    doc.nodes.push(node);
    doc.bump(format!("add node {label_clone}"));
    save_canvas(project_path, &doc)?;
    Ok(doc)
}

pub fn connect_nodes(
    project_path: &Path,
    canvas_id: &str,
    from: &str,
    to: &str,
) -> Result<CanvasDocument, CanvasError> {
    let mut doc = load_canvas(project_path, canvas_id)?;
    if !doc.nodes.iter().any(|n| n.id == from) || !doc.nodes.iter().any(|n| n.id == to) {
        return Err(CanvasError::Invalid("unknown node id".into()));
    }
    if doc
        .edges
        .iter()
        .any(|e| e.from == from && e.to == to)
    {
        return Ok(doc);
    }
    doc.edges.push(CanvasEdge {
        id: format!("e-{}", now_ms()),
        from: from.into(),
        to: to.into(),
    });
    doc.bump(format!("connect {from} → {to}"));
    save_canvas(project_path, &doc)?;
    Ok(doc)
}

pub fn delete_node(
    project_path: &Path,
    canvas_id: &str,
    node_id: &str,
) -> Result<CanvasDocument, CanvasError> {
    let mut doc = load_canvas(project_path, canvas_id)?;
    let before = doc.nodes.len();
    doc.nodes.retain(|n| n.id != node_id);
    if doc.nodes.len() == before {
        return Err(CanvasError::NotFound(node_id.into()));
    }
    doc.edges
        .retain(|e| e.from != node_id && e.to != node_id);
    doc.bump(format!("delete node {node_id}"));
    save_canvas(project_path, &doc)?;
    Ok(doc)
}

/// Viewport hint for the UI (not persisted).
pub fn fit_hint(doc: &CanvasDocument) -> (f64, f64, f64, f64) {
    if doc.nodes.is_empty() {
        return (0.0, 0.0, 800.0, 600.0);
    }
    let min_x = doc.nodes.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
    let min_y = doc.nodes.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
    let max_x = doc
        .nodes
        .iter()
        .map(|n| n.x + 140.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = doc
        .nodes
        .iter()
        .map(|n| n.y + 64.0)
        .fold(f64::NEG_INFINITY, f64::max);
    (min_x - 40.0, min_y - 40.0, max_x + 40.0, max_y + 40.0)
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
    use tempfile::tempdir;

    #[test]
    fn create_add_connect() {
        let dir = tempdir().unwrap();
        let doc = create_canvas(dir.path(), "Net").unwrap();
        let doc = add_node(dir.path(), &doc.id, "A", Some("machine".into())).unwrap();
        let a = doc.nodes[0].id.clone();
        let doc = add_node(dir.path(), &doc.id, "B", Some("machine".into())).unwrap();
        let b = doc.nodes[1].id.clone();
        let doc = connect_nodes(dir.path(), &doc.id, &a, &b).unwrap();
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(list_canvases(dir.path()).unwrap().len(), 1);
    }
}

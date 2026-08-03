//! OpenMesh Agent Extensions — skills, hooks, plugins (local MVP).
//!
//! Skills: markdown packs with YAML-ish frontmatter injected into the system prompt.
//! Hooks: declarative lifecycle context appenders (no remote code).
//! Plugins: folder + `openmesh.plugin.json` contributing skills and/or hooks.
//!
//! ponytail: local folders + settings toggles only; remote marketplace is v2.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// User-level extensions root: `$XDG_CONFIG_HOME/openmesh` or macOS Application Support.
pub fn user_extensions_root() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openmesh")
}

pub fn user_skills_dir() -> PathBuf {
    user_extensions_root().join("skills")
}

pub fn user_plugins_dir() -> PathBuf {
    user_extensions_root().join("plugins")
}

pub fn project_skills_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(".openmesh").join("skills")
}

pub fn project_plugins_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(".openmesh").join("plugins")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionSource {
    Builtin,
    User,
    Project,
    Plugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    OnChatStart,
    OnBeforeTurn,
    OnAfterTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub body: String,
    pub enabled: bool,
    pub source: ExtensionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookDefinition {
    pub id: String,
    pub event: HookEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_context: Option<String>,
    /// Reserved — shell hooks are not executed in this MVP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub enabled: bool,
    pub source: ExtensionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub source: ExtensionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub hook_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsInventory {
    pub skills: Vec<SkillPack>,
    pub hooks: Vec<HookDefinition>,
    pub plugins: Vec<PluginRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsSettings {
    /// Explicit enable map. Missing key ⇒ enabled (opt-out).
    #[serde(default)]
    pub skills: HashMap<String, bool>,
    #[serde(default)]
    pub hooks: HashMap<String, bool>,
    #[serde(default)]
    pub plugins: HashMap<String, bool>,
}

impl ExtensionsSettings {
    pub fn is_enabled(&self, map: &HashMap<String, bool>, id: &str) -> bool {
        map.get(id).copied().unwrap_or(true)
    }

    pub fn set_skill(&mut self, id: &str, enabled: bool) {
        self.skills.insert(id.to_string(), enabled);
    }

    pub fn set_hook(&mut self, id: &str, enabled: bool) {
        self.hooks.insert(id.to_string(), enabled);
    }

    pub fn set_plugin(&mut self, id: &str, enabled: bool) {
        self.plugins.insert(id.to_string(), enabled);
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginManifestFile {
    id: String,
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    hooks: Vec<PluginHookFile>,
}

fn default_version() -> String {
    "0.1.0".into()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginHookFile {
    id: String,
    event: String,
    #[serde(default)]
    append_context: Option<String>,
    #[serde(default)]
    command: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("io: {0}")]
    Io(String),
    #[error("invalid skill: {0}")]
    InvalidSkill(String),
    #[error("invalid plugin: {0}")]
    InvalidPlugin(String),
}

/// Built-in catalog skills (always available; enable state from settings).
pub fn builtin_skills() -> Vec<SkillPack> {
    vec![
        SkillPack {
            id: "openmesh-voice".into(),
            name: "OpenMesh Voice".into(),
            description: "Speak in clear OpenMesh workbench language — local-first, continuity-aware."
                .into(),
            body: BUILTIN_OPENMESH_VOICE.into(),
            enabled: true,
            source: ExtensionSource::Builtin,
            plugin_id: None,
            path: None,
        },
        SkillPack {
            id: "concise-replies".into(),
            name: "Concise Replies".into(),
            description: "Keep answers short and actionable unless the user asks for depth.".into(),
            body: BUILTIN_CONCISE.into(),
            enabled: true,
            source: ExtensionSource::Builtin,
            plugin_id: None,
            path: None,
        },
        SkillPack {
            id: "continuity-first".into(),
            name: "Continuity First".into(),
            description: "Prefer Continuity / Current State tools before inventing project status."
                .into(),
            body: BUILTIN_CONTINUITY.into(),
            enabled: true,
            source: ExtensionSource::Builtin,
            plugin_id: None,
            path: None,
        },
    ]
}

pub fn builtin_hooks() -> Vec<HookDefinition> {
    vec![HookDefinition {
        id: "turn-focus".into(),
        event: HookEvent::OnBeforeTurn,
        append_context: Some(
            "Stay grounded in the open project. Prefer tools over speculation.".into(),
        ),
        command: None,
        enabled: true,
        source: ExtensionSource::Builtin,
        plugin_id: None,
    }]
}

const BUILTIN_OPENMESH_VOICE: &str = r#"# OpenMesh Voice

When helping in OpenMesh Agent Workbench:
- Prefer local project facts from tools over generic advice.
- Use OpenMesh terms: Continuity, Current State, mesh, Work Proxy drafts, handoffs.
- Never invent mesh/team/trust state. If unsure, say what tool would confirm it.
- Do not ask for or echo API keys."#;

const BUILTIN_CONCISE: &str = r#"# Concise Replies

Default to short answers: one clear verdict, then bullets only if needed.
Expand only when the user asks for detail, a plan, or a walkthrough."#;

const BUILTIN_CONTINUITY: &str = r#"# Continuity First

Before answering about project progress, blockers, or "where we left off":
1. Prefer Continuity / Current State / pending tools when available.
2. Cite what the tools returned; do not fabricate sprint or mesh facts.
3. If Continuity data is missing, say so and suggest rebuilding Current State."#;

/// Parse `SKILL.md` with optional `---` frontmatter (`name`, `description`).
pub fn parse_skill_markdown(id: &str, raw: &str) -> Result<SkillPack, ExtensionError> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let (name, description, body) = if trimmed.starts_with("---") {
        let rest = &trimmed[3..];
        let end = rest
            .find("\n---")
            .ok_or_else(|| ExtensionError::InvalidSkill("unclosed frontmatter".into()))?;
        let fm = &rest[..end];
        let body = rest[end + 4..].trim_start_matches('\n').to_string();
        let mut name = id.to_string();
        let mut description = String::new();
        for line in fm.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name:") {
                name = unquote(v.trim());
            } else if let Some(v) = line.strip_prefix("description:") {
                description = unquote(v.trim());
            }
        }
        (name, description, body)
    } else {
        (id.to_string(), String::new(), trimmed.to_string())
    };

    if body.trim().is_empty() {
        return Err(ExtensionError::InvalidSkill("empty body".into()));
    }

    Ok(SkillPack {
        id: id.to_string(),
        name,
        description,
        body,
        enabled: true,
        source: ExtensionSource::User,
        plugin_id: None,
        path: None,
    })
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_plugin_manifest(raw: &str) -> Result<PluginManifestFile, ExtensionError> {
    serde_json::from_str(raw).map_err(|e| ExtensionError::InvalidPlugin(e.to_string()))
}

fn parse_hook_event(s: &str) -> Option<HookEvent> {
    match s.trim() {
        "on_chat_start" | "onChatStart" => Some(HookEvent::OnChatStart),
        "on_before_turn" | "onBeforeTurn" => Some(HookEvent::OnBeforeTurn),
        "on_after_turn" | "onAfterTurn" => Some(HookEvent::OnAfterTurn),
        _ => None,
    }
}

fn load_skill_dir(
    dir: &Path,
    source: ExtensionSource,
    plugin_id: Option<&str>,
) -> Result<Option<SkillPack>, ExtensionError> {
    let skill_md = dir.join("SKILL.md");
    if !skill_md.is_file() {
        return Ok(None);
    }
    let id = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("skill")
        .to_string();
    let raw = fs::read_to_string(&skill_md).map_err(|e| ExtensionError::Io(e.to_string()))?;
    let mut skill = parse_skill_markdown(&id, &raw)?;
    skill.source = source;
    skill.plugin_id = plugin_id.map(|s| s.to_string());
    skill.path = Some(dir.display().to_string());
    Ok(Some(skill))
}

fn scan_skills_root(root: &Path, source: ExtensionSource) -> Vec<SkillPack> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(Some(skill)) = load_skill_dir(&path, source, None) {
            out.push(skill);
        }
    }
    out
}

fn load_plugin_dir(dir: &Path, source: ExtensionSource) -> Result<Option<(PluginRecord, Vec<SkillPack>, Vec<HookDefinition>)>, ExtensionError> {
    let manifest_path = dir.join("openmesh.plugin.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&manifest_path).map_err(|e| ExtensionError::Io(e.to_string()))?;
    let manifest = parse_plugin_manifest(&raw)?;

    let mut skills = Vec::new();
    for rel in &manifest.skills {
        let skill_path = dir.join(rel);
        if let Some(skill) = load_skill_dir(&skill_path, ExtensionSource::Plugin, Some(&manifest.id))?
        {
            skills.push(skill);
        }
    }

    let mut hooks = Vec::new();
    for h in &manifest.hooks {
        let Some(event) = parse_hook_event(&h.event) else {
            continue;
        };
        hooks.push(HookDefinition {
            id: format!("{}::{}", manifest.id, h.id),
            event,
            append_context: h.append_context.clone(),
            command: h.command.clone(),
            enabled: true,
            source: ExtensionSource::Plugin,
            plugin_id: Some(manifest.id.clone()),
        });
    }

    let record = PluginRecord {
        id: manifest.id.clone(),
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        enabled: true,
        source,
        path: Some(dir.display().to_string()),
        skill_ids: skills.iter().map(|s| s.id.clone()).collect(),
        hook_ids: hooks.iter().map(|h| h.id.clone()).collect(),
    };
    Ok(Some((record, skills, hooks)))
}

fn scan_plugins_root(root: &Path, source: ExtensionSource) -> (Vec<PluginRecord>, Vec<SkillPack>, Vec<HookDefinition>) {
    let mut plugins = Vec::new();
    let mut skills = Vec::new();
    let mut hooks = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return (plugins, skills, hooks);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(Some((plugin, s, h))) = load_plugin_dir(&path, source) {
            plugins.push(plugin);
            skills.extend(s);
            hooks.extend(h);
        }
    }
    (plugins, skills, hooks)
}

/// Load inventory from builtins + user + project paths. Applies enable flags from settings.
pub fn load_inventory(
    project_path: Option<&str>,
    settings: &ExtensionsSettings,
) -> ExtensionsInventory {
    let mut skills = builtin_skills();
    let mut hooks = builtin_hooks();
    let mut plugins = Vec::new();

    skills.extend(scan_skills_root(&user_skills_dir(), ExtensionSource::User));
    let (up, us, uh) = scan_plugins_root(&user_plugins_dir(), ExtensionSource::User);
    plugins.extend(up);
    skills.extend(us);
    hooks.extend(uh);

    if let Some(pp) = project_path {
        if !pp.trim().is_empty() {
            skills.extend(scan_skills_root(
                &project_skills_dir(pp),
                ExtensionSource::Project,
            ));
            let (pp_plugins, ps, ph) =
                scan_plugins_root(&project_plugins_dir(pp), ExtensionSource::Project);
            plugins.extend(pp_plugins);
            skills.extend(ps);
            hooks.extend(ph);
        }
    }

    // Dedupe skills by id (first wins: builtin > earlier scan order).
    let mut seen_skills = HashSet::new();
    skills.retain(|s| seen_skills.insert(s.id.clone()));
    let mut seen_hooks = HashSet::new();
    hooks.retain(|h| seen_hooks.insert(h.id.clone()));
    let mut seen_plugins = HashSet::new();
    plugins.retain(|p| seen_plugins.insert(p.id.clone()));

    for p in &mut plugins {
        p.enabled = settings.is_enabled(&settings.plugins, &p.id);
    }
    let disabled_plugins: HashSet<String> = plugins
        .iter()
        .filter(|p| !p.enabled)
        .map(|p| p.id.clone())
        .collect();

    for s in &mut skills {
        let plugin_ok = s
            .plugin_id
            .as_ref()
            .map(|pid| !disabled_plugins.contains(pid))
            .unwrap_or(true);
        s.enabled = plugin_ok && settings.is_enabled(&settings.skills, &s.id);
    }
    for h in &mut hooks {
        let plugin_ok = h
            .plugin_id
            .as_ref()
            .map(|pid| !disabled_plugins.contains(pid))
            .unwrap_or(true);
        h.enabled = plugin_ok && settings.is_enabled(&settings.hooks, &h.id);
    }

    ExtensionsInventory {
        skills,
        hooks,
        plugins,
    }
}

/// Build the markdown section injected into the Agent Engine system prompt.
pub fn build_skills_prompt_section(skills: &[SkillPack]) -> String {
    let enabled: Vec<_> = skills.iter().filter(|s| s.enabled).collect();
    if enabled.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n## Enabled OpenMesh Skills\n");
    out.push_str(
        "Follow these skill packs when relevant. They are user-enabled extensions.\n",
    );
    for s in enabled {
        out.push_str("\n### ");
        out.push_str(&s.name);
        out.push_str(" (`");
        out.push_str(&s.id);
        out.push_str("`)\n");
        if !s.description.is_empty() {
            out.push_str(&s.description);
            out.push('\n');
        }
        out.push_str(&s.body);
        out.push('\n');
    }
    out
}

/// Collect declarative hook context for a lifecycle event (enabled only; ignores shell).
pub fn collect_hook_context(hooks: &[HookDefinition], event: HookEvent) -> String {
    let mut parts = Vec::new();
    for h in hooks.iter().filter(|h| h.enabled && h.event == event) {
        if let Some(ctx) = h.append_context.as_ref().filter(|c| !c.trim().is_empty()) {
            parts.push(format!("- [{}] {}", h.id, ctx.trim()));
        }
        // ponytail: shell `command` ignored until Tools allowlist is wired for hooks
    }
    if parts.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n## Hook Context\n");
    out.push_str(&parts.join("\n"));
    out.push('\n');
    out
}

/// Enrich a base system prompt with enabled skills + before-turn / chat-start hooks.
pub fn enrich_system_prompt(
    base: &str,
    inventory: &ExtensionsInventory,
    is_new_chat: bool,
) -> String {
    let mut prompt = base.to_string();
    prompt.push_str(&build_skills_prompt_section(&inventory.skills));
    if is_new_chat {
        prompt.push_str(&collect_hook_context(
            &inventory.hooks,
            HookEvent::OnChatStart,
        ));
    }
    prompt.push_str(&collect_hook_context(
        &inventory.hooks,
        HookEvent::OnBeforeTurn,
    ));
    prompt
}

/// Copy a skill or plugin folder into the user extensions directory.
pub fn install_from_path(source: &Path) -> Result<String, ExtensionError> {
    if !source.is_dir() {
        return Err(ExtensionError::Io(
            "source must be a folder (zip import: unzip first — remote/zip is v2)".into(),
        ));
    }

    let skill_md = source.join("SKILL.md");
    let plugin_manifest = source.join("openmesh.plugin.json");

    if plugin_manifest.is_file() {
        let raw =
            fs::read_to_string(&plugin_manifest).map_err(|e| ExtensionError::Io(e.to_string()))?;
        let manifest = parse_plugin_manifest(&raw)?;
        let dest = user_plugins_dir().join(&manifest.id);
        copy_dir_recursive(source, &dest)?;
        return Ok(format!("plugin:{}", manifest.id));
    }

    if skill_md.is_file() {
        let id = source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ExtensionError::InvalidSkill("bad folder name".into()))?
            .to_string();
        let raw = fs::read_to_string(&skill_md).map_err(|e| ExtensionError::Io(e.to_string()))?;
        let _ = parse_skill_markdown(&id, &raw)?;
        let dest = user_skills_dir().join(&id);
        copy_dir_recursive(source, &dest)?;
        return Ok(format!("skill:{id}"));
    }

    Err(ExtensionError::InvalidPlugin(
        "folder needs SKILL.md or openmesh.plugin.json".into(),
    ))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ExtensionError> {
    fs::create_dir_all(dst).map_err(|e| ExtensionError::Io(e.to_string()))?;
    for entry in fs::read_dir(src).map_err(|e| ExtensionError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| ExtensionError::Io(e.to_string()))?;
        let ty = entry
            .file_type()
            .map_err(|e| ExtensionError::Io(e.to_string()))?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &to).map_err(|e| ExtensionError::Io(e.to_string()))?;
        }
    }
    Ok(())
}

/// Catalog entries for the local Browse UI (builtins + short blurbs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub installed: bool,
}

pub fn local_catalog(inventory: &ExtensionsInventory) -> Vec<CatalogEntry> {
    let installed_skills: HashSet<_> = inventory.skills.iter().map(|s| s.id.clone()).collect();
    let mut entries: Vec<CatalogEntry> = builtin_skills()
        .into_iter()
        .map(|s| CatalogEntry {
            id: s.id.clone(),
            kind: "skill".into(),
            name: s.name,
            description: s.description,
            installed: installed_skills.contains(&s.id),
        })
        .collect();

    entries.push(CatalogEntry {
        id: "sample-continuity-plugin".into(),
        kind: "plugin".into(),
        name: "Continuity Brief Plugin".into(),
        description:
            "Sample plugin (repo plugins/sample-continuity-plugin) — Install from folder to dogfood."
                .into(),
        installed: inventory
            .plugins
            .iter()
            .any(|p| p.id == "sample-continuity-plugin"),
    });

    entries.push(CatalogEntry {
        id: "remote-registry".into(),
        kind: "coming_soon".into(),
        name: "OpenMesh Registry".into(),
        description: "Remote curated registry — coming soon. Local folder install works today."
            .into(),
        installed: false,
    });

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_skill_frontmatter_and_body() {
        let raw = r#"---
name: Demo Skill
description: Does a thing
---
# Hello

Body text.
"#;
        let skill = parse_skill_markdown("demo-skill", raw).unwrap();
        assert_eq!(skill.name, "Demo Skill");
        assert_eq!(skill.description, "Does a thing");
        assert!(skill.body.contains("Body text"));
    }

    #[test]
    fn enable_filter_defaults_true() {
        let settings = ExtensionsSettings::default();
        assert!(settings.is_enabled(&settings.skills, "anything"));
        let mut settings = ExtensionsSettings::default();
        settings.set_skill("concise-replies", false);
        assert!(!settings.is_enabled(&settings.skills, "concise-replies"));
    }

    #[test]
    fn build_prompt_only_enabled() {
        let mut skills = builtin_skills();
        for s in &mut skills {
            s.enabled = s.id == "concise-replies";
        }
        let section = build_skills_prompt_section(&skills);
        assert!(section.contains("Concise Replies"));
        assert!(!section.contains("OpenMesh Voice"));
    }

    #[test]
    fn parse_plugin_manifest_json() {
        let raw = r#"{
          "id": "sample-continuity-plugin",
          "name": "Continuity Brief",
          "version": "0.1.0",
          "description": "Demo",
          "skills": ["skills/brief"],
          "hooks": [
            { "id": "focus", "event": "on_before_turn", "appendContext": "Use Continuity." }
          ]
        }"#;
        let m = parse_plugin_manifest(raw).unwrap();
        assert_eq!(m.id, "sample-continuity-plugin");
        assert_eq!(m.skills.len(), 1);
        assert_eq!(m.hooks[0].event, "on_before_turn");
    }

    #[test]
    fn load_inventory_respects_disabled_skill() {
        let mut settings = ExtensionsSettings::default();
        settings.set_skill("openmesh-voice", false);
        let inv = load_inventory(None, &settings);
        let voice = inv.skills.iter().find(|s| s.id == "openmesh-voice").unwrap();
        assert!(!voice.enabled);
        let concise = inv
            .skills
            .iter()
            .find(|s| s.id == "concise-replies")
            .unwrap();
        assert!(concise.enabled);
    }

    #[test]
    fn install_skill_folder_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("my-skill");
        fs::create_dir_all(&src).unwrap();
        let mut f = fs::File::create(src.join("SKILL.md")).unwrap();
        writeln!(
            f,
            "---\nname: My Skill\ndescription: Test\n---\n\n# Body\n\nDo X.\n"
        )
        .unwrap();

        // Redirect user root by installing into a temp via env is hard — call copy path logic via install
        // after temporarily overriding is not available. Test parse + copy_dir instead.
        let dest = tmp.path().join("dest").join("my-skill");
        copy_dir_recursive(&src, &dest).unwrap();
        let loaded = load_skill_dir(&dest, ExtensionSource::User, None)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "My Skill");
        assert!(loaded.body.contains("Do X"));
    }

    #[test]
    fn enrich_includes_hooks_on_new_chat() {
        let inv = ExtensionsInventory {
            skills: vec![],
            hooks: builtin_hooks(),
            plugins: vec![],
        };
        let p = enrich_system_prompt("BASE", &inv, true);
        assert!(p.contains("BASE"));
        assert!(p.contains("Hook Context"));
        assert!(p.contains("turn-focus"));
    }
}

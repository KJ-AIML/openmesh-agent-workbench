//! OpenAI-compatible chat/completions client with tool_calls.

use super::types::{
    AgentDefinition, AgentEngineError, AgentProviderKind, ChatMessage, ChatRole, ToolCallRequest,
    ToolSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl ProviderConfig {
    pub fn from_definition(
        def: &AgentDefinition,
        api_key: &str,
    ) -> Result<Self, AgentEngineError> {
        if api_key.trim().is_empty() {
            return Err(AgentEngineError::MissingApiKey);
        }
        let base_url = match def.provider {
            AgentProviderKind::OpenAi => def
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            AgentProviderKind::DeepSeek => def
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.deepseek.com/v1".into()),
            AgentProviderKind::OpenAiCompatible => def.base_url.clone().ok_or_else(|| {
                AgentEngineError::Provider("baseUrl required for openai-compatible".into())
            })?,
        };
        Ok(Self {
            api_key: api_key.to_string(),
            model: def.model.clone(),
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssistantTurn {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
}

pub trait ChatProvider: Send + Sync {
    fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<AssistantTurn, AgentEngineError>;
}

/// Live HTTP OpenAI-compatible provider.
pub struct OpenAiCompatibleProvider {
    pub config: ProviderConfig,
    pub client: reqwest::blocking::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProbeResult {
    pub ok: bool,
    pub model: String,
    pub base_url: String,
    pub latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, AgentEngineError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AgentEngineError::Provider(e.to_string()))?;
        Ok(Self { config, client })
    }

    /// Minimal chat/completions probe (no tools) for Settings → Test connection.
    pub fn probe_connection(&self) -> Result<ProviderProbeResult, AgentEngineError> {
        if is_dashscope_coding_plan_base(&self.config.base_url) {
            return Ok(ProviderProbeResult {
                ok: false,
                model: self.config.model.clone(),
                base_url: redact_base(&self.config.base_url),
                latency_ms: 0,
                reply_preview: None,
                error: Some(
                    "DashScope Coding Plan rejects OpenMesh Agent Engine. Use openai / deepseek / xai or DashScope compatible-mode (not coding-intl)."
                        .into(),
                ),
            });
        }

        let started = std::time::Instant::now();
        let url = format!("{}/chat/completions", self.config.base_url);
        let body = json!({
            "model": self.config.model,
            "messages": [
                { "role": "user", "content": "Reply with exactly: ok" }
            ],
            "max_tokens": 16,
            "enable_thinking": false,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("Content-Type", "application/json")
            .header("User-Agent", "OpenMesh-AgentEngine/0.1.23")
            .json(&body)
            .send()
            .map_err(|e| AgentEngineError::Provider(sanitize_err(e.to_string())))?;

        let latency_ms = started.elapsed().as_millis() as u64;
        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| AgentEngineError::Provider(sanitize_err(e.to_string())))?;

        if !status.is_success() {
            let err = if text.to_lowercase().contains("coding plan")
                || text.to_lowercase().contains("coding agents")
            {
                "Coding Plan endpoint rejected this client. Use a pay-as-you-go / OpenAI-compatible provider.".into()
            } else {
                format!(
                    "HTTP {status}: {}",
                    sanitize_err(truncate(&text, 280))
                )
            };
            return Ok(ProviderProbeResult {
                ok: false,
                model: self.config.model.clone(),
                base_url: redact_base(&self.config.base_url),
                latency_ms,
                reply_preview: None,
                error: Some(err),
            });
        }

        let turn = parse_chat_completion(&text)?;
        let preview = turn.content.trim();
        Ok(ProviderProbeResult {
            ok: true,
            model: self.config.model.clone(),
            base_url: redact_base(&self.config.base_url),
            latency_ms,
            reply_preview: Some(truncate(preview, 80)),
            error: None,
        })
    }
}

fn redact_base(base_url: &str) -> String {
    // Keep host path; never include query secrets.
    base_url.split('?').next().unwrap_or(base_url).to_string()
}

/// Resolve provider kind + base URL from Settings-style fields.
pub fn resolve_provider_kind(
    name: Option<&str>,
    base_url: Option<&str>,
) -> (AgentProviderKind, Option<String>) {
    let n = name.unwrap_or("openai").to_lowercase();
    if let Some(url) = base_url.filter(|u| !u.trim().is_empty()) {
        return (AgentProviderKind::OpenAiCompatible, Some(url.to_string()));
    }
    if n.contains("deepseek") {
        return (AgentProviderKind::DeepSeek, None);
    }
    if n.contains("xai") || n.contains("grok") {
        return (
            AgentProviderKind::OpenAiCompatible,
            Some("https://api.x.ai/v1".into()),
        );
    }
    (AgentProviderKind::OpenAi, None)
}

/// Build config + run probe (used by Desktop/CLI).
pub fn probe_provider(
    api_key: &str,
    model: &str,
    provider_name: Option<&str>,
    base_url: Option<&str>,
) -> Result<ProviderProbeResult, AgentEngineError> {
    let (kind, resolved_base) = resolve_provider_kind(provider_name, base_url);
    let mut def = AgentDefinition::default_workspace_agent(model);
    def.provider = kind;
    def.base_url = resolved_base;
    let config = ProviderConfig::from_definition(&def, api_key)?;
    let client = OpenAiCompatibleProvider::new(config)?;
    client.probe_connection()
}

impl ChatProvider for OpenAiCompatibleProvider {
    fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<AssistantTurn, AgentEngineError> {
        if is_dashscope_coding_plan_base(&self.config.base_url) {
            return Err(AgentEngineError::Provider(
                "DashScope Coding Plan (coding-intl.dashscope.aliyuncs.com) only allows Coding Agents — not OpenMesh Agent Engine chat/tools. \
Use a normal OpenAI-compatible endpoint: openai, deepseek, xai (https://api.x.ai/v1), or DashScope compatible-mode (not Coding Plan). \
Slash tools still work without the LLM."
                    .into(),
            ));
        }

        let url = format!("{}/chat/completions", self.config.base_url);
        let mut body = build_request_body(&self.config.model, messages, tools);
        // Harmless for most providers; required by some Qwen-compatible hosts.
        if let Some(obj) = body.as_object_mut() {
            obj.entry("enable_thinking")
                .or_insert(Value::Bool(false));
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("Content-Type", "application/json")
            .header("User-Agent", "OpenMesh-AgentEngine/0.1.23")
            .json(&body)
            .send()
            .map_err(|e| AgentEngineError::Provider(sanitize_err(e.to_string())))?;

        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| AgentEngineError::Provider(sanitize_err(e.to_string())))?;
        if !status.is_success() {
            if text.to_lowercase().contains("coding plan")
                || text.to_lowercase().contains("coding agents")
            {
                return Err(AgentEngineError::Provider(
                    "This API key/endpoint is a Coding Plan product — it rejects OpenMesh Agent Engine. \
Switch Settings → Provider to openai / deepseek / xai (or clear Coding Plan base URL). Slash tools still work."
                        .into(),
                ));
            }
            return Err(AgentEngineError::Provider(format!(
                "HTTP {status}: {}",
                sanitize_err(truncate(&text, 400))
            )));
        }
        parse_chat_completion(&text)
    }
}

fn is_dashscope_coding_plan_base(base_url: &str) -> bool {
    let lower = base_url.to_ascii_lowercase();
    lower.contains("coding-intl.dashscope.aliyuncs.com")
        || lower.contains("coding.dashscope.aliyuncs.com")
}

fn sanitize_err(s: String) -> String {
    // Never echo long bodies that might contain credentials.
    let lower = s.to_lowercase();
    if lower.contains("bearer ") || lower.contains("api_key") || lower.contains("authorization") {
        "provider request failed (details redacted)".into()
    } else {
        s
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

pub fn build_request_body(model: &str, messages: &[ChatMessage], tools: &[ToolSpec]) -> Value {
    let msgs: Vec<Value> = messages.iter().map(message_to_json).collect();
    let mut body = json!({
        "model": model,
        "messages": msgs,
    });
    if !tools.is_empty() {
        let tool_defs: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = Value::Array(tool_defs);
        body["tool_choice"] = json!("auto");
    }
    body
}

fn message_to_json(m: &ChatMessage) -> Value {
    let role = match m.role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
    };
    let mut obj = json!({
        "role": role,
        "content": m.content,
    });
    if let Some(id) = &m.tool_call_id {
        obj["tool_call_id"] = json!(id);
    }
    if let Some(name) = &m.name {
        obj["name"] = json!(name);
    }
    if !m.tool_calls.is_empty() {
        obj["tool_calls"] = Value::Array(
            m.tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect(),
        );
        // Some APIs want content null when tool_calls present
        if m.content.is_empty() {
            obj["content"] = Value::Null;
        }
    }
    obj
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RawToolCall {
    id: String,
    function: RawFunction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RawFunction {
    name: String,
    arguments: String,
}

pub fn parse_chat_completion(body: &str) -> Result<AssistantTurn, AgentEngineError> {
    let parsed: ChatCompletionResponse = serde_json::from_str(body)
        .map_err(|e| AgentEngineError::InvalidResponse(e.to_string()))?;
    let msg = parsed
        .choices
        .first()
        .map(|c| &c.message)
        .ok_or_else(|| AgentEngineError::InvalidResponse("no choices".into()))?;
    let tool_calls = msg
        .tool_calls
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|tc| ToolCallRequest {
            id: tc.id,
            name: tc.function.name,
            arguments: tc.function.arguments,
        })
        .collect();
    Ok(AssistantTurn {
        content: msg.content.clone().unwrap_or_default(),
        tool_calls,
    })
}

/// Scripted provider for tests.
pub struct ScriptedProvider {
    pub turns: std::sync::Mutex<Vec<AssistantTurn>>,
}

impl ScriptedProvider {
    pub fn new(turns: Vec<AssistantTurn>) -> Self {
        Self {
            turns: std::sync::Mutex::new(turns),
        }
    }
}

impl ChatProvider for ScriptedProvider {
    fn complete(
        &self,
        _messages: &[ChatMessage],
        _tools: &[ToolSpec],
    ) -> Result<AssistantTurn, AgentEngineError> {
        let mut guard = self
            .turns
            .lock()
            .map_err(|e| AgentEngineError::Provider(e.to_string()))?;
        if guard.is_empty() {
            return Err(AgentEngineError::Provider("no scripted turns left".into()));
        }
        Ok(guard.remove(0))
    }
}

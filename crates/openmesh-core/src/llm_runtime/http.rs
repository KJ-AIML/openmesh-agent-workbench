//! Shared OpenAI-compatible HTTP execution (blocking + async).
//!
//! Agent Engine uses the blocking client. The HTTP proxy adapter uses the
//! async client against the same URL/body/parse/error rules — no localhost hop.

use super::spec::ProviderRuntimeSpec;
use super::LlmRuntimeError;
use serde_json::Value;

pub fn is_dashscope_coding_plan_base(base_url: &str) -> bool {
    let lower = base_url.to_ascii_lowercase();
    lower.contains("coding-intl.dashscope.aliyuncs.com")
        || lower.contains("coding.dashscope.aliyuncs.com")
}

pub fn classify_chat_http(status: u16, body: &str) -> LlmRuntimeError {
    let lower = body.to_ascii_lowercase();
    if lower.contains("coding plan") || lower.contains("coding agents") {
        return LlmRuntimeError::Upstream(
            "This API key/endpoint is a Coding Plan product — it rejects OpenMesh Agent Engine. \
Switch Settings → Provider to openai / deepseek / xai (or clear Coding Plan base URL). Slash tools still work."
                .into(),
        );
    }
    LlmRuntimeError::from_http_status(status, body)
}

pub fn complete_openai_chat_blocking(
    client: &reqwest::blocking::Client,
    spec: &ProviderRuntimeSpec,
    body: Value,
) -> Result<String, LlmRuntimeError> {
    if spec.protocol != crate::proxy_server::ProxyProviderProtocol::OpenAiCompatible {
        return Err(LlmRuntimeError::Upstream(
            "in-process adapter currently supports openai-compatible upstreams".into(),
        ));
    }
    if is_dashscope_coding_plan_base(&spec.base_url) {
        return Err(LlmRuntimeError::Upstream(
            "DashScope Coding Plan (coding-intl.dashscope.aliyuncs.com) only allows Coding Agents — not OpenMesh Agent Engine chat/tools. \
Use a normal OpenAI-compatible endpoint: openai, deepseek, xai (https://api.x.ai/v1), or DashScope compatible-mode (not Coding Plan). \
Slash tools still work without the LLM."
                .into(),
        ));
    }
    let url = spec.chat_completions_url();
    let resp = client
        .post(&url)
        .bearer_auth(&spec.api_key)
        .header("Content-Type", "application/json")
        .header("User-Agent", "OpenMesh-ProviderRuntime/0.2")
        .json(&body)
        .send()
        .map_err(|_| LlmRuntimeError::ProviderUnavailable)?;
    let status = resp.status();
    let text = resp
        .text()
        .map_err(|_| LlmRuntimeError::ProviderUnavailable)?;
    if !status.is_success() {
        return Err(classify_chat_http(
            status.as_u16(),
            &truncate_body(&text, 400),
        ));
    }
    Ok(text)
}

pub async fn complete_openai_chat_async(
    client: &reqwest::Client,
    spec: &ProviderRuntimeSpec,
    body: Value,
) -> Result<String, LlmRuntimeError> {
    if spec.protocol != crate::proxy_server::ProxyProviderProtocol::OpenAiCompatible {
        return Err(LlmRuntimeError::Upstream(
            "in-process adapter currently supports openai-compatible upstreams".into(),
        ));
    }
    if is_dashscope_coding_plan_base(&spec.base_url) {
        return Err(LlmRuntimeError::Upstream(
            "DashScope Coding Plan (coding-intl.dashscope.aliyuncs.com) only allows Coding Agents — not OpenMesh Agent Engine chat/tools. \
Use a normal OpenAI-compatible endpoint: openai, deepseek, xai (https://api.x.ai/v1), or DashScope compatible-mode (not Coding Plan). \
Slash tools still work without the LLM."
                .into(),
        ));
    }
    let url = spec.chat_completions_url();
    let resp = client
        .post(&url)
        .bearer_auth(&spec.api_key)
        .header("Content-Type", "application/json")
        .header("User-Agent", "OpenMesh-ProviderRuntime/0.2")
        .json(&body)
        .send()
        .await
        .map_err(|_| LlmRuntimeError::ProviderUnavailable)?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|_| LlmRuntimeError::ProviderUnavailable)?;
    if !status.is_success() {
        return Err(classify_chat_http(
            status.as_u16(),
            &truncate_body(&text, 400),
        ));
    }
    Ok(text)
}

fn truncate_body(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

//! Agent tool loop — OpenMesh Agent Engine (0.1.23).

use super::provider::ChatProvider;
use super::registry::{filter_tools, ToolExecutor};
use super::types::{
    AgentDefinition, AgentEngineError, AgentSession, ChatMessage, ChatRole, EngineTurnResult,
    ToolStep, DEFAULT_TOOL_RESULT_MAX_CHARS,
};

pub fn run_agent_turn(
    def: &AgentDefinition,
    session: &mut AgentSession,
    user_text: &str,
    provider: &dyn ChatProvider,
    executor: &dyn ToolExecutor,
) -> Result<EngineTurnResult, AgentEngineError> {
    let tools = filter_tools(&def.tool_allowlist);
    // Keep system prompt current (skills/hooks may change between turns).
    ensure_system_prompt(session, &def.system_prompt);
    session.messages.push(ChatMessage {
        role: ChatRole::User,
        content: user_text.to_string(),
        tool_call_id: None,
        name: None,
        tool_calls: vec![],
    });

    let mut tool_steps = Vec::new();
    let mut iterations = 0u32;

    loop {
        iterations += 1;
        if iterations > def.max_tool_iterations {
            return Err(AgentEngineError::MaxIterations(def.max_tool_iterations));
        }

        let turn = provider.complete(&session.messages, &tools)?;

        if turn.tool_calls.is_empty() {
            let text = turn.content.trim().to_string();
            session.messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: text.clone(),
                tool_call_id: None,
                name: None,
                tool_calls: vec![],
            });
            return Ok(EngineTurnResult {
                assistant_text: if text.is_empty() {
                    "(empty model response)".into()
                } else {
                    text
                },
                tool_steps,
                iterations,
                model: def.model.clone(),
                provider: format!("{:?}", def.provider),
                refused: false,
                error: None,
            });
        }

        // Assistant message with tool_calls
        session.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: turn.content.clone(),
            tool_call_id: None,
            name: None,
            tool_calls: turn.tool_calls.clone(),
        });

        for call in &turn.tool_calls {
            let allowed = tools.iter().any(|t| t.name == call.name);
            let (ok, summary) = if !allowed {
                (false, format!("tool not allowed: {}", call.name))
            } else {
                match executor.execute(&call.name, &call.arguments) {
                    Ok(raw) => (true, clip(&raw, DEFAULT_TOOL_RESULT_MAX_CHARS)),
                    Err(e) => (false, clip(&e, DEFAULT_TOOL_RESULT_MAX_CHARS)),
                }
            };
            tool_steps.push(ToolStep {
                tool_name: call.name.clone(),
                tool_call_id: call.id.clone(),
                ok,
                summary: summary.clone(),
            });
            session.messages.push(ChatMessage {
                role: ChatRole::Tool,
                content: summary,
                tool_call_id: Some(call.id.clone()),
                name: Some(call.name.clone()),
                tool_calls: vec![],
            });
        }
    }
}

fn ensure_system_prompt(session: &mut AgentSession, prompt: &str) {
    if let Some(first) = session.messages.first_mut() {
        if first.role == ChatRole::System {
            first.content = prompt.to_string();
            return;
        }
    }
    session.messages.insert(
        0,
        ChatMessage {
            role: ChatRole::System,
            content: prompt.to_string(),
            tool_call_id: None,
            name: None,
            tool_calls: vec![],
        },
    );
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}\n…(truncated)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::provider::{AssistantTurn, ScriptedProvider};
    use crate::agent_engine::registry::StubToolExecutor;
    use crate::agent_engine::types::ToolCallRequest;
    use std::collections::BTreeMap;

    #[test]
    fn tool_call_then_final_answer() {
        let provider = ScriptedProvider::new(vec![
            AssistantTurn {
                content: String::new(),
                tool_calls: vec![ToolCallRequest {
                    id: "call_1".into(),
                    name: "project_info".into(),
                    arguments: "{}".into(),
                }],
            },
            AssistantTurn {
                content: "Project is ready.".into(),
                tool_calls: vec![],
            },
        ]);
        let mut responses = BTreeMap::new();
        responses.insert("project_info".into(), r#"{"path":"/tmp/p"}"#.into());
        let executor = StubToolExecutor { responses };
        let def = AgentDefinition::default_workspace_agent("test-model");
        let mut session = AgentSession::default();
        let result = run_agent_turn(
            &def,
            &mut session,
            "What project am I in?",
            &provider,
            &executor,
        )
        .expect("turn");
        assert_eq!(result.assistant_text, "Project is ready.");
        assert_eq!(result.tool_steps.len(), 1);
        assert!(result.tool_steps[0].ok);
        assert_eq!(result.tool_steps[0].tool_name, "project_info");
        assert_eq!(result.iterations, 2);
    }

    #[test]
    fn direct_answer_without_tools() {
        let provider = ScriptedProvider::new(vec![AssistantTurn {
            content: "Hello.".into(),
            tool_calls: vec![],
        }]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let def = AgentDefinition::default_workspace_agent("m");
        let mut session = AgentSession::default();
        let result = run_agent_turn(&def, &mut session, "hi", &provider, &executor).unwrap();
        assert_eq!(result.assistant_text, "Hello.");
        assert!(result.tool_steps.is_empty());
    }
}

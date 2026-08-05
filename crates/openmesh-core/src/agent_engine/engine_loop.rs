//! Agent tool loop — OpenMesh Agent Engine (0.1.23).

use super::provider::ChatProvider;
use super::registry::{filter_tools, ToolExecutor};
use super::turn_cancel;
use super::types::{
    AgentDefinition, AgentEngineError, AgentSession, ChatMessage, ChatRole, EngineTurnResult,
    ToolStep, DEFAULT_MAX_TOOLS_PER_ITERATION, DEFAULT_TOOL_RESULT_MAX_CHARS,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn run_agent_turn(
    def: &AgentDefinition,
    session: &mut AgentSession,
    user_text: &str,
    provider: &dyn ChatProvider,
    executor: &dyn ToolExecutor,
) -> Result<EngineTurnResult, AgentEngineError> {
    run_agent_turn_cancellable(def, session, user_text, provider, executor, None)
}

pub fn run_agent_turn_cancellable(
    def: &AgentDefinition,
    session: &mut AgentSession,
    user_text: &str,
    provider: &dyn ChatProvider,
    executor: &dyn ToolExecutor,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<EngineTurnResult, AgentEngineError> {
    let tools = filter_tools(&def.tool_allowlist);
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
    let mut total_tools: usize = 0;

    loop {
        if cancel
            .as_ref()
            .map(|f| turn_cancel::is_cancelled(f))
            .unwrap_or(false)
        {
            return Ok(EngineTurnResult {
                assistant_text: "Turn cancelled.".into(),
                tool_steps,
                iterations,
                model: def.model.clone(),
                provider: format!("{:?}", def.provider),
                refused: false,
                error: Some("cancelled".into()),
            });
        }

        iterations += 1;
        if iterations > def.max_tool_iterations {
            // Soft stop: return what we have instead of failing the whole turn.
            return Ok(EngineTurnResult {
                assistant_text: format!(
                    "Stopped after {iterations} model rounds (tool budget). \
                     Ask a more specific question, or switch to Plan/Act for deeper work."
                ),
                tool_steps,
                iterations,
                model: def.model.clone(),
                provider: format!("{:?}", def.provider),
                refused: false,
                error: Some(format!("max_iterations:{}", def.max_tool_iterations)),
            });
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

        // Cap parallel tool storms from a single model message.
        let (accepted, skipped) = if turn.tool_calls.len() > DEFAULT_MAX_TOOLS_PER_ITERATION {
            let (head, tail) = turn.tool_calls.split_at(DEFAULT_MAX_TOOLS_PER_ITERATION);
            (head.to_vec(), tail.to_vec())
        } else {
            (turn.tool_calls.clone(), Vec::new())
        };

        session.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: turn.content.clone(),
            tool_call_id: None,
            name: None,
            tool_calls: accepted.clone(),
        });

        for call in &accepted {
            if cancel
                .as_ref()
                .map(|f| turn_cancel::is_cancelled(f))
                .unwrap_or(false)
            {
                return Ok(EngineTurnResult {
                    assistant_text: "Turn cancelled during tool execution.".into(),
                    tool_steps,
                    iterations,
                    model: def.model.clone(),
                    provider: format!("{:?}", def.provider),
                    refused: false,
                    error: Some("cancelled".into()),
                });
            }
            let allowed = tools.iter().any(|t| t.name == call.name);
            let (ok, summary) = if !allowed {
                (false, format!("tool not allowed: {}", call.name))
            } else {
                match executor.execute(&call.name, &call.arguments) {
                    Ok(raw) => (true, clip(&raw, DEFAULT_TOOL_RESULT_MAX_CHARS)),
                    Err(e) => (false, clip(&e, DEFAULT_TOOL_RESULT_MAX_CHARS)),
                }
            };
            total_tools += 1;
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

        if !skipped.is_empty() {
            let names: Vec<_> = skipped.iter().map(|c| c.name.as_str()).collect();
            let notice = format!(
                "\n\n[tool budget] skipped {} extra call(s) this round ({}). \
                 Answer now or ask the user — do not continue exhaustive search.",
                skipped.len(),
                names.join(", ")
            );
            // Append to the last real tool result (keeps OpenAI tool_call_id pairing valid).
            if let Some(last) = session.messages.last_mut() {
                if last.role == ChatRole::Tool {
                    last.content.push_str(&notice);
                }
            }
            if let Some(step) = tool_steps.last_mut() {
                step.summary.push_str(&notice);
            }
        }

        // Absolute ceiling across the whole turn (iterations × per-iter cap).
        let hard_cap = def.max_tool_iterations as usize * DEFAULT_MAX_TOOLS_PER_ITERATION;
        if total_tools >= hard_cap {
            return Ok(EngineTurnResult {
                assistant_text: format!(
                    "Stopped after {total_tools} tool calls (budget). \
                     Rephrase more specifically, or use Plan/Act for deeper work."
                ),
                tool_steps,
                iterations,
                model: def.model.clone(),
                provider: format!("{:?}", def.provider),
                refused: false,
                error: Some(format!("max_tools:{hard_cap}")),
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
    use std::sync::atomic::Ordering;

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

    #[test]
    fn cancelled_before_first_complete() {
        let provider = ScriptedProvider::new(vec![AssistantTurn {
            content: "should not run".into(),
            tool_calls: vec![],
        }]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let def = AgentDefinition::default_workspace_agent("m");
        let mut session = AgentSession::default();
        let flag = Arc::new(AtomicBool::new(true));
        let result = run_agent_turn_cancellable(
            &def,
            &mut session,
            "hi",
            &provider,
            &executor,
            Some(flag.clone()),
        )
        .unwrap();
        assert_eq!(result.error.as_deref(), Some("cancelled"));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn caps_tools_per_iteration() {
        let many: Vec<_> = (0..6)
            .map(|i| ToolCallRequest {
                id: format!("c{i}"),
                name: "project_info".into(),
                arguments: "{}".into(),
            })
            .collect();
        let provider = ScriptedProvider::new(vec![
            AssistantTurn {
                content: String::new(),
                tool_calls: many,
            },
            AssistantTurn {
                content: "Done with budget.".into(),
                tool_calls: vec![],
            },
        ]);
        let mut responses = BTreeMap::new();
        responses.insert("project_info".into(), "{}".into());
        let executor = StubToolExecutor { responses };
        let def = AgentDefinition::default_workspace_agent("m");
        let mut session = AgentSession::default();
        let result = run_agent_turn(&def, &mut session, "x", &provider, &executor).unwrap();
        let executed = result
            .tool_steps
            .iter()
            .filter(|t| t.tool_name == "project_info")
            .count();
        assert_eq!(executed, DEFAULT_MAX_TOOLS_PER_ITERATION);
        assert!(result
            .tool_steps
            .last()
            .map(|t| t.summary.contains("[tool budget]"))
            .unwrap_or(false));
        assert_eq!(result.assistant_text, "Done with budget.");
    }
}

//! E2E: session persistence lifecycle tests (bd-277x).
//!
//! These tests exercise the real `AgentSession` + `Session` persistence path
//! using deterministic in-process provider streams.

mod common;

use asupersync::runtime::RuntimeBuilder;
use async_trait::async_trait;
use common::TestHarness;
use futures::Stream;
use pi::agent::{Agent, AgentConfig, AgentSession};
use pi::compaction::ResolvedCompactionSettings;
use pi::error::{Error, Result};
use pi::model::{
    AssistantMessage, ContentBlock, Message, StopReason, StreamEvent, TextContent, ToolCall, Usage,
    UserContent,
};
use pi::provider::{Context, Provider, StreamOptions};
use pi::session::{Session, SessionEntry, SessionMessage};
use pi::tools::ToolRegistry;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
struct PlannedStep {
    stop_reason: StopReason,
    content: Vec<ContentBlock>,
    min_context_messages: usize,
    total_tokens: u64,
}

#[derive(Debug)]
struct PlannedProvider {
    steps: Vec<PlannedStep>,
    call_count: AtomicUsize,
}

impl PlannedProvider {
    fn new(steps: Vec<PlannedStep>) -> Self {
        Self {
            steps,
            call_count: AtomicUsize::new(0),
        }
    }

    fn assistant_message(
        &self,
        stop_reason: StopReason,
        content: Vec<ContentBlock>,
        total_tokens: u64,
    ) -> AssistantMessage {
        AssistantMessage {
            content,
            api: self.api().to_string(),
            provider: self.name().to_string(),
            model: self.model_id().to_string(),
            usage: Usage {
                total_tokens,
                output: total_tokens,
                ..Usage::default()
            },
            stop_reason,
            error_message: None,
            timestamp: 0,
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Provider for PlannedProvider {
    fn name(&self) -> &str {
        "planned-provider"
    }

    fn api(&self) -> &str {
        "planned-api"
    }

    fn model_id(&self) -> &str {
        "planned-model"
    }

    async fn stream(
        &self,
        context: &Context,
        _options: &StreamOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let index = self.call_count.fetch_add(1, Ordering::SeqCst);
        let Some(step) = self.steps.get(index) else {
            return Err(Error::api("planned provider exhausted its scripted steps"));
        };
        if context.messages.len() < step.min_context_messages {
            return Err(Error::api(format!(
                "planned provider expected >= {} context messages, got {}",
                step.min_context_messages,
                context.messages.len()
            )));
        }

        let message =
            self.assistant_message(step.stop_reason, step.content.clone(), step.total_tokens);
        let partial = self.assistant_message(StopReason::Stop, Vec::new(), 0);
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(StreamEvent::Start { partial }),
            Ok(StreamEvent::Done {
                reason: message.stop_reason,
                message,
            }),
        ])))
    }
}

fn run_async_test<F>(future: F)
where
    F: std::future::Future<Output = ()>,
{
    let runtime = RuntimeBuilder::current_thread()
        .build()
        .expect("runtime build");
    runtime.block_on(future);
}

fn text_step(text: &str, min_context_messages: usize, total_tokens: u64) -> PlannedStep {
    PlannedStep {
        stop_reason: StopReason::Stop,
        content: vec![ContentBlock::Text(TextContent::new(text))],
        min_context_messages,
        total_tokens,
    }
}

fn tool_step(tool_call: ToolCall, min_context_messages: usize, total_tokens: u64) -> PlannedStep {
    PlannedStep {
        stop_reason: StopReason::ToolUse,
        content: vec![ContentBlock::ToolCall(tool_call)],
        min_context_messages,
        total_tokens,
    }
}

fn tool_names() -> [&'static str; 7] {
    ["read", "write", "edit", "bash", "grep", "find", "ls"]
}

fn make_agent_session(
    cwd: &Path,
    provider: Arc<dyn Provider>,
    session: Arc<asupersync::sync::Mutex<Session>>,
) -> AgentSession {
    let agent = Agent::new(
        provider,
        ToolRegistry::new(&tool_names(), cwd, None),
        AgentConfig {
            max_tool_iterations: 12,
            stream_options: StreamOptions {
                api_key: Some("test-key".to_string()),
                ..StreamOptions::default()
            },
            ..AgentConfig::default()
        },
    );
    AgentSession::new(agent, session, true, ResolvedCompactionSettings::default())
}

fn write_jsonl_artifacts(harness: &TestHarness, test_name: &str) {
    let log_path = harness.temp_path(format!("{test_name}.log.jsonl"));
    harness
        .write_jsonl_logs(&log_path)
        .expect("write jsonl logs");
    harness.record_artifact(format!("{test_name}.log.jsonl"), &log_path);

    let normalized_log_path = harness.temp_path(format!("{test_name}.log.normalized.jsonl"));
    harness
        .write_jsonl_logs_normalized(&normalized_log_path)
        .expect("write normalized jsonl logs");
    harness.record_artifact(
        format!("{test_name}.log.normalized.jsonl"),
        &normalized_log_path,
    );
}

async fn current_session_path(session: &Arc<asupersync::sync::Mutex<Session>>) -> PathBuf {
    let cx = asupersync::Cx::for_testing();
    let guard = session.lock(&cx).await.expect("lock session");
    guard.path.clone().expect("session path")
}

async fn current_messages(session: &Arc<asupersync::sync::Mutex<Session>>) -> Vec<Message> {
    let cx = asupersync::Cx::for_testing();
    let guard = session.lock(&cx).await.expect("lock session");
    guard.to_messages_for_current_path()
}

#[test]
fn create_and_save() {
    let test_name = "e2e_session_create_and_save";
    let harness = TestHarness::new(test_name);
    run_async_test(async {
        let cwd = harness.temp_dir().to_path_buf();
        let session = Arc::new(asupersync::sync::Mutex::new(Session::create_with_dir(
            Some(cwd.clone()),
        )));
        let provider: Arc<dyn Provider> = Arc::new(PlannedProvider::new(vec![text_step(
            "created session",
            1,
            12,
        )]));
        let mut agent_session = make_agent_session(&cwd, provider, Arc::clone(&session));

        let response = agent_session
            .run_text("hello persistence".to_string(), |_| {})
            .await
            .expect("run first turn");
        assert_eq!(response.stop_reason, StopReason::Stop);

        agent_session
            .persist_session()
            .await
            .expect("persist session");
        let path = current_session_path(&session).await;
        harness.record_artifact("session.jsonl", &path);

        assert!(path.exists(), "session file should exist");
        let raw = std::fs::read_to_string(&path).expect("read session jsonl");
        let lines = raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        assert!(
            lines.len() >= 3,
            "expected header + user + assistant entries"
        );

        let header: serde_json::Value = serde_json::from_str(lines[0]).expect("parse header line");
        assert_eq!(
            header.get("type").and_then(serde_json::Value::as_str),
            Some("session")
        );
        assert!(
            lines.iter().any(|line| line.contains("\"role\":\"user\"")),
            "missing user message entry"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("\"role\":\"assistant\"")),
            "missing assistant message entry"
        );
    });
    write_jsonl_artifacts(&harness, test_name);
}

#[test]
fn reload_session() {
    let test_name = "e2e_session_reload_continue";
    let harness = TestHarness::new(test_name);
    run_async_test(async {
        let cwd = harness.temp_dir().to_path_buf();
        let session = Arc::new(asupersync::sync::Mutex::new(Session::create_with_dir(
            Some(cwd.clone()),
        )));
        let initial_provider: Arc<dyn Provider> = Arc::new(PlannedProvider::new(vec![text_step(
            "first response",
            1,
            10,
        )]));
        let mut first = make_agent_session(&cwd, initial_provider, Arc::clone(&session));
        first
            .run_text("first prompt".to_string(), |_| {})
            .await
            .expect("first run");
        first.persist_session().await.expect("first persist");

        let saved_path = current_session_path(&session).await;
        harness.record_artifact("initial-session.jsonl", &saved_path);
        let reopened = Session::open(saved_path.to_string_lossy().as_ref())
            .await
            .expect("reopen saved session");
        let reopened_handle = Arc::new(asupersync::sync::Mutex::new(reopened));

        let continue_provider: Arc<dyn Provider> = Arc::new(PlannedProvider::new(vec![text_step(
            "continued response",
            3,
            11,
        )]));
        let mut continued =
            make_agent_session(&cwd, continue_provider, Arc::clone(&reopened_handle));
        continued
            .run_text("second prompt".to_string(), |_| {})
            .await
            .expect("continued run");
        continued
            .persist_session()
            .await
            .expect("persist continued run");

        let messages = current_messages(&reopened_handle).await;
        let user_texts = messages
            .iter()
            .filter_map(|message| match message {
                Message::User(user) => match &user.content {
                    UserContent::Text(text) => Some(text.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            user_texts.len(),
            2,
            "expected two user prompts after reload"
        );
        assert!(user_texts.iter().any(|text| text == "first prompt"));
        assert!(user_texts.iter().any(|text| text == "second prompt"));
    });
    write_jsonl_artifacts(&harness, test_name);
}

#[test]
fn session_branching() {
    let test_name = "e2e_session_branching";
    let harness = TestHarness::new(test_name);
    run_async_test(async {
        let cwd = harness.temp_dir().to_path_buf();
        let session = Arc::new(asupersync::sync::Mutex::new(Session::create_with_dir(
            Some(cwd.clone()),
        )));
        let provider: Arc<dyn Provider> = Arc::new(PlannedProvider::new(vec![
            text_step("reply one", 1, 8),
            text_step("reply two", 3, 8),
            text_step("reply three", 5, 8),
        ]));
        let mut agent_session = make_agent_session(&cwd, provider, Arc::clone(&session));
        for prompt in ["turn one", "turn two", "turn three"] {
            agent_session
                .run_text(prompt.to_string(), |_| {})
                .await
                .expect("run turn");
        }

        let branched_from = {
            let cx = asupersync::Cx::for_testing();
            let mut guard = session.lock(&cx).await.expect("lock session");
            let user_ids = guard
                .entries
                .iter()
                .filter_map(|entry| match entry {
                    SessionEntry::Message(message_entry) => match &message_entry.message {
                        SessionMessage::User { .. } => message_entry.base.id.clone(),
                        _ => None,
                    },
                    _ => None,
                })
                .collect::<Vec<_>>();
            let target = user_ids
                .get(1)
                .cloned()
                .expect("second user message id for branch");
            assert!(guard.create_branch_from(&target), "create branch");
            guard.append_message(SessionMessage::User {
                content: UserContent::Text("branch turn".to_string()),
                timestamp: Some(0),
            });
            guard.save().await.expect("save branch");
            target
        };

        let path = current_session_path(&session).await;
        let reopened = Session::open(path.to_string_lossy().as_ref())
            .await
            .expect("reopen branched session");
        let summary = reopened.branch_summary();
        assert!(summary.branch_point_count >= 1);
        assert!(
            summary.branch_points.contains(&branched_from),
            "expected branch point at second user message"
        );
    });
    write_jsonl_artifacts(&harness, test_name);
}

#[test]
fn session_metadata() {
    let test_name = "e2e_session_metadata";
    let harness = TestHarness::new(test_name);
    run_async_test(async {
        let cwd = harness.temp_dir().to_path_buf();
        let mut session = Session::create_with_dir(Some(cwd));
        session.append_message(SessionMessage::User {
            content: UserContent::Text("metadata baseline".to_string()),
            timestamp: Some(0),
        });
        session.append_model_change("anthropic".to_string(), "claude-sonnet-4-5".to_string());
        session.append_thinking_level_change("high".to_string());
        session.set_model_header(
            Some("anthropic".to_string()),
            Some("claude-sonnet-4-5".to_string()),
            Some("high".to_string()),
        );
        session.save().await.expect("save metadata session");

        let path = session.path.clone().expect("metadata session path");
        harness.record_artifact("metadata-session.jsonl", &path);
        let raw = std::fs::read_to_string(&path).expect("read metadata session");
        assert!(raw.contains("\"type\":\"model_change\""));
        assert!(raw.contains("\"type\":\"thinking_level_change\""));

        let reopened = Session::open(path.to_string_lossy().as_ref())
            .await
            .expect("reopen metadata session");
        assert_eq!(reopened.header.provider.as_deref(), Some("anthropic"));
        assert_eq!(
            reopened.header.model_id.as_deref(),
            Some("claude-sonnet-4-5")
        );
        assert_eq!(reopened.header.thinking_level.as_deref(), Some("high"));
        assert!(
            reopened
                .entries
                .iter()
                .any(|entry| matches!(entry, SessionEntry::ModelChange(_)))
        );
        assert!(
            reopened
                .entries
                .iter()
                .any(|entry| matches!(entry, SessionEntry::ThinkingLevelChange(_)))
        );
    });
    write_jsonl_artifacts(&harness, test_name);
}

#[test]
fn multi_turn_persistence() {
    let test_name = "e2e_session_multi_turn_persistence";
    let harness = TestHarness::new(test_name);
    run_async_test(async {
        let cwd = harness.temp_dir().to_path_buf();
        let fixture = harness.create_file("fixtures/persist.txt", "persisted-value\n");
        let session = Arc::new(asupersync::sync::Mutex::new(Session::create_with_dir(
            Some(cwd.clone()),
        )));

        let steps = vec![
            text_step("turn one response", 1, 9),
            tool_step(
                ToolCall {
                    id: "read-1".to_string(),
                    name: "read".to_string(),
                    arguments: json!({ "path": fixture.display().to_string() }),
                    thought_signature: None,
                },
                3,
                18,
            ),
            text_step("tool turn completed", 5, 10),
            text_step("turn three response", 7, 11),
        ];
        let provider: Arc<dyn Provider> = Arc::new(PlannedProvider::new(steps));
        let mut agent_session = make_agent_session(&cwd, provider, Arc::clone(&session));

        agent_session
            .run_text("turn one".to_string(), |_| {})
            .await
            .expect("run turn one");
        agent_session
            .run_text("turn two with tool".to_string(), |_| {})
            .await
            .expect("run turn two");
        agent_session
            .run_text("turn three".to_string(), |_| {})
            .await
            .expect("run turn three");
        agent_session
            .persist_session()
            .await
            .expect("persist multi-turn session");

        let path = current_session_path(&session).await;
        harness.record_artifact("multi-turn-session.jsonl", &path);
        let reopened = Session::open(path.to_string_lossy().as_ref())
            .await
            .expect("reopen multi-turn session");

        let (mut user_count, mut assistant_count, mut tool_result_count) = (0usize, 0usize, 0usize);
        for entry in &reopened.entries {
            if let SessionEntry::Message(message_entry) = entry {
                match &message_entry.message {
                    SessionMessage::User { .. } => user_count += 1,
                    SessionMessage::Assistant { .. } => assistant_count += 1,
                    SessionMessage::ToolResult { .. } => tool_result_count += 1,
                    _ => {}
                }
            }
        }

        assert_eq!(user_count, 3, "expected three persisted user turns");
        assert!(
            assistant_count >= 4,
            "expected assistant tool-use + completion turns to persist"
        );
        assert!(
            tool_result_count >= 1,
            "expected persisted tool result entries"
        );
    });
    write_jsonl_artifacts(&harness, test_name);
}

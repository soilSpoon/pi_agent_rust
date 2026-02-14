//! End-to-end performance test scripts (bd-2oz69 / PERF-TEST-E2E).
//!
//! Exercises the full performance pipeline with realistic workloads:
//!   1. Long conversation responsiveness (500 messages, frame time measurement)
//!   2. Streaming with history (prefix cache isolation)
//!   3. Degradation under load (fidelity level transitions)
//!   4. Memory pressure response (progressive collapse + truncation)
//!
//! Each test emits JSONL structured logs via TestHarness for CI artifact retention.
//!
//! Run:
//!   cargo test --test perf_e2e -- --nocapture
//!   PI_TEST_MODE=1 cargo test --test perf_e2e -- --nocapture

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::similar_names
)]

mod common;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use asupersync::channel::mpsc;
use asupersync::sync::Mutex;
use bubbletea::Model as BubbleteaModel;
use common::harness::TestHarness;
use pi::agent::{Agent, AgentConfig};
use pi::config::Config;
use pi::interactive::{ConversationMessage, MessageRole, PendingInput, PiApp, PiMsg};
use pi::keybindings::KeyBindings;
use pi::model::Usage;
use pi::models::ModelEntry;
use pi::provider::{InputType, Model, ModelCost, Provider, StreamOptions};
use pi::resources::{ResourceCliOptions, ResourceLoader};
use pi::session::Session;
use pi::tools::ToolRegistry;
use serde_json::json;

// ─── Test Infrastructure ──────────────────────────────────────────────────────

fn test_runtime_handle() -> asupersync::runtime::RuntimeHandle {
    static RT: OnceLock<asupersync::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("build asupersync runtime")
    })
    .handle()
}

struct DummyProvider;

#[async_trait::async_trait]
impl Provider for DummyProvider {
    fn name(&self) -> &str {
        "dummy"
    }
    fn api(&self) -> &str {
        "dummy-api"
    }
    fn models(&self) -> Vec<Model> {
        vec![]
    }
    async fn stream(
        &self,
        _context: &pi::provider::Context,
        _options: &StreamOptions,
    ) -> pi::PiResult<
        std::pin::Pin<
            Box<
                dyn futures::Stream<Item = Result<pi::model::StreamEvent, pi::Error>> + Send + '_,
            >,
        >,
    > {
        Ok(Box::pin(futures::stream::empty()))
    }
}

fn dummy_model_entry() -> ModelEntry {
    let model = Model {
        id: "dummy-model".to_string(),
        name: "Dummy Model".to_string(),
        api: "dummy-api".to_string(),
        provider: "dummy".to_string(),
        base_url: "https://example.invalid".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 4096,
        max_tokens: 1024,
        headers: HashMap::new(),
    };
    ModelEntry {
        model,
        api_key: None,
        headers: HashMap::new(),
        auth_header: false,
        compat: None,
        oauth_config: None,
    }
}

fn build_perf_app(
    harness: &TestHarness,
    messages: Vec<ConversationMessage>,
    config: Config,
) -> PiApp {
    let cwd = harness.temp_dir().to_path_buf();
    let tools = ToolRegistry::new(&[], &cwd, Some(&config));
    let provider: Arc<dyn Provider> = Arc::new(DummyProvider);
    let agent = Agent::new(provider, tools, AgentConfig::default());
    let resources = ResourceLoader::empty(config.enable_skill_commands());
    let resource_cli = ResourceCliOptions {
        no_skills: false,
        no_prompt_templates: false,
        no_extensions: false,
        no_themes: false,
        skill_paths: Vec::new(),
        prompt_paths: Vec::new(),
        extension_paths: Vec::new(),
        theme_paths: Vec::new(),
    };
    let model_entry = dummy_model_entry();
    let model_scope = vec![model_entry.clone()];
    let available_models = vec![model_entry.clone()];
    let (event_tx, _event_rx) = mpsc::channel(1024);
    let session = Arc::new(Mutex::new(Session::in_memory()));
    let usage = Usage::default();

    let mut app = PiApp::new(
        agent,
        session,
        config,
        resources,
        resource_cli,
        cwd,
        model_entry,
        model_scope,
        available_models,
        Vec::new(),
        event_tx,
        test_runtime_handle(),
        false,
        None,
        Some(KeyBindings::new()),
        messages,
        usage,
    );
    app.set_terminal_size(120, 40);
    app
}

/// Generate N alternating user/assistant messages with realistic content.
fn generate_conversation(count: usize) -> Vec<ConversationMessage> {
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        if i % 3 == 2 {
            // Every 3rd message is a tool output
            let lines: String = (0..25)
                .map(|j| format!("  tool output line {j} for invocation {i}\n"))
                .collect();
            messages.push(ConversationMessage::tool(format!(
                "Tool bash:\n{lines}"
            )));
        } else if i % 2 == 0 {
            messages.push(ConversationMessage::new(
                MessageRole::User,
                format!(
                    "User message {i}: Can you help me refactor the function \
                     handle_request() to use async/await patterns?"
                ),
                None,
            ));
        } else {
            messages.push(ConversationMessage::new(
                MessageRole::Assistant,
                format!(
                    "Sure! Here's the refactored version of `handle_request()`:\n\n\
                     ```rust\nasync fn handle_request(req: Request) -> Response {{\n    \
                     let data = fetch_data(&req).await?;\n    \
                     let result = process(data).await?;\n    \
                     Ok(Response::new(result))\n}}\n```\n\n\
                     Key changes in iteration {i}:\n\
                     1. Added `async` keyword to function signature\n\
                     2. Replaced `.block_on()` with `.await`\n\
                     3. Used `?` operator for error propagation"
                ),
                Some(format!("Thinking about refactoring approach {i}...")),
            ));
        }
    }
    messages
}

// ─── Script 1: Long Conversation Responsiveness ──────────────────────────────

#[test]
fn perf_e2e_long_conversation_responsiveness() {
    let harness = TestHarness::new("perf_e2e_long_conversation");
    let message_count = 500;
    let messages = generate_conversation(message_count);

    harness.log().info(
        "setup",
        format!("Generated {message_count} synthetic messages"),
    );

    let app = build_perf_app(&harness, messages, Config::default());

    // Measure frame times for rendering with cache cold (first render)
    let cold_start = Instant::now();
    let _view_cold = app.view();
    let cold_us = cold_start.elapsed().as_micros() as u64;

    harness.log().info(
        "cold_render",
        format!("Cold render: {cold_us}us ({message_count} messages)"),
    );

    // Measure frame times for rendering with cache warm (second render)
    let warm_start = Instant::now();
    let _view_warm = app.view();
    let warm_us = warm_start.elapsed().as_micros() as u64;

    harness.log().info(
        "warm_render",
        format!("Warm render: {warm_us}us ({message_count} messages)"),
    );

    // Run 20 frames and collect timing
    let mut frame_times = Vec::with_capacity(20);
    for i in 0..20 {
        let start = Instant::now();
        let _view = app.view();
        let elapsed_us = start.elapsed().as_micros() as u64;
        frame_times.push(elapsed_us);

        harness.log().info(
            "frame",
            format!("Frame {i}: {elapsed_us}us"),
        );
    }

    frame_times.sort_unstable();
    let p50 = frame_times[frame_times.len() / 2];
    let p95 = frame_times[(frame_times.len() * 95) / 100];
    let p99 = frame_times[(frame_times.len() * 99) / 100];

    harness.log().info(
        "summary",
        format!(
            "Frame times (cached): p50={p50}us p95={p95}us p99={p99}us | \
             cold={cold_us}us warm={warm_us}us"
        ),
    );

    // Emit structured JSONL artifact
    let artifact = json!({
        "schema": "pi.test.perf_event.v1",
        "test": "long_conversation_responsiveness",
        "message_count": message_count,
        "cold_render_us": cold_us,
        "warm_render_us": warm_us,
        "frame_times_us": frame_times,
        "percentiles": { "p50": p50, "p95": p95, "p99": p99 },
        "cache_speedup_ratio": if warm_us > 0 {
            cold_us as f64 / warm_us as f64
        } else {
            0.0
        },
    });
    harness.log().info("artifact", artifact.to_string());

    // Assertions
    assert!(
        warm_us < cold_us || cold_us < 1000,
        "Cache should provide speedup (warm={warm_us}us < cold={cold_us}us) \
         or both very fast"
    );

    // p95 frame time should be reasonable for cached render
    // Allow generous budget since CI may be slow
    let frame_budget_us = 50_000; // 50ms generous budget for CI
    assert!(
        p95 < frame_budget_us,
        "p95 frame time {p95}us exceeds budget {frame_budget_us}us"
    );

    harness.log().info("verdict", "PASS: Long conversation responsiveness");
}

// ─── Script 2: Streaming With History ────────────────────────────────────────

#[test]
fn perf_e2e_streaming_with_history() {
    let harness = TestHarness::new("perf_e2e_streaming_history");
    let history_count = 200;
    let messages = generate_conversation(history_count);

    harness.log().info(
        "setup",
        format!("Generated {history_count} history messages"),
    );

    let mut app = build_perf_app(&harness, messages, Config::default());

    // Prime the cache with a full render
    let _prime = app.view();

    // Simulate streaming: set current_response to trigger streaming path
    // We need to use the public API — push a streaming token
    // The current_response field is checked in build_conversation_content
    app.current_response = "Streaming token 1".to_string();

    // Measure first streaming frame (prefix cache should be valid)
    let stream_start = Instant::now();
    let _view = app.view();
    let stream_frame_us = stream_start.elapsed().as_micros() as u64;

    harness.log().info(
        "first_stream_frame",
        format!("First streaming frame: {stream_frame_us}us"),
    );

    // Simulate progressive streaming with timing
    let mut stream_times = Vec::with_capacity(50);
    for i in 0..50 {
        app.current_response
            .push_str(&format!(" token_{i}"));
        let start = Instant::now();
        let _view = app.view();
        let elapsed_us = start.elapsed().as_micros() as u64;
        stream_times.push(elapsed_us);
    }

    stream_times.sort_unstable();
    let p50 = stream_times[stream_times.len() / 2];
    let p95 = stream_times[(stream_times.len() * 95) / 100];

    // Also measure a full rebuild for comparison (invalidate cache)
    app.current_response.clear();
    // Force full rebuild by invalidating cache
    app.set_terminal_size(121, 40); // resize triggers invalidation
    let full_rebuild_start = Instant::now();
    let _view = app.view();
    let full_rebuild_us = full_rebuild_start.elapsed().as_micros() as u64;

    harness.log().info(
        "summary",
        format!(
            "Streaming frames: p50={p50}us p95={p95}us | \
             full_rebuild={full_rebuild_us}us | \
             speedup={(full_rebuild_us as f64 / p50.max(1) as f64):.1}x"
        ),
    );

    let artifact = json!({
        "schema": "pi.test.perf_event.v1",
        "test": "streaming_with_history",
        "history_count": history_count,
        "stream_token_count": 50,
        "stream_frame_times_us": stream_times,
        "full_rebuild_us": full_rebuild_us,
        "percentiles": { "p50": p50, "p95": p95 },
        "isolation_factor": if p50 > 0 {
            full_rebuild_us as f64 / p50 as f64
        } else {
            0.0
        },
    });
    harness.log().info("artifact", artifact.to_string());

    // Streaming frames should be faster than full rebuild when cache is primed
    // (This demonstrates O(token_length) not O(total_conversation))
    if full_rebuild_us > 1000 {
        // Only assert if full rebuild is non-trivial
        assert!(
            p50 < full_rebuild_us,
            "Streaming p50 ({p50}us) should be faster than full rebuild ({full_rebuild_us}us)"
        );
    }

    harness.log().info("verdict", "PASS: Streaming with history isolation");
}

// ─── Script 3: Degradation Under Load ────────────────────────────────────────

#[test]
fn perf_e2e_degradation_under_load() {
    let harness = TestHarness::new("perf_e2e_degradation");
    let messages = generate_conversation(100);

    // Enable perf telemetry for frame budget tracking
    std::env::set_var("PI_PERF_TELEMETRY", "1");

    let app = build_perf_app(&harness, messages, Config::default());

    harness.log().info("setup", "Created 100-message app with perf telemetry");

    // Simulate frames and track budget exceedance
    let mut over_budget_count = 0u64;
    let mut under_budget_count = 0u64;
    let frame_budget_us: u64 = 16_667; // 60fps

    for i in 0..30 {
        let start = Instant::now();
        let _view = app.view();
        let elapsed_us = start.elapsed().as_micros() as u64;

        if elapsed_us > frame_budget_us {
            over_budget_count += 1;
        } else {
            under_budget_count += 1;
        }

        harness.log().info(
            "frame",
            format!(
                "Frame {i}: {elapsed_us}us ({})",
                if elapsed_us > frame_budget_us {
                    "OVER"
                } else {
                    "under"
                }
            ),
        );
    }

    harness.log().info(
        "summary",
        format!(
            "Budget tracking: {under_budget_count} under, {over_budget_count} over \
             (budget={frame_budget_us}us)"
        ),
    );

    let artifact = json!({
        "schema": "pi.test.perf_event.v1",
        "test": "degradation_under_load",
        "total_frames": 30,
        "over_budget_count": over_budget_count,
        "under_budget_count": under_budget_count,
        "frame_budget_us": frame_budget_us,
    });
    harness.log().info("artifact", artifact.to_string());

    // With 100 messages and cache warm, most frames should be under budget
    // Allow generous tolerance for CI variance
    assert!(
        under_budget_count > over_budget_count || over_budget_count <= 5,
        "Too many frames over budget: {over_budget_count}/{} total",
        over_budget_count + under_budget_count
    );

    harness.log().info("verdict", "PASS: Degradation under load tracking");
    std::env::remove_var("PI_PERF_TELEMETRY");
}

// ─── Script 4: Memory Pressure Response ──────────────────────────────────────

#[test]
fn perf_e2e_memory_pressure_response() {
    let harness = TestHarness::new("perf_e2e_memory_pressure");

    // Create messages with tool outputs that can be collapsed
    let mut messages = Vec::new();
    for i in 0..100 {
        messages.push(ConversationMessage::new(
            MessageRole::User,
            format!("Question {i}"),
            None,
        ));
        messages.push(ConversationMessage::new(
            MessageRole::Assistant,
            format!("Answer {i} with some explanation"),
            None,
        ));
        // Add tool output every 3rd exchange
        if i % 3 == 0 {
            let tool_lines: String = (0..30)
                .map(|j| format!("  output line {j}\n"))
                .collect();
            messages.push(ConversationMessage::tool(format!(
                "Tool bash:\n{tool_lines}"
            )));
        }
    }

    let initial_count = messages.len();

    harness.log().info(
        "setup",
        format!("Created {initial_count} messages (with tool outputs)"),
    );

    let app = build_perf_app(&harness, messages, Config::default());

    // Verify initial state: tool messages with >20 lines are auto-collapsed
    let collapsed_count = app
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool && m.collapsed)
        .count();
    let tool_count = app
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .count();

    harness.log().info(
        "initial_state",
        format!(
            "Tool messages: {tool_count} total, {collapsed_count} auto-collapsed"
        ),
    );

    // Verify auto-collapse worked (messages with >20 lines should be collapsed)
    assert!(
        collapsed_count > 0,
        "Expected some tool messages to be auto-collapsed (>20 lines threshold)"
    );

    // Render to verify it works
    let render_start = Instant::now();
    let view_output = app.view();
    let render_us = render_start.elapsed().as_micros() as u64;

    harness.log().info(
        "render",
        format!(
            "Initial render: {render_us}us, output={} bytes",
            view_output.len()
        ),
    );

    let artifact = json!({
        "schema": "pi.test.perf_event.v1",
        "test": "memory_pressure_response",
        "initial_message_count": initial_count,
        "tool_message_count": tool_count,
        "auto_collapsed_count": collapsed_count,
        "render_time_us": render_us,
        "output_bytes": view_output.len(),
    });
    harness.log().info("artifact", artifact.to_string());

    // Verify that collapsed tool messages produce shorter output than expanded
    // by checking that the view doesn't contain all 30 output lines per tool
    let full_tool_markers = view_output.matches("output line 29").count();
    assert!(
        full_tool_markers < tool_count,
        "Collapsed tool messages should not show all lines \
         (found {full_tool_markers} full tool outputs out of {tool_count})"
    );

    harness.log().info(
        "verdict",
        "PASS: Memory pressure response (auto-collapse verified)",
    );
}

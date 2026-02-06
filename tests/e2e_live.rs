//! **Live E2E integration tests** — hit real provider APIs.
//!
//! These tests are gated behind `PI_E2E_TESTS=1` so they never run in normal
//! `cargo test`.  They exercise the full streaming pipeline using real API keys
//! from `~/.pi/agent/models.json`.
//!
//! # Running
//!
//! ```bash
//! PI_E2E_TESTS=1 cargo test e2e_live -- --nocapture
//! PI_E2E_TESTS=1 cargo test e2e_live::anthropic -- --nocapture   # single provider
//! ```
//!
//! # Cost control
//!
//! Every prompt is deliberately tiny ("Say just the word hello") so each call
//! uses ≈20–50 tokens.  Estimated total cost for running the full suite once
//! against all six providers: < $0.01.

mod common;

use common::TestHarness;
use futures::StreamExt;
use pi::model::{Message, StopReason, StreamEvent, UserContent, UserMessage};
use pi::provider::{Context, Provider, StreamOptions};
use pi::providers::normalize_openai_base;
use std::env;
use std::path::PathBuf;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Gate: skip entire module unless PI_E2E_TESTS=1
// ---------------------------------------------------------------------------

fn e2e_enabled() -> bool {
    env::var("PI_E2E_TESTS").is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
}

macro_rules! skip_unless_e2e {
    () => {
        if !e2e_enabled() {
            eprintln!("SKIPPED (set PI_E2E_TESTS=1 to run)");
            return;
        }
    };
}

// ---------------------------------------------------------------------------
// API key loading from ~/.pi/agent/models.json
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct ApiKeys {
    anthropic: Option<String>,
    openai: Option<String>,
    google: Option<String>,
    openrouter: Option<String>,
    xai: Option<String>,
    deepseek: Option<String>,
}

fn load_api_keys() -> ApiKeys {
    let path = models_json_path();
    let mut keys = ApiKeys::default();

    let Ok(content) = std::fs::read_to_string(&path) else {
        eprintln!("  models.json not found at {}", path.display());
        return keys;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        eprintln!("  models.json: invalid JSON");
        return keys;
    };

    let providers = json.get("providers").and_then(|p| p.as_object());
    let Some(providers) = providers else {
        return keys;
    };

    fn extract_key(
        providers: &serde_json::Map<String, serde_json::Value>,
        name: &str,
    ) -> Option<String> {
        providers
            .get(name)
            .and_then(|p| p.get("apiKey"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    }

    keys.anthropic = extract_key(providers, "anthropic");
    keys.openai = extract_key(providers, "openai");
    keys.google = extract_key(providers, "google");
    keys.openrouter = extract_key(providers, "openrouter");
    keys.xai = extract_key(providers, "xai");
    keys.deepseek = extract_key(providers, "deepseek");

    keys
}

fn models_json_path() -> PathBuf {
    // Respect PI_CODING_AGENT_DIR if set, else ~/.pi/agent
    let agent_dir = env::var("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".pi/agent")
        });
    agent_dir.join("models.json")
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn user_text(text: &str) -> Message {
    Message::User(UserMessage {
        content: UserContent::Text(text.to_string()),
        timestamp: 0,
    })
}

fn simple_context(prompt: &str) -> Context {
    Context {
        system_prompt: Some("You are a test harness. Respond concisely.".to_string()),
        messages: vec![user_text(prompt)],
        tools: vec![],
    }
}

/// Build stream options with API key set via `api_key` field.
/// Providers read this field and construct the appropriate auth header themselves.
fn simple_options(api_key: &str) -> StreamOptions {
    StreamOptions {
        api_key: Some(api_key.to_string()),
        max_tokens: Some(64),
        temperature: Some(0.0),
        ..Default::default()
    }
}

/// Collect all stream events from a provider, logging each one.
async fn collect_stream(
    provider: &dyn Provider,
    context: &Context,
    options: &StreamOptions,
    harness: &TestHarness,
) -> (Vec<StreamEvent>, Option<String>) {
    let start = Instant::now();
    harness
        .log()
        .info_ctx("stream", "Starting provider stream", |ctx| {
            ctx.push(("provider".into(), provider.name().to_string()));
            ctx.push(("model".into(), provider.model_id().to_string()));
            ctx.push(("api".into(), provider.api().to_string()));
        });

    let stream_result = provider.stream(context, options).await;
    let elapsed_connect = start.elapsed();

    harness.log().info_ctx("stream", "Stream opened", |ctx| {
        ctx.push((
            "connect_ms".into(),
            format!("{}", elapsed_connect.as_millis()),
        ));
        ctx.push(("ok".into(), format!("{}", stream_result.is_ok())));
    });

    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("{e}");
            harness
                .log()
                .error("stream", format!("Stream error: {msg}"));
            return (vec![], Some(msg));
        }
    };

    let mut events = Vec::new();
    let mut text_accum = String::new();
    let mut stream_error = None;
    let mut event_count = 0u32;

    while let Some(item) = stream.next().await {
        event_count += 1;
        match item {
            Ok(event) => {
                match &event {
                    StreamEvent::TextDelta { delta, .. } => text_accum.push_str(delta),
                    StreamEvent::Done { reason, message } => {
                        harness.log().info_ctx("stream", "Stream done", |ctx| {
                            ctx.push(("stop_reason".into(), format!("{reason:?}")));
                            ctx.push(("input_tokens".into(), format!("{}", message.usage.input)));
                            ctx.push((
                                "output_tokens".into(),
                                format!("{}", message.usage.output),
                            ));
                        });
                    }
                    _ => {}
                }
                events.push(event);
            }
            Err(e) => {
                stream_error = Some(format!("{e}"));
                harness.log().error("stream", format!("Event error: {}", e));
                break;
            }
        }
    }

    let elapsed_total = start.elapsed();
    harness.log().info_ctx("stream", "Stream complete", |ctx| {
        ctx.push(("total_ms".into(), format!("{}", elapsed_total.as_millis())));
        ctx.push(("event_count".into(), format!("{event_count}")));
        ctx.push(("text_length".into(), format!("{}", text_accum.len())));
        ctx.push((
            "text_preview".into(),
            text_accum.chars().take(200).collect::<String>(),
        ));
    });

    (events, stream_error)
}

/// Assert basic streaming success: got events, no error, non-empty text.
fn assert_basic_stream_success(
    events: &[StreamEvent],
    stream_error: &Option<String>,
    harness: &TestHarness,
    test_name: &str,
) {
    if let Some(err) = stream_error {
        harness
            .log()
            .error("assert", format!("{test_name}: stream error: {err}"));
        panic!("{test_name}: unexpected stream error: {err}");
    }

    assert!(
        !events.is_empty(),
        "{test_name}: expected at least one event"
    );

    // Accept TextDelta, TextEnd, or ThinkingDelta as "content" events.
    // Some providers (e.g. Gemini 2.5) may deliver content via thinking events.
    let has_content = events.iter().any(|e| {
        matches!(
            e,
            StreamEvent::TextDelta { .. }
                | StreamEvent::TextEnd { .. }
                | StreamEvent::ThinkingDelta { .. }
        )
    });
    assert!(
        has_content,
        "{test_name}: expected at least one content event (TextDelta/TextEnd/ThinkingDelta)"
    );

    let has_done = events.iter().any(|e| matches!(e, StreamEvent::Done { .. }));
    assert!(has_done, "{test_name}: expected a Done event");

    harness.log().info("assert", format!("{test_name}: PASSED"));
}

// ---------------------------------------------------------------------------
// Anthropic E2E Tests
// ---------------------------------------------------------------------------

mod anthropic {
    use super::*;
    use pi::providers::anthropic::AnthropicProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.anthropic else {
            eprintln!("SKIPPED: no Anthropic API key");
            return;
        };
        let harness = TestHarness::new("e2e_anthropic_basic_message");

        common::run_async(async move {
            let provider = AnthropicProvider::new("claude-haiku-4-5-20251001");
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "anthropic_basic");

            // Verify text contains "hello" (case-insensitive)
            let text: String = events
                .iter()
                .filter_map(|e| match e {
                    StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
                    _ => None,
                })
                .collect();
            assert!(
                text.to_lowercase().contains("hello"),
                "Expected 'hello' in response, got: {text}"
            );
        });
    }

    #[test]
    fn streaming_event_order() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.anthropic else {
            eprintln!("SKIPPED: no Anthropic API key");
            return;
        };
        let harness = TestHarness::new("e2e_anthropic_streaming_order");

        common::run_async(async move {
            let provider = AnthropicProvider::new("claude-haiku-4-5-20251001");
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "anthropic_order");

            // Verify Start comes first
            assert!(
                matches!(events.first(), Some(StreamEvent::Start { .. })),
                "First event should be Start, got {:?}",
                events.first()
            );

            // Verify Done comes last
            assert!(
                matches!(events.last(), Some(StreamEvent::Done { .. })),
                "Last event should be Done, got {:?}",
                events.last()
            );

            // Count text deltas
            let text_count = events
                .iter()
                .filter(|e| matches!(e, StreamEvent::TextDelta { .. }))
                .count();
            harness
                .log()
                .info("verify", format!("Text deltas: {text_count}"));
            assert!(text_count >= 1, "Expected at least 1 text delta");
        });
    }

    #[test]
    fn stop_reason_end_turn() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.anthropic else {
            eprintln!("SKIPPED: no Anthropic API key");
            return;
        };
        let harness = TestHarness::new("e2e_anthropic_stop_reason");

        common::run_async(async move {
            let provider = AnthropicProvider::new("claude-haiku-4-5-20251001");
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, _) = collect_stream(&provider, &context, &options, &harness).await;

            let done = events.iter().find_map(|e| match e {
                StreamEvent::Done { reason, message } => Some((reason, message)),
                _ => None,
            });
            assert!(done.is_some(), "Expected Done event");
            let (reason, message) = done.unwrap();
            assert_eq!(*reason, StopReason::Stop, "Expected Stop reason");
            assert!(message.usage.input > 0, "Expected non-zero input tokens");
            assert!(message.usage.output > 0, "Expected non-zero output tokens");
        });
    }
}

// ---------------------------------------------------------------------------
// OpenAI E2E Tests (Responses API)
// ---------------------------------------------------------------------------

mod openai {
    use super::*;
    use pi::providers::openai_responses::OpenAIResponsesProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.openai else {
            eprintln!("SKIPPED: no OpenAI API key");
            return;
        };
        let harness = TestHarness::new("e2e_openai_basic_message");

        common::run_async(async move {
            let provider = OpenAIResponsesProvider::new("gpt-4o-mini");
            let context = simple_context("Say just the word hello");
            // Use simple_options: provider reads api_key and builds Authorization header itself.
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "openai_basic");
        });
    }

    #[test]
    fn streaming_events() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.openai else {
            eprintln!("SKIPPED: no OpenAI API key");
            return;
        };
        let harness = TestHarness::new("e2e_openai_streaming");

        common::run_async(async move {
            let provider = OpenAIResponsesProvider::new("gpt-4o-mini");
            let context = simple_context("Count from 1 to 5, one number per line");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "openai_streaming");

            let text: String = events
                .iter()
                .filter_map(|e| match e {
                    StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
                    _ => None,
                })
                .collect();
            // Should contain at least "1" and "5"
            assert!(text.contains('1'), "Expected '1' in response");
            assert!(text.contains('5'), "Expected '5' in response");
        });
    }
}

// ---------------------------------------------------------------------------
// Google Gemini E2E Tests
// ---------------------------------------------------------------------------

mod gemini {
    use super::*;
    use pi::providers::gemini::GeminiProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.google else {
            eprintln!("SKIPPED: no Google API key");
            return;
        };
        let harness = TestHarness::new("e2e_gemini_basic_message");

        common::run_async(async move {
            // Use gemini-2.0-flash: the 2.5 models may emit thinking-only responses
            // for simple prompts, which our harness correctly handles.
            let provider = GeminiProvider::new("gemini-2.0-flash");
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "gemini_basic");
        });
    }

    #[test]
    fn streaming_events() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.google else {
            eprintln!("SKIPPED: no Google API key");
            return;
        };
        let harness = TestHarness::new("e2e_gemini_streaming");

        common::run_async(async move {
            let provider = GeminiProvider::new("gemini-2.0-flash");
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "gemini_streaming");

            // Verify we got Start event
            let has_start = events
                .iter()
                .any(|e| matches!(e, StreamEvent::Start { .. }));
            assert!(has_start, "Expected Start event from Gemini");
        });
    }
}

// ---------------------------------------------------------------------------
// OpenRouter E2E Tests (OpenAI-compat)
// ---------------------------------------------------------------------------

mod openrouter {
    use super::*;
    use pi::providers::openai::OpenAIProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.openrouter else {
            eprintln!("SKIPPED: no OpenRouter API key");
            return;
        };
        let harness = TestHarness::new("e2e_openrouter_basic_message");

        common::run_async(async move {
            // normalize_openai_base appends /chat/completions to the base URL
            let provider = OpenAIProvider::new("deepseek/deepseek-chat")
                .with_base_url(normalize_openai_base("https://openrouter.ai/api/v1"));
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "openrouter_basic");
        });
    }
}

// ---------------------------------------------------------------------------
// xAI/Grok E2E Tests (OpenAI-compat)
// ---------------------------------------------------------------------------

mod xai {
    use super::*;
    use pi::providers::openai::OpenAIProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.xai else {
            eprintln!("SKIPPED: no xAI API key");
            return;
        };
        let harness = TestHarness::new("e2e_xai_basic_message");

        common::run_async(async move {
            let provider = OpenAIProvider::new("grok-3-mini")
                .with_base_url(normalize_openai_base("https://api.x.ai/v1"));
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "xai_basic");
        });
    }
}

// ---------------------------------------------------------------------------
// DeepSeek E2E Tests (OpenAI-compat)
// ---------------------------------------------------------------------------

mod deepseek {
    use super::*;
    use pi::providers::openai::OpenAIProvider;

    #[test]
    fn basic_message() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let Some(api_key) = keys.deepseek else {
            eprintln!("SKIPPED: no DeepSeek API key");
            return;
        };
        let harness = TestHarness::new("e2e_deepseek_basic_message");

        common::run_async(async move {
            let provider = OpenAIProvider::new("deepseek-chat")
                .with_base_url(normalize_openai_base("https://api.deepseek.com/v1"));
            let context = simple_context("Say just the word hello");
            let options = simple_options(&api_key);

            let (events, error) = collect_stream(&provider, &context, &options, &harness).await;
            assert_basic_stream_success(&events, &error, &harness, "deepseek_basic");
        });
    }
}

// ---------------------------------------------------------------------------
// Cross-provider comparison
// ---------------------------------------------------------------------------

mod cross_provider {
    use super::*;
    use pi::providers::anthropic::AnthropicProvider;
    use pi::providers::gemini::GeminiProvider;
    use pi::providers::openai_responses::OpenAIResponsesProvider;

    #[test]
    fn all_available_providers_respond() {
        skip_unless_e2e!();
        let keys = load_api_keys();
        let harness = TestHarness::new("e2e_cross_provider_all_respond");

        common::run_async(async move {
            let prompt = "Say just the word hello";
            let mut results: Vec<(&str, bool, u128)> = Vec::new();

            // Anthropic
            if let Some(ref api_key) = keys.anthropic {
                let provider = AnthropicProvider::new("claude-haiku-4-5-20251001");
                let start = Instant::now();
                let (events, error) = collect_stream(
                    &provider,
                    &simple_context(prompt),
                    &simple_options(api_key),
                    &harness,
                )
                .await;
                let ms = start.elapsed().as_millis();
                let ok = error.is_none() && !events.is_empty();
                results.push(("anthropic", ok, ms));
            }

            // OpenAI
            if let Some(ref api_key) = keys.openai {
                let provider = OpenAIResponsesProvider::new("gpt-4o-mini");
                let start = Instant::now();
                let (events, error) = collect_stream(
                    &provider,
                    &simple_context(prompt),
                    &simple_options(api_key),
                    &harness,
                )
                .await;
                let ms = start.elapsed().as_millis();
                let ok = error.is_none() && !events.is_empty();
                results.push(("openai", ok, ms));
            }

            // Gemini
            if let Some(ref api_key) = keys.google {
                let provider = GeminiProvider::new("gemini-2.0-flash");
                let start = Instant::now();
                let (events, error) = collect_stream(
                    &provider,
                    &simple_context(prompt),
                    &simple_options(api_key),
                    &harness,
                )
                .await;
                let ms = start.elapsed().as_millis();
                let ok = error.is_none() && !events.is_empty();
                results.push(("gemini", ok, ms));
            }

            // Log summary table
            harness
                .log()
                .info("summary", "=== Cross-Provider Results ===");
            for (name, ok, ms) in &results {
                let status = if *ok { "PASS" } else { "FAIL" };
                harness
                    .log()
                    .info_ctx("summary", format!("{name}: {status}"), |ctx| {
                        ctx.push(("latency_ms".into(), format!("{ms}")));
                    });
            }

            let all_passed = results.iter().all(|(_, ok, _)| *ok);
            assert!(all_passed, "Not all providers succeeded: {:?}", results);
            assert!(
                !results.is_empty(),
                "No providers were available for testing"
            );
        });
    }
}

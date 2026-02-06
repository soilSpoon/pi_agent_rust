//! Live provider E2E harness (real APIs, short prompt, rich JSONL logging).
//!
//! This test is intentionally gated behind `CI_E2E_TESTS=1` to avoid unexpected
//! network usage during normal `cargo test`.

mod common;

use common::{
    LIVE_SHORT_PROMPT, LiveE2eRegistry, LiveProviderTarget, TestHarness, ci_e2e_tests_enabled,
    run_live_provider_target, write_live_provider_runs_jsonl,
};

const LIVE_TARGETS: [LiveProviderTarget; 6] = [
    LiveProviderTarget::new(
        "anthropic",
        "ANTHROPIC_TEST_MODEL",
        &[
            "claude-haiku-4-5",
            "claude-3-5-haiku-20241022",
            "claude-sonnet-4-5",
        ],
        LIVE_SHORT_PROMPT,
    ),
    LiveProviderTarget::new(
        "openai",
        "OPENAI_TEST_MODEL",
        &["gpt-4o-mini", "gpt-4o", "gpt-5.1-codex"],
        LIVE_SHORT_PROMPT,
    ),
    LiveProviderTarget::new(
        "google",
        "GOOGLE_TEST_MODEL",
        &["gemini-2.5-flash", "gemini-1.5-flash", "gemini-2.5-pro"],
        LIVE_SHORT_PROMPT,
    ),
    LiveProviderTarget::new(
        "openrouter",
        "OPENROUTER_TEST_MODEL",
        &[],
        LIVE_SHORT_PROMPT,
    ),
    LiveProviderTarget::new("xai", "XAI_TEST_MODEL", &[], LIVE_SHORT_PROMPT),
    LiveProviderTarget::new("deepseek", "DEEPSEEK_TEST_MODEL", &[], LIVE_SHORT_PROMPT),
];

#[test]
fn e2e_live_provider_harness_smoke() {
    let harness = TestHarness::new("e2e_live_provider_harness_smoke");

    if !ci_e2e_tests_enabled() {
        harness.log().warn(
            "live_e2e",
            "Skipping live provider E2E harness (set CI_E2E_TESTS=1 to enable)",
        );
        return;
    }

    let registry = LiveE2eRegistry::load(harness.log())
        .unwrap_or_else(|err| panic!("failed to load live E2E registry: {err}"));

    asupersync::test_utils::run_test(|| {
        let harness_ref = &harness;
        let registry = registry.clone();
        async move {
            let vcr_dir = harness_ref.temp_path("live_provider_vcr");
            std::fs::create_dir_all(&vcr_dir)
                .unwrap_or_else(|err| panic!("create live provider vcr dir: {err}"));

            let mut runs = Vec::with_capacity(LIVE_TARGETS.len());
            for target in LIVE_TARGETS {
                let run = run_live_provider_target(harness_ref, &registry, &target, &vcr_dir).await;
                runs.push(run);
            }

            write_live_provider_runs_jsonl(harness_ref, "live_provider_results.jsonl", &runs)
                .unwrap_or_else(|err| panic!("write live provider results jsonl: {err}"));

            let log_path = harness_ref.temp_path("live_provider_log.jsonl");
            harness_ref
                .write_jsonl_logs(&log_path)
                .unwrap_or_else(|err| panic!("write live provider JSONL log: {err}"));
            harness_ref.record_artifact("live_provider_log.jsonl", &log_path);

            let artifact_path = harness_ref.temp_path("live_provider_artifacts.jsonl");
            harness_ref
                .write_artifact_index_jsonl(&artifact_path)
                .unwrap_or_else(|err| panic!("write live provider artifact index: {err}"));
            harness_ref.record_artifact("live_provider_artifacts.jsonl", &artifact_path);

            let attempted = runs.iter().filter(|run| run.status != "skipped").count();
            let passed = runs.iter().filter(|run| run.status == "passed").count();
            let skipped = runs.iter().filter(|run| run.status == "skipped").count();
            let failed: Vec<String> = runs
                .iter()
                .filter(|run| run.status == "failed")
                .map(|run| {
                    format!(
                        "{}/{} ({})",
                        run.provider,
                        run.model.as_deref().unwrap_or("<none>"),
                        run.error.as_deref().unwrap_or("unknown error"),
                    )
                })
                .collect();

            harness_ref
                .log()
                .info_ctx("live_e2e", "Live harness suite summary", |ctx| {
                    ctx.push(("targets".into(), LIVE_TARGETS.len().to_string()));
                    ctx.push(("attempted".into(), attempted.to_string()));
                    ctx.push(("passed".into(), passed.to_string()));
                    ctx.push(("skipped".into(), skipped.to_string()));
                    ctx.push(("failed".into(), failed.len().to_string()));
                });

            assert!(
                attempted > 0,
                "CI_E2E_TESTS=1 but no providers were runnable. Ensure ~/.pi/agent/models.json and API keys are configured."
            );
            assert!(
                failed.is_empty(),
                "live provider harness failures: {}",
                failed.join("; ")
            );
        }
    });
}

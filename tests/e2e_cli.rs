//! End-to-end CLI tests (offline).
//!
//! These tests invoke the compiled `pi` binary directly and verify that
//! offline flags/subcommands behave as expected, with verbose logging
//! and artifact capture for debugging failures.

mod common;

use common::TestHarness;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

struct CliResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration: Duration,
}

struct CliTestHarness {
    harness: TestHarness,
    binary_path: PathBuf,
    env: BTreeMap<String, String>,
}

impl CliTestHarness {
    fn new(name: &str) -> Self {
        let harness = TestHarness::new(name);
        let binary_path = PathBuf::from(env!("CARGO_BIN_EXE_pi"));

        let mut env = BTreeMap::new();

        let env_root = harness.temp_path("pi-env");
        let _ = std::fs::create_dir_all(&env_root);

        // Fully isolate global/project state for determinism.
        env.insert(
            "PI_CODING_AGENT_DIR".to_string(),
            env_root.join("agent").display().to_string(),
        );
        env.insert(
            "PI_CONFIG_PATH".to_string(),
            env_root.join("settings.json").display().to_string(),
        );
        env.insert(
            "PI_SESSIONS_DIR".to_string(),
            env_root.join("sessions").display().to_string(),
        );
        env.insert(
            "PI_PACKAGE_DIR".to_string(),
            env_root.join("packages").display().to_string(),
        );

        Self {
            harness,
            binary_path,
            env,
        }
    }

    fn run(&self, args: &[&str]) -> CliResult {
        self.harness
            .log()
            .info("action", format!("Running CLI: {}", args.join(" ")));
        self.harness.log().info_ctx("action", "CLI env", |ctx| {
            for (key, value) in &self.env {
                ctx.push((key.clone(), value.clone()));
            }
        });

        let start = Instant::now();
        let output = Command::new(&self.binary_path)
            .args(args)
            .envs(self.env.clone())
            .current_dir(self.harness.temp_dir())
            .output()
            .expect("run pi");
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        self.harness
            .log()
            .info_ctx("result", "CLI completed", |ctx| {
                ctx.push(("exit_code".to_string(), exit_code.to_string()));
                ctx.push(("duration_ms".to_string(), duration.as_millis().to_string()));
                ctx.push(("stdout_len".to_string(), stdout.len().to_string()));
                ctx.push(("stderr_len".to_string(), stderr.len().to_string()));
            });

        let stdout_path = self.harness.temp_path("stdout.txt");
        let stderr_path = self.harness.temp_path("stderr.txt");
        let _ = std::fs::write(&stdout_path, &stdout);
        let _ = std::fs::write(&stderr_path, &stderr);
        self.harness.record_artifact("stdout.txt", &stdout_path);
        self.harness.record_artifact("stderr.txt", &stderr_path);

        CliResult {
            exit_code,
            stdout,
            stderr,
            duration,
        }
    }
}

fn assert_contains(harness: &TestHarness, haystack: &str, needle: &str) {
    harness.assert_log(format!("assert contains: {needle}").as_str());
    assert!(
        haystack.contains(needle),
        "expected output to contain '{needle}'"
    );
}

fn assert_contains_case_insensitive(harness: &TestHarness, haystack: &str, needle: &str) {
    harness.assert_log(format!("assert contains (ci): {needle}").as_str());
    assert!(
        haystack.to_lowercase().contains(&needle.to_lowercase()),
        "expected output to contain (case-insensitive) '{needle}'"
    );
}

fn assert_exit_code(harness: &TestHarness, result: &CliResult, expected: i32) {
    harness.assert_log(format!("assert exit_code == {expected}").as_str());
    assert_eq!(result.exit_code, expected);
}

#[test]
fn e2e_cli_extension_compat_ledger_logged_when_enabled() {
    let mut harness = CliTestHarness::new("e2e_cli_extension_compat_ledger_logged_when_enabled");
    harness
        .env
        .insert("PI_EXT_COMPAT_SCAN".to_string(), "1".to_string());
    harness
        .env
        .insert("RUST_LOG".to_string(), "info".to_string());

    let ext_path = harness.harness.temp_path("ext.ts");
    std::fs::write(
        &ext_path,
        "import fs from 'fs';\nimport { spawn } from 'child_process';\npi.tool('read', { path: 'README.md' });\nnew Function('return 1');\neval('1');\n",
    )
    .expect("write ext.ts");

    let ext_arg = ext_path.display().to_string();
    let result = harness.run(&["--list-models", "--extension", ext_arg.as_str()]);

    assert_exit_code(&harness.harness, &result, 0);
    let combined = format!("{}\n{}", result.stdout, result.stderr);
    assert_contains(&harness.harness, &combined, "pi.ext.compat_ledger.v1");
}

#[test]
fn e2e_cli_version_flag() {
    let harness = CliTestHarness::new("e2e_cli_version_flag");
    let result = harness.run(&["--version"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains(&harness.harness, &result.stdout, "pi ");
    assert_contains(&harness.harness, &result.stdout, env!("CARGO_PKG_VERSION"));
    assert_contains(&harness.harness, &result.stdout, "\n");
}

#[test]
fn e2e_cli_help_flag() {
    let harness = CliTestHarness::new("e2e_cli_help_flag");
    let result = harness.run(&["--help"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains_case_insensitive(&harness.harness, &result.stdout, "usage");
    assert_contains(&harness.harness, &result.stdout, "pi");
}

#[test]
fn e2e_cli_invalid_flag_is_error() {
    let harness = CliTestHarness::new("e2e_cli_invalid_flag_is_error");
    let result = harness.run(&["--invalid-flag"]);

    harness
        .harness
        .assert_log("assert exit_code != 0 for invalid flag");
    assert_ne!(result.exit_code, 0);
    assert_contains_case_insensitive(&harness.harness, &result.stderr, "error");
}

#[test]
fn e2e_cli_config_subcommand_prints_paths() {
    let harness = CliTestHarness::new("e2e_cli_config_subcommand_prints_paths");
    let result = harness.run(&["config"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains(&harness.harness, &result.stdout, "Settings paths:");
    assert_contains(&harness.harness, &result.stdout, "Global:");
    assert_contains(&harness.harness, &result.stdout, "Project:");
    assert_contains(&harness.harness, &result.stdout, "Sessions:");
}

#[test]
fn e2e_cli_config_paths_honor_env_overrides() {
    let mut harness = CliTestHarness::new("e2e_cli_config_paths_honor_env_overrides");

    let env_root = harness.harness.temp_path("env-overrides");
    let agent_dir = env_root.join("agent-root");
    let config_path = env_root.join("settings-override.json");
    let sessions_dir = env_root.join("sessions-root");
    let packages_dir = env_root.join("packages-root");

    std::fs::create_dir_all(&agent_dir).expect("create agent dir");
    std::fs::write(&config_path, "{}").expect("write override settings");

    harness.env.insert(
        "PI_CODING_AGENT_DIR".to_string(),
        agent_dir.display().to_string(),
    );
    harness.env.insert(
        "PI_CONFIG_PATH".to_string(),
        config_path.display().to_string(),
    );
    harness.env.insert(
        "PI_SESSIONS_DIR".to_string(),
        sessions_dir.display().to_string(),
    );
    harness.env.insert(
        "PI_PACKAGE_DIR".to_string(),
        packages_dir.display().to_string(),
    );

    let result = harness.run(&["config"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Global:  {}", config_path.display()),
    );
    let project_path = harness.harness.temp_dir().join(".pi/settings.json");
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Project: {}", project_path.display()),
    );
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Sessions: {}", sessions_dir.display()),
    );
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Auth:     {}", agent_dir.join("auth.json").display()),
    );
}

#[test]
fn e2e_cli_config_paths_fallback_to_agent_dir() {
    let mut harness = CliTestHarness::new("e2e_cli_config_paths_fallback_to_agent_dir");

    let env_root = harness.harness.temp_path("env-fallback");
    let agent_dir = env_root.join("agent-root");
    std::fs::create_dir_all(&agent_dir).expect("create agent dir");

    harness.env.insert(
        "PI_CODING_AGENT_DIR".to_string(),
        agent_dir.display().to_string(),
    );
    harness.env.remove("PI_CONFIG_PATH");
    harness.env.remove("PI_SESSIONS_DIR");
    harness.env.remove("PI_PACKAGE_DIR");

    let result = harness.run(&["config"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Global:  {}", agent_dir.join("settings.json").display()),
    );
    let project_path = harness.harness.temp_dir().join(".pi/settings.json");
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Project: {}", project_path.display()),
    );
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Sessions: {}", agent_dir.join("sessions").display()),
    );
    assert_contains(
        &harness.harness,
        &result.stdout,
        &format!("Auth:     {}", agent_dir.join("auth.json").display()),
    );
}

#[test]
fn e2e_cli_list_subcommand_works_offline() {
    let harness = CliTestHarness::new("e2e_cli_list_subcommand_works_offline");
    let result = harness.run(&["list"]);

    assert_exit_code(&harness.harness, &result, 0);
    assert_contains_case_insensitive(&harness.harness, &result.stdout, "packages");
}

#[test]
fn e2e_cli_version_is_fast_enough_for_test_env() {
    let harness = CliTestHarness::new("e2e_cli_version_is_fast_enough_for_test_env");
    let result = harness.run(&["--version"]);

    assert_exit_code(&harness.harness, &result, 0);

    // Avoid hard <100ms assertions in CI; we only enforce that the CLI isn't hanging.
    harness.harness.assert_log("assert duration < 5s (sanity)");
    assert!(result.duration < Duration::from_secs(5));
}

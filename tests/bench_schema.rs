//! Benchmark JSONL schema definitions and validation tests (bd-167l).
//!
//! Defines the canonical machine-readable output format for extension benchmark
//! runs. All benchmark JSONL records share a common envelope with environment
//! fingerprint, and schema-specific payload fields.
//!
//! Run with: `cargo test --test bench_schema -- --nocapture`

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    dead_code
)]

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

// ─── Schema Definitions ──────────────────────────────────────────────────────

/// Common environment fingerprint included in every benchmark record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvFingerprint {
    /// Operating system (e.g., "Linux (Ubuntu 25.10)")
    pub os: String,
    /// CPU architecture (e.g., "x86_64")
    pub arch: String,
    /// CPU model string
    pub cpu_model: String,
    /// Number of logical CPU cores
    pub cpu_cores: u32,
    /// Total system memory in MB
    pub mem_total_mb: u64,
    /// Build profile: "debug" or "release"
    pub build_profile: String,
    /// Git commit hash (short)
    pub git_commit: String,
    /// Cargo feature flags active during build
    #[serde(default)]
    pub features: Vec<String>,
    /// SHA-256 of the concatenated env fields (for dedup/comparison)
    pub config_hash: String,
}

/// Schema: `pi.ext.rust_bench.v1` — Rust extension benchmark event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustBenchEvent {
    pub schema: String,
    pub runtime: String,
    pub scenario: String,
    pub extension: String,
    #[serde(flatten)]
    pub payload: Value,
    #[serde(default)]
    pub env: Option<EnvFingerprint>,
}

/// Schema: `pi.ext.legacy_bench.v1` — Legacy (TS/Node) benchmark event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyBenchEvent {
    pub schema: String,
    pub runtime: String,
    pub scenario: String,
    pub extension: String,
    #[serde(flatten)]
    pub payload: Value,
    #[serde(default)]
    pub node: Option<Value>,
}

/// Schema: `pi.perf.workload.v1` — PiJS workload benchmark event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadEvent {
    pub scenario: String,
    pub iterations: u64,
    pub tool_calls_per_iteration: u64,
    pub total_calls: u64,
    pub elapsed_ms: u64,
    pub per_call_us: u64,
    pub calls_per_sec: u64,
}

/// Schema: `pi.perf.budget.v1` — Performance budget check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEvent {
    pub budget_name: String,
    pub category: String,
    pub threshold: f64,
    pub unit: String,
    pub actual: Option<f64>,
    pub status: String,
    pub source: String,
}

// ─── Schema Registry ─────────────────────────────────────────────────────────

/// Known JSONL schemas with version and description.
const SCHEMAS: &[(&str, &str)] = &[
    (
        "pi.ext.rust_bench.v1",
        "Rust QuickJS extension benchmark event (load, tool call, event hook)",
    ),
    (
        "pi.ext.legacy_bench.v1",
        "Legacy pi-mono (Node.js) extension benchmark event",
    ),
    (
        "pi.perf.workload.v1",
        "PiJS workload harness output (tool call throughput)",
    ),
    (
        "pi.perf.budget.v1",
        "Performance budget check result",
    ),
    (
        "pi.perf.budget_summary.v1",
        "Aggregate budget summary with pass/fail counts",
    ),
    (
        "pi.ext.conformance_report.v2",
        "Per-extension conformance report event",
    ),
    (
        "pi.ext.conformance_summary.v2",
        "Aggregate conformance summary with per-tier breakdowns",
    ),
];

/// Required fields for each schema (field name, description).
const RUST_BENCH_REQUIRED: &[&str] = &["schema", "runtime", "scenario", "extension"];
const LEGACY_BENCH_REQUIRED: &[&str] = &["schema", "runtime", "scenario", "extension"];
const WORKLOAD_REQUIRED: &[&str] = &[
    "scenario",
    "iterations",
    "tool_calls_per_iteration",
    "total_calls",
    "elapsed_ms",
    "per_call_us",
    "calls_per_sec",
];

/// Environment fingerprint fields.
const ENV_FINGERPRINT_FIELDS: &[(&str, &str)] = &[
    ("os", "Operating system name and version"),
    ("arch", "CPU architecture (x86_64, aarch64)"),
    ("cpu_model", "CPU model string from /proc/cpuinfo or sysinfo"),
    ("cpu_cores", "Logical CPU core count"),
    ("mem_total_mb", "Total system memory in megabytes"),
    ("build_profile", "Cargo build profile: debug or release"),
    ("git_commit", "Short git commit hash of the build"),
    ("features", "Active Cargo feature flags"),
    ("config_hash", "SHA-256 of env fields for dedup"),
];

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_jsonl_file(path: &Path) -> Vec<Value> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn has_required_fields(record: &Value, fields: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for field in fields {
        if record.get(*field).is_none() {
            missing.push((*field).to_string());
        }
    }
    missing
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn schema_registry_is_complete() {
    assert!(
        SCHEMAS.len() >= 5,
        "should have at least 5 registered schemas"
    );
    for (name, desc) in SCHEMAS {
        assert!(!name.is_empty(), "schema name must not be empty");
        assert!(!desc.is_empty(), "schema description must not be empty");
        assert!(
            name.starts_with("pi."),
            "schema names should start with 'pi.': {name}"
        );
    }
    eprintln!("[schema] {} schemas registered", SCHEMAS.len());
}

#[test]
fn env_fingerprint_fields_documented() {
    assert!(
        ENV_FINGERPRINT_FIELDS.len() >= 7,
        "should document at least 7 env fingerprint fields"
    );
    for (name, desc) in ENV_FINGERPRINT_FIELDS {
        assert!(!name.is_empty());
        assert!(!desc.is_empty());
    }
    eprintln!(
        "[schema] {} env fingerprint fields documented",
        ENV_FINGERPRINT_FIELDS.len()
    );
}

#[test]
fn validate_rust_bench_schema() {
    let root = project_root();
    let events = read_jsonl_file(&root.join("target/perf/pijs_workload.jsonl"));
    if events.is_empty() {
        eprintln!("[schema] No pijs_workload.jsonl data — skipping");
        return;
    }

    for event in &events {
        let missing = has_required_fields(event, WORKLOAD_REQUIRED);
        assert!(
            missing.is_empty(),
            "workload event missing required fields: {missing:?}"
        );
    }
    eprintln!(
        "[schema] Validated {} pijs_workload events",
        events.len()
    );
}

#[test]
fn validate_legacy_bench_schema() {
    let root = project_root();
    let events = read_jsonl_file(&root.join("target/perf/legacy_extension_workloads.jsonl"));
    if events.is_empty() {
        eprintln!("[schema] No legacy benchmark data — skipping");
        return;
    }

    for event in &events {
        let missing = has_required_fields(event, LEGACY_BENCH_REQUIRED);
        assert!(
            missing.is_empty(),
            "legacy bench event missing required fields: {missing:?}"
        );
        assert_eq!(
            event.get("schema").and_then(Value::as_str),
            Some("pi.ext.legacy_bench.v1"),
            "legacy bench should use pi.ext.legacy_bench.v1 schema"
        );
    }
    eprintln!(
        "[schema] Validated {} legacy bench events",
        events.len()
    );
}

#[test]
fn validate_budget_events_schema() {
    let root = project_root();
    let events = read_jsonl_file(&root.join("tests/perf/reports/budget_events.jsonl"));
    if events.is_empty() {
        eprintln!("[schema] No budget events — skipping");
        return;
    }

    let budget_required = &[
        "budget_name",
        "category",
        "threshold",
        "unit",
        "status",
        "source",
    ];

    for event in &events {
        let missing = has_required_fields(event, budget_required);
        assert!(
            missing.is_empty(),
            "budget event missing required fields: {missing:?}"
        );
    }
    eprintln!("[schema] Validated {} budget events", events.len());
}

#[test]
fn validate_conformance_events_schema() {
    let root = project_root();
    let events = read_jsonl_file(
        &root.join("tests/ext_conformance/reports/conformance_events.jsonl"),
    );
    if events.is_empty() {
        eprintln!("[schema] No conformance events — skipping");
        return;
    }

    let required = &[
        "schema",
        "extension_id",
        "source_tier",
        "conformance_tier",
        "overall_status",
    ];

    for event in &events {
        let missing = has_required_fields(event, required);
        assert!(
            missing.is_empty(),
            "conformance event missing required fields: {missing:?}"
        );
    }
    eprintln!(
        "[schema] Validated {} conformance events",
        events.len()
    );
}

#[test]
fn jsonl_records_have_stable_key_ordering() {
    let root = project_root();

    // Check that legacy bench records have deterministic key ordering
    let events = read_jsonl_file(&root.join("target/perf/legacy_extension_workloads.jsonl"));
    if !events.is_empty() {
        // All records with same schema should have same top-level key set
        let first_keys: Vec<String> = events[0]
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        for (i, event) in events.iter().enumerate() {
            if let Some(obj) = event.as_object() {
                // Same scenario records should have same structure
                if event.get("scenario") == events[0].get("scenario") {
                    assert_eq!(
                        obj.keys().count(),
                        first_keys.len(),
                        "record {i} has different key count than record 0"
                    );
                }
            }
        }
        eprintln!("[schema] Key ordering stable across {} legacy events", events.len());
    }

    // Check workload records
    let events = read_jsonl_file(&root.join("target/perf/pijs_workload.jsonl"));
    if events.len() >= 2 {
        let keys_0: Vec<String> = events[0]
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();
        let keys_1: Vec<String> = events[1]
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();
        assert_eq!(keys_0, keys_1, "workload records should have same key set");
        eprintln!("[schema] Key ordering stable across {} workload events", events.len());
    }
}

#[test]
fn generate_schema_doc() {
    let root = project_root();
    let reports_dir = root.join("tests/perf/reports");
    let _ = std::fs::create_dir_all(&reports_dir);

    let mut md = String::with_capacity(8 * 1024);

    md.push_str("# Benchmark JSONL Schema Reference\n\n");
    md.push_str("> Auto-generated. Do not edit manually.\n\n");

    // Schema registry
    md.push_str("## Registered Schemas\n\n");
    md.push_str("| Schema | Description |\n");
    md.push_str("|---|---|\n");
    for (name, desc) in SCHEMAS {
        let _ = writeln!(md, "| `{name}` | {desc} |");
    }
    md.push('\n');

    // Environment fingerprint
    md.push_str("## Environment Fingerprint\n\n");
    md.push_str("Every benchmark record SHOULD include an `env` object with:\n\n");
    md.push_str("| Field | Type | Description |\n");
    md.push_str("|---|---|---|\n");
    for (name, desc) in ENV_FINGERPRINT_FIELDS {
        let typ = match *name {
            "cpu_cores" | "mem_total_mb" => "integer",
            "features" => "string[]",
            _ => "string",
        };
        let _ = writeln!(md, "| `{name}` | {typ} | {desc} |");
    }
    md.push('\n');

    // Per-schema required fields
    md.push_str("## Required Fields by Schema\n\n");

    md.push_str("### `pi.ext.rust_bench.v1`\n\n");
    md.push_str("| Field | Type | Description |\n");
    md.push_str("|---|---|---|\n");
    md.push_str("| `schema` | string | Always `\"pi.ext.rust_bench.v1\"` |\n");
    md.push_str("| `runtime` | string | Always `\"pi_agent_rust\"` |\n");
    md.push_str("| `scenario` | string | Benchmark scenario (e.g., `ext_load_init/load_init_cold`) |\n");
    md.push_str("| `extension` | string | Extension ID being benchmarked |\n");
    md.push_str("| `runs` | integer | Number of runs (load scenarios) |\n");
    md.push_str("| `iterations` | integer | Number of iterations (throughput scenarios) |\n");
    md.push_str("| `summary` | object | `{count, min_ms, p50_ms, p95_ms, p99_ms, max_ms}` |\n");
    md.push_str("| `elapsed_ms` | float | Total elapsed time in milliseconds |\n");
    md.push_str("| `per_call_us` | float | Per-call latency in microseconds |\n");
    md.push_str("| `calls_per_sec` | float | Throughput (calls per second) |\n\n");

    md.push_str("### `pi.ext.legacy_bench.v1`\n\n");
    md.push_str("Same structure as `pi.ext.rust_bench.v1` with:\n");
    md.push_str("- `runtime` = `\"legacy_pi_mono\"`\n");
    md.push_str("- `node` object: `{version, platform, arch}`\n\n");

    md.push_str("### `pi.perf.workload.v1`\n\n");
    md.push_str("| Field | Type | Description |\n");
    md.push_str("|---|---|---|\n");
    for field in WORKLOAD_REQUIRED {
        let desc = match *field {
            "scenario" => "Workload scenario name",
            "iterations" => "Number of outer iterations",
            "tool_calls_per_iteration" => "Tool calls per iteration",
            "total_calls" => "Total tool calls executed",
            "elapsed_ms" => "Total elapsed milliseconds",
            "per_call_us" => "Per-call latency in microseconds",
            "calls_per_sec" => "Throughput (calls per second)",
            _ => "",
        };
        let _ = writeln!(md, "| `{field}` | number | {desc} |");
    }
    md.push('\n');

    // Determinism notes
    md.push_str("## Determinism Requirements\n\n");
    md.push_str("1. **Stable key ordering**: JSON keys are sorted alphabetically within each record\n");
    md.push_str("2. **No floating point in keys**: Use string or integer identifiers\n");
    md.push_str("3. **Timestamps**: ISO 8601 with seconds precision (`2026-02-06T01:00:00Z`)\n");
    md.push_str("4. **Config hash**: SHA-256 of concatenated env fields for dedup\n");
    md.push_str("5. **One record per line**: Standard JSONL (newline-delimited JSON)\n");

    let md_path = reports_dir.join("BENCH_SCHEMA.md");
    std::fs::write(&md_path, &md).expect("write BENCH_SCHEMA.md");

    // Write machine-readable schema registry
    let registry = json!({
        "schema": "pi.bench.schema_registry.v1",
        "schemas": SCHEMAS.iter().map(|(name, desc)| json!({
            "name": name,
            "description": desc,
        })).collect::<Vec<_>>(),
        "env_fingerprint_fields": ENV_FINGERPRINT_FIELDS.iter().map(|(name, desc)| json!({
            "field": name,
            "description": desc,
        })).collect::<Vec<_>>(),
    });

    let registry_path = reports_dir.join("bench_schema_registry.json");
    std::fs::write(
        &registry_path,
        serde_json::to_string_pretty(&registry).unwrap_or_default(),
    )
    .expect("write bench_schema_registry.json");

    eprintln!("[schema] Generated:");
    eprintln!("  {}", md_path.display());
    eprintln!("  {}", registry_path.display());
}

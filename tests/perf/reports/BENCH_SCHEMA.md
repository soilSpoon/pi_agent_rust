# Benchmark JSONL Schema Reference

> Auto-generated. Do not edit manually.

## Registered Schemas

| Schema | Description |
|---|---|
| `pi.bench.protocol.v1` | Canonical benchmark protocol contract (partitions, datasets, metadata, replay inputs) |
| `pi.ext.rust_bench.v1` | Rust QuickJS extension benchmark event (load, tool call, event hook) |
| `pi.ext.legacy_bench.v1` | Legacy pi-mono (Node.js) extension benchmark event |
| `pi.perf.workload.v1` | PiJS workload harness output (tool call throughput) |
| `pi.perf.budget.v1` | Performance budget check result |
| `pi.perf.budget_summary.v1` | Aggregate budget summary with pass/fail counts |
| `pi.ext.conformance_report.v2` | Per-extension conformance report event |
| `pi.ext.conformance_summary.v2` | Aggregate conformance summary with per-tier breakdowns |

## Environment Fingerprint

Every benchmark record SHOULD include an `env` object with:

| Field | Type | Description |
|---|---|---|
| `os` | string | Operating system name and version |
| `arch` | string | CPU architecture (x86_64, aarch64) |
| `cpu_model` | string | CPU model string from /proc/cpuinfo or sysinfo |
| `cpu_cores` | integer | Logical CPU core count |
| `mem_total_mb` | integer | Total system memory in megabytes |
| `build_profile` | string | Cargo build profile: debug or release |
| `git_commit` | string | Short git commit hash of the build |
| `features` | string[] | Active Cargo feature flags |
| `config_hash` | string | SHA-256 of env fields for dedup |

## Required Fields by Schema

### `pi.ext.rust_bench.v1`

| Field | Type | Description |
|---|---|---|
| `schema` | string | Always `"pi.ext.rust_bench.v1"` |
| `runtime` | string | Always `"pi_agent_rust"` |
| `scenario` | string | Benchmark scenario (e.g., `ext_load_init/load_init_cold`) |
| `extension` | string | Extension ID being benchmarked |
| `runs` | integer | Number of runs (load scenarios) |
| `iterations` | integer | Number of iterations (throughput scenarios) |
| `summary` | object | `{count, min_ms, p50_ms, p95_ms, p99_ms, max_ms}` |
| `elapsed_ms` | float | Total elapsed time in milliseconds |
| `per_call_us` | float | Per-call latency in microseconds |
| `calls_per_sec` | float | Throughput (calls per second) |

### `pi.ext.legacy_bench.v1`

Same structure as `pi.ext.rust_bench.v1` with:
- `runtime` = `"legacy_pi_mono"`
- `node` object: `{version, platform, arch}`

### `pi.perf.workload.v1`

| Field | Type | Description |
|---|---|---|
| `scenario` | number | Workload scenario name |
| `iterations` | number | Number of outer iterations |
| `tool_calls_per_iteration` | number | Tool calls per iteration |
| `total_calls` | number | Total tool calls executed |
| `elapsed_ms` | number | Total elapsed milliseconds |
| `per_call_us` | number | Per-call latency in microseconds |
| `calls_per_sec` | number | Throughput (calls per second) |

### `pi.bench.protocol.v1`

| Field | Type | Description |
|---|---|---|
| `schema` | string | Always `"pi.bench.protocol.v1"` |
| `version` | string | Protocol version used by all benchmark harnesses |
| `partition_tags` | string[] | Must include `matched-state` and `realistic` |
| `realistic_session_sizes` | integer[] | Canonical matrix: 100k, 200k, 500k, 1M, 5M |
| `matched_state_scenarios` | object[] | `cold_start`, `warm_start`, `tool_call`, `event_dispatch` with replay inputs |
| `required_metadata_fields` | string[] | `runtime`, `build_profile`, `host`, `scenario_id`, `correlation_id` |
| `evidence_labels` | object | `evidence_class` (`measured/inferred`) + `confidence` (`high/medium/low`) |

## Protocol Matrix

| Partition | Scenario ID | Replay Input |
|---|---|---|
| `matched-state` | `cold_start` | `{"extension_fixture_set":["hello","pirate","diff"],"runs":5}` |
| `matched-state` | `warm_start` | `{"extension_fixture_set":["hello","pirate","diff"],"runs":5}` |
| `matched-state` | `tool_call` | `{"extension_fixture_set":["hello","pirate","diff"],"iterations":500}` |
| `matched-state` | `event_dispatch` | `{"event_name":"before_agent_start","iterations":500}` |
| `realistic` | `realistic/session_100000` | `{"mode":"replay","seed":7,"transcript_fixture":"tests/artifacts/perf/session_100000.jsonl"}` |
| `realistic` | `realistic/session_200000` | `{"mode":"replay","seed":7,"transcript_fixture":"tests/artifacts/perf/session_200000.jsonl"}` |
| `realistic` | `realistic/session_500000` | `{"mode":"replay","seed":7,"transcript_fixture":"tests/artifacts/perf/session_500000.jsonl"}` |
| `realistic` | `realistic/session_1000000` | `{"mode":"replay","seed":7,"transcript_fixture":"tests/artifacts/perf/session_1000000.jsonl"}` |
| `realistic` | `realistic/session_5000000` | `{"mode":"replay","seed":7,"transcript_fixture":"tests/artifacts/perf/session_5000000.jsonl"}` |

## Determinism Requirements

1. **Stable key ordering**: JSON keys are sorted alphabetically within each record
2. **No floating point in keys**: Use string or integer identifiers
3. **Timestamps**: ISO 8601 with seconds precision (`2026-02-06T01:00:00Z`)
4. **Config hash**: SHA-256 of concatenated env fields for dedup
5. **One record per line**: Standard JSONL (newline-delimited JSON)

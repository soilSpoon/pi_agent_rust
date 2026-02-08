# Benchmark JSONL Schema Reference

> Auto-generated. Do not edit manually.

## Registered Schemas

| Schema | Description |
|---|---|
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

## Determinism Requirements

1. **Stable key ordering**: JSON keys are sorted alphabetically within each record
2. **No floating point in keys**: Use string or integer identifiers
3. **Timestamps**: ISO 8601 with seconds precision (`2026-02-06T01:00:00Z`)
4. **Config hash**: SHA-256 of concatenated env fields for dedup
5. **One record per line**: Standard JSONL (newline-delimited JSON)

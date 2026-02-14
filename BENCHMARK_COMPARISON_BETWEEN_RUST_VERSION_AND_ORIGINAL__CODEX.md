# BENCHMARK_COMPARISON_BETWEEN_RUST_VERSION_AND_ORIGINAL__CODEX

Generated: 2026-02-14
Workspace: `/data/projects/pi_agent_rust`

## 1) Lede (Do Not Bury This)

1. Rust is currently **slower** than legacy in wall-clock for long-session resume/workload paths in this snapshot, often ~`1.2x` to `2.0x` slower than Node and ~`1.9x` to `4.0x` slower than Bun in realistic end-to-end runs.
2. Rust is currently **much smaller in memory footprint** for equivalent workloads in synthetic matched-state runs, and still significantly smaller in realistic runs with heavy exports/forks/extension activity.
3. Extension compatibility is substantial but not complete: vendored corpus is `223` extensions, with `187` pass, `29` fail, `7` pending manifest alignment.
4. Rust has significantly expanded first-class capability surface versus legacy coding-agent CLI (commands, policy explainers, provider metadata/control, risk/quota/security instrumentation).
5. The largest practical optimization target remains session append/save behavior at high token-volume and large histories; this is the best lever for major speed gains.

---

## 2) Scope and Comparison Modes

### 2.1 Apples-to-Apples Scope
- Rust target: this repo (`pi_agent_rust`)
- Legacy target: `legacy_pi_mono_code/pi-mono/packages/coding-agent`

### 2.2 Apples-to-Oranges Scope (Full Legacy Runtime Context)
- Legacy aggregate target: `packages/{ai,agent,coding-agent,tui}`
- Purpose: include behavior that legacy offloads into sibling packages (provider stack, UI/runtime services), so comparisons are not unfairly narrow.

### 2.3 Benchmarks Included
- Matched-state long-session benchmark: resume + append the same 10 user messages.
- Realistic E2E benchmark: resume + append + extension-like activity + slash-like state changes + forking + exports + compactions.
- Extension microbench: real extension loading and real tool/event dispatch.
- Extension corpus conformance: full vendored/unvendored compatibility reports.
- These suites are intended to function as a practical system-level regression harness, not just synthetic microbench snapshots.

### 2.4 Provider API Cost Control
- This report does **not** use paid external API calls for the benchmark matrices.
- No cost-driving live-provider throughput benchmark is included here.
- If provider-call benchmarks are added, use `ollama` first for cost control.

---

## 3) Codebase Scale and Complexity

## 3.1 LOC (Production vs Test)

Method: `tokei` scoped by language (`Rust` for Rust repo, `TypeScript` for legacy scopes).

| Scope | Production LOC | Test LOC |
|---|---:|---:|
| Rust (`src`, Rust only) | 169,439 | 180,790 (`tests`, Rust only) |
| Legacy coding-agent only (`src/test`, TS only) | 27,412 | 8,871 |
| Legacy full stack (`ai+agent+coding-agent+tui`, TS only) | 55,313 | 21,779 |

Ratios:
- Rust vs legacy coding-agent: prod `6.18x`, test `20.38x`
- Rust vs legacy full-stack: prod `3.06x`, test `8.30x`

## 3.2 Function/Callable Inventory

Method note:
- Rust callable count here uses signature-oriented scanning (`fn` signatures + test attrs); this is approximate for Rust language forms.
- Legacy callable count uses AST inventory generated from TypeScript compiler APIs.

Rust (signature inventory):
- `src` function signatures: `6,964`
- `tests` function signatures: `7,863`
- test attributes total: `9,208` (`src=3,798`, `tests=5,410`)

Legacy AST callable inventory:
- coding-agent `src`: `2,180`
- coding-agent `test`: `1,325`
- full stack `src`: `3,016`
- full stack `test`: `3,086`

## 3.3 Test Coverage Baseline (Rust)

From `docs/coverage-baseline-map.json`:
- Line coverage: `79.08%` (`95,706 / 121,018`)
- Function coverage: `78.01%` (`8,545 / 10,954`)
- Branch coverage: `51.95%` (documented lower-bound due llvm-cov export SIGSEGV on subset of files)

---

## 4) Verified Feature/Functionality Delta

This section lists **verified Rust-first-class surfaces** missing from legacy coding-agent CLI in this workspace snapshot.

## 4.1 CLI Surface Delta (Direct Help Diff)

Rust-only top-level commands:
- `doctor`
- `info`
- `search`
- `update-index`

Rust-only flags:
- `--extension-policy`
- `--explain-extension-policy`
- `--repair-policy`
- `--explain-repair-policy`
- `--list-providers`
- `--theme-path`

## 4.2 Rust-Only Major Capability Areas (with complexity hints)

| Capability area | Primary Rust implementation | Approx LOC | Approx fn count |
|---|---|---:|---:|
| Extension runtime + policy + host integration | `src/extensions.rs` | 31,995 | 560 |
| QuickJS bridge + hostcall plumbing + runtime adapters | `src/extensions_js.rs` | 20,341 | 111 |
| Dispatcher for protocol/hostcall integration | `src/extension_dispatcher.rs` | 8,968 | 146 |
| Provider canonical metadata + alias routing | `src/provider_metadata.rs` | 2,650 | 46 |
| Extension index/search/info/update pipeline | `src/extension_index.rs` | 1,409 | 51 |
| Environment + compatibility diagnostics (`doctor`) | `src/doctor.rs` | 1,472 | 32 |
| Runtime risk ledger/replay/calibration tooling | `src/extensions.rs`, `src/bin/ext_runtime_risk_ledger.rs` | large integrated surface | integrated |
| Per-extension quota enforcement engine | `src/extensions.rs` | integrated in core runtime | integrated |

## 4.3 Provider Breadth Delta

- Rust canonical provider IDs: `87`
- Rust alias IDs: `34`
- Legacy provider IDs (from generated legacy model table): `22`
- Rust provider IDs not in mapped legacy set: `68`

Complete Rust-only provider ID list appears in **Appendix B**.

---

## 5) Benchmark Methodology (Realistic + Extreme)

All major benchmark classes were run with identical workload structure per runtime where possible.

## 5.1 Realistic E2E Workload Semantics

Realistic mode executes:
- resume/open existing long session
- append new user+assistant turns
- insert tool-result messages
- extension custom-entry activity
- slash-like state changes (model, thinking level, session info, labels)
- compaction entries
- fork simulation (`branch` summary operations)
- export generation (HTML)
- final save/index update

Parameters for realistic matrix:
- `messages=5000`
- `append=10`
- `compactions=12`
- `extension_ops=40`
- `slash_ops=40`
- `forks=8`
- `exports=2`
- token levels: `100k`, `200k`, `500k`, `1M`, `5M`
- runs per cell: `3`

---

## 6) Performance Results

## 6.1 Realistic E2E Latency (p50, ms)

| Runtime | Token level | Open | Append/Ops | Save | Total |
|---|---:|---:|---:|---:|---:|
| legacy_bun | 100k | 24.63 | 143.84 | 0.00 | 168.47 |
| legacy_node | 100k | 47.20 | 220.70 | 0.00 | 267.91 |
| rust | 100k | 36.84 | 219.06 | 64.64 | 320.71 |
| legacy_bun | 200k | 29.55 | 196.99 | 0.00 | 226.70 |
| legacy_node | 200k | 58.77 | 303.60 | 0.00 | 362.37 |
| rust | 200k | 40.42 | 397.48 | 113.92 | 552.70 |
| legacy_bun | 500k | 39.01 | 375.75 | 0.00 | 415.27 |
| legacy_node | 500k | 76.68 | 607.04 | 0.00 | 684.64 |
| rust | 500k | 51.22 | 925.65 | 250.27 | 1,226.71 |
| legacy_bun | 1M | 50.83 | 649.51 | 0.00 | 700.52 |
| legacy_node | 1M | 119.76 | 1,117.65 | 0.00 | 1,238.67 |
| rust | 1M | 68.86 | 1,846.67 | 482.81 | 2,401.35 |
| legacy_bun | 5M | 155.63 | 2,801.90 | 0.00 | 2,959.42 |
| legacy_node | 5M | 396.41 | 5,578.20 | 0.00 | 5,974.67 |
| rust | 5M | 204.35 | 9,266.76 | 2,359.30 | 11,828.14 |

Rust total p50 ratio vs legacy:

| Token level | Rust/Node | Rust/Bun |
|---|---:|---:|
| 100k | 1.20x | 1.90x |
| 200k | 1.53x | 2.44x |
| 500k | 1.79x | 2.95x |
| 1M | 1.94x | 3.43x |
| 5M | 1.98x | 4.00x |

Key interpretation:
- Rust open phase is competitive and often better than Node at higher scale.
- Current Rust bottleneck is append/save behavior under large long-session churn.

## 6.2 Matched-State Synthetic Benchmark (Same Session State, Resume + 10)

This is the direct “same state then add same 10 messages” comparison.

| Runtime | Token level | Open ms | Append ms | Save ms | Total ms | RSS KB | User s | Sys s | FS out |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rust | 1M | 68.78 | 254.76 | 432.47 | 756.01 | 32,092 | 0.86 | 0.04 | 112 |
| legacy_node | 1M | 126.37 | 170.57 | 0.00 | 296.94 | 167,752 | 0.76 | 0.16 | 0 |
| legacy_bun | 1M | 52.85 | 90.51 | 0.00 | 143.36 | 184,492 | 0.31 | 0.17 | 0 |
| rust | 5M | 210.30 | 1,282.08 | 2,124.94 | 3,617.31 | 129,836 | 3.84 | 0.38 | 112 |
| legacy_node | 5M | 399.61 | 1,395.80 | 0.00 | 1,795.41 | 411,372 | 1.95 | 0.63 | 0 |
| legacy_bun | 5M | 156.24 | 405.62 | 0.00 | 561.86 | 481,852 | 0.56 | 0.42 | 0 |

Ratios at matched state:
- 1M: Rust latency `2.55x` Node, `5.27x` Bun; Rust memory is `5.23x` smaller than Node and `5.75x` smaller than Bun.
- 5M: Rust latency `2.01x` Node, `6.44x` Bun; Rust memory is `3.17x` smaller than Node and `3.71x` smaller than Bun.

## 6.3 Realistic Footprint (Same Realistic Ops, 1M/5M)

| Runtime | Token level | Open ms | Append/Ops ms | Save ms | Total ms | RSS KB | User s | Sys s | FS out | Wall |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rust | 1M | 163.91 | 2,654.88 | 505.34 | 3,324.14 | 76,240 | 3.31 | 0.21 | 112 | 0:03.36 |
| legacy_node | 1M | 199.10 | 1,508.55 | 0.00 | 1,707.65 | 820,380 | 1.44 | 1.16 | 0 | 0:02.26 |
| legacy_bun | 1M | 92.71 | 810.69 | 0.00 | 903.40 | 875,092 | 0.63 | 0.79 | 0 | 0:01.21 |
| rust | 5M | 674.47 | 13,224.33 | 2,460.56 | 16,359.37 | 274,832 | 15.79 | 1.06 | 112 | 0:16.40 |
| legacy_node | 5M | 793.81 | 8,018.42 | 0.00 | 8,812.23 | 2,173,096 | 4.77 | 5.57 | 0 | 0:09.54 |
| legacy_bun | 5M | 325.28 | 3,882.84 | 0.00 | 4,208.12 | 3,057,908 | 1.67 | 3.42 | 0 | 0:04.75 |

Interpretation:
- Latency: Rust still slower in realistic E2E.
- Memory: Rust remains much smaller (`~7.9x` to `~11.5x` lower RSS in these realistic runs).

---

## 7) Extension Runtime Design and Compatibility Status

## 7.1 Rust Extension Architecture (Deep-Dive)

Rust extension handling is centered on a capability-gated QuickJS host runtime with explicit hostcall dispatch and policy enforcement.

Core properties:
- Connector model instead of ambient Node/Bun authority (`tool`, `exec`, `http`, `session`, `ui`, `events`, `log`).
- Policy-first dispatch (`allow/prompt/deny`) with explainable profiles and CLI explainers.
- Deterministic event-loop bridge (microtask drain + host completion scheduling discipline).
- Structured lifecycle controls and bounded execution regions.
- Compatibility shims for high-value Node/Bun surfaces rather than full runtime emulation.
- Runtime risk scoring + hash-chained ledger + replay/calibration artifacts.
- Per-extension quota enforcement integrated into shared hostcall dispatch.

Design/implementation emphasis areas:
- `src/extensions.rs`
- `src/extensions_js.rs`
- `src/extension_dispatcher.rs`
- `EXTENSIONS.md` (runtime contract + conformance process)

## 7.2 Real Extension Execution Benchmarks (Rust vs Legacy)

| Scenario | Extension | Rust | Legacy | Rust/Legacy |
|---|---|---:|---:|---:|
| `ext_load_init/load_init_cold` | hello | 13.43 ms | 141.84 ms | 0.09x |
| `ext_load_init/load_init_cold` | pirate | 13.11 ms | 23.17 ms | 0.57x |
| `ext_tool_call/hello` | hello | 35.46 us/call | 2.74 us/call | 12.96x slower |
| `ext_event_hook/before_agent_start` | pirate | 36.55 us/call | 4.82 us/call | 7.59x slower |

Interpretation:
- Rust cold-load can be competitive or faster in these samples.
- Per-call dispatch overhead is still much higher in Rust and needs focused optimization.

## 7.3 Corpus Conformance (223+ extension target)

Source: `tests/ext_conformance/reports/pipeline/full_validation_report.compat2.json` (`generatedAt=2026-02-14T09:05:16Z`)

Corpus:
- total candidates: `1000`
- vendored: `223`
- unvendored: `777`

Vendored status:
- pass: `187`
- fail: `29`
- pending manifest alignment: `7`
- tested pass rate (`pass/(pass+fail)`): `86.57%`
- overall vendored pass rate (`pass/223`): `83.86%`

Failure taxonomy (vendored non-pass):
- `harness_gap`: `23`
- `needs_review`: `12`
- `extension_problem`: `1`

Stage summary:
- passed: `8`
- failed: `1` (`auto_repair_full_corpus`, exit 101)
- skipped: `1` (`differential_suite`)

## 7.4 Extensions Not Yet 100% Passing (All 36 Vendored Non-Pass)

Columns: `id`, `status`, `verdict`, `failure_category`, `reason`, `suggested_fix`

```tsv
agents-mikeastock/extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
community/nicobailon-interview-tool	fail	extension_problem	extension_load_error	Extension expects local assets/files unavailable at runtime.	Bundle required assets or extend missing_asset auto-repair policy.
community/prateekmedia-lsp	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
doom-overlay	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/@verioussmith/pi-openrouter	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/agentsbox	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/aliou-pi-linkup	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-synthetic	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/lsp-pi	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-bar	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-core	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/mitsupi	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/oh-my-pi-anthropic-websearch	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-exa	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-lsp	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-pi-git-tool	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-subagents	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/pi-amplike	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-bash-confirm	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-messenger	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-package-test	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-search-agent	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-shell-completions	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/shitty-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/tmustier-pi-arcade	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/vaayne-agent-kit	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-mcp	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
third-party/aliou-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/ben-vargas-pi-packages	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/charles-cooper-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/kcosr-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/marckrenn-pi-sub	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/openclaw-openclaw	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/pasky-pi-amplike	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/w-winter-dot314	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
```

## 7.5 Remediation Plan for Remaining Extension Gaps

1. Close `harness_gap` first (`23` items): refresh TS oracle snapshots and regenerate validated manifests.
2. Resolve pending manifest drift (`7` items): rebuild `VALIDATED_MANIFEST.json`, re-run shards.
3. Triage `needs_review` load failures (`12` items): classify runtime shim gap vs extension defect with dossier reproduction.
4. Contain true extension defects (`extension_problem`): package missing assets or mark as extension-side defect.

---

## 8) Test Surface Comparison (Unit + E2E)

Rust:
- Rust test files: `228`
- Rust e2e-prefixed test files: `33`
- Rust test attributes: `9,208`

Legacy proxies (AST-based):
- coding-agent test files: `49`
- coding-agent test callsites: `762`
- full stack test files (`ai+agent+coding-agent+tui`): `107`
- full stack test callsites: `1,724`

Note: legacy tree in this workspace does not provide an equivalent consolidated coverage JSON artifact like `docs/coverage-baseline-map.json` for direct percentage parity.

---

## 9) Security / Reliability / asupersync Impact

## 9.1 Security
- Rust extension path is capability-gated and auditable per hostcall.
- Policy explainers + explicit deny/prompt/allow semantics are first-class.
- Risk and quota controls are integrated and test-instrumented.

## 9.2 Reliability and Correctness
- Structured concurrency foundation (`asupersync`) reduces async lifecycle ambiguity.
- Deterministic cancellation/resource scoping improves robustness of long-lived CLI sessions.
- Hash-chained risk ledger + replay/calibration tooling improve post-incident reproducibility.

## 9.3 asupersync “Correct-by-Design” Impact
- Work is scoped to explicit lifetimes, which reduces hidden background-task leakage and orphaned async work.
- Cancellation becomes a first-class control flow primitive instead of a best-effort convention, reducing stuck-session and shutdown race risk.
- Deterministic runtime patterns make failure reproduction and forensic replay more credible (especially with extension hostcall/risk ledgers).
- The primary tradeoff is a stricter execution model that can add engineering/coordination overhead versus loosely structured async graphs.
- In this benchmark snapshot, correctness and controllability gains are clear, while latency still needs targeted optimization in the large-session hot paths.

## 9.4 Performance Trade in This Snapshot
- Legacy (especially Bun) wins latency on current long-session end-to-end paths.
- Rust wins memory footprint substantially.
- High-value optimization targets are clear and measurable.

---

## 10) Extreme Optimization Priorities (To Reach Next 5-10x)

These are the highest expected-value targets from measured bottlenecks:

1. Session append/save hot path:
- Minimize repeated full-history serialization work.
- Introduce incremental persistence for large session files.
- Reduce allocation churn and copy amplification in append/update routines.

2. JSON parse/serialize fast path:
- Eliminate avoidable intermediate `Value` transforms in hot loops.
- Prefer typed deserialization in critical paths.
- Use zero-copy/borrowed parsing where safe and measurable.

3. Extension per-call overhead:
- Reduce hostcall marshalling overhead and temporary allocations.
- Batch or precompute invariant policy/risk metadata for high-frequency calls.
- Optimize hot connector dispatch paths (`tool`/`events`).

4. Multi-core and locality:
- Partition expensive analysis and indexing work off the foreground session loop.
- Improve cache locality in session entry scans/index updates.
- Keep save/index updates append-oriented rather than full-rebuild when possible.

5. Regression guardrails:
- Keep the realistic 100k/200k/500k/1M/5M matrix as a blocking perf CI track.
- Track p50/p95, RSS, and FS I/O deltas per commit series.

---

## 11) Appendix A — Full Vendored Extension List (223)

Columns: `id`, `sourceTier`, `candidateStatus`, `conformanceStatus`, `verdict`, `conformanceFailureCategory`, `classificationReason`, `suggestedFix`

```tsv
agents-mikeastock/extensions	agents-mikeastock	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
antigravity-image-gen	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
auto-commit-on-exit	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
bash-spawn-hook	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
bookmark	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
claude-rules	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/ferologics-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-clipboard	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-cost-tracker	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-flicker-corp	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-funny-working-message	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-handoff	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-memory-mode	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-oracle	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-plan-mode	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-resistance	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-speedreading	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-status-widget	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-ultrathink	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-usage-bar	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/jyaunches-canvas	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-answer	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-control	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-cwd-history	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-files	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-review	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-todos	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-uv	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-whimsical	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-interactive-shell	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-interview-tool	community	vendored	fail	extension_problem	extension_load_error	Extension expects local assets/files unavailable at runtime.	Bundle required assets or extend missing_asset auto-repair policy.
community/nicobailon-mcp-adapter	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-powerline-footer	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-rewind-hook	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-subagents	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/ogulcancelik-ghostty-theme-sync	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-checkpoint	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-lsp	community	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
community/prateekmedia-permission	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-ralph-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-repeat	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-token-rate	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-background-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-compact-config	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-pi-agent-scip	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-safe-git	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-safe-rm	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-session-color	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-session-emoji	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-agent-guidance	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-mario-not	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-picman	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-ping	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-spice-invaders	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-tetris	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-code-actions	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-files-widget	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-ralph-wiggum	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-raw-paste	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-tab-status	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-usage-extension	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
confirm-destructive	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-compaction	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-footer	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-header	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-anthropic	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-gitlab-duo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-qwen-cli	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
dirty-repo-guard	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
doom-overlay	official-pi-mono	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
dynamic-resources	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
event-bus	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
file-trigger	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
git-checkpoint	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
handoff	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
hello	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
inline-bash	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
input-transform	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
interactive-shell	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
mac-system-theme	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
message-renderer	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
modal-editor	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
model-status	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
notify	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/@verioussmith/pi-openrouter	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/agentsbox	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/aliou-pi-extension-dev	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-guardrails	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-linkup	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-processes	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-synthetic	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-toolchain	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-ancestor-discovery	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-antigravity-image-gen	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-synthetic-provider	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/checkpoint-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/imsus-pi-extension-minimax-coding-plan-mcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-extension-settings	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-files	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-gob	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/lsp-pi	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-bar	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-core	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/mitsupi	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/ogulcancelik-pi-sketch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/oh-my-pi-anthropic-websearch	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-basics	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/oh-my-pi-exa	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-lsp	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-pi-git-tool	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-subagents	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/permission-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-agentic-compaction	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-amplike	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-annotate	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-bash-confirm	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-brave-search	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-command-center	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-ephemeral	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-extensions	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-ghostty-theme-sync	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-interactive-shell	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-interview	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-mcp-adapter	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-md-export	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-mermaid	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-messenger	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-model-switch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-moonshot	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-multicodex	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-notify	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-package-test	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-poly-notify	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-powerline-footer	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-prompt-template-model	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-repoprompt-mcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-review-loop	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-screenshots-picker	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-search-agent	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-session-ask	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-shadow-git	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-shell-completions	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-skill-palette	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-subdir-context	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-super-curl	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-telemetry-otel	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-threads	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-voice-of-god	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-wakatime	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-watch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-web-access	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/qualisero-pi-agent-scip	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/ralph-loop-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/repeat-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/shitty-extensions	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/tmustier-pi-arcade	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/token-rate-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vaayne-agent-kit	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-mcp	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-subagent	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vaayne-pi-web-tools	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vpellegrino-pi-skills	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/walterra-pi-charts	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/walterra-pi-graphviz	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/zenobius-pi-dcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
overlay-qa-tests	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
overlay-test	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
permission-gate	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
pirate	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
plan-mode	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
preset	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
protected-paths	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
qna	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
question	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
questionnaire	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
rainbow-editor	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
rpc-demo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
sandbox	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
send-user-message	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
session-name	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
shutdown-command	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
snake	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
space-invaders	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
ssh	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
status-line	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
subagent	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
summarize	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
system-prompt-header	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/aliou-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/ben-vargas-pi-packages	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/charles-cooper-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/cv-pi-ssh-remote	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/graffioh-pi-screenshots-picker	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/graffioh-pi-super-curl	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/jyaunches-pi-canvas	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/kcosr-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/limouren-agent-things	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/lsj5031-pi-notification-extension	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/marckrenn-pi-sub	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/michalvavra-agents	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/ogulcancelik-pi-sketch	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/openclaw-openclaw	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/pasky-pi-amplike	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/qualisero-pi-agent-scip	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/raunovillberg-pi-stuffed	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-direnv	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-questionnaire	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-slow-mode	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/vtemian-pi-config	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/w-winter-dot314	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/zenobi-us-pi-dcp	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
timed-confirm	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
titlebar-spinner	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
todo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
tool-override	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
tools	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
trigger-compact	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
truncated-tool	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
widget-placement	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
with-deps	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
```

## 12) Appendix B — Rust Provider IDs Not in Legacy Mapped Set (68)

```text
302ai
abacus
aihubmix
alibaba
alibaba-cn
bailing
baseten
berget
chutes
cloudflare-ai-gateway
cloudflare-workers-ai
cohere
cortecs
deepinfra
deepseek
fastrouter
fireworks
firmware
friendli
github-models
gitlab
helicone
iflowcn
inception
inference
io-net
jiekou
llama
lmstudio
lucidquery
minimax-cn-coding-plan
minimax-coding-plan
moark
modelscope
moonshotai
moonshotai-cn
morph
nano-gpt
nebius
nova
novita-ai
nvidia
ollama
ollama-cloud
ovhcloud
perplexity
poe
privatemode-ai
requesty
sap-ai-core
scaleway
siliconflow
siliconflow-cn
stackit
submodel
synthetic
togetherai
upstage
v0
venice
vivgrid
vultr
wandb
xiaomi
zai-coding-plan
zenmux
zhipuai
zhipuai-coding-plan
```

## 13) Appendix C — Feature Complexity Tables Used in This Report

### 13.1 Rust Feature Complexity Table

```tsv
file	loc	fn_count
src/extensions.rs	31995	560
src/extensions_js.rs	20341	111
src/extension_dispatcher.rs	8968	146
src/provider_metadata.rs	2650	46
src/extension_index.rs	1409	51
src/doctor.rs	1472	32
src/session.rs	5041	80
src/session_index.rs	1388	40
src/cli.rs	863	67
src/main.rs	2261	19
src/providers/mod.rs	2323	69
src/providers/openai.rs	1948	26
src/providers/anthropic.rs	1771	18
src/providers/gemini.rs	1362	21
src/providers/azure.rs	1180	13
src/providers/cohere.rs	1738	22
src/providers/vertex.rs	987	14
src/providers/bedrock.rs	1146	6
src/providers/gitlab.rs	480	9
src/providers/copilot.rs	542	11
src/bin/ext_full_validation.rs	1806	1
src/bin/ext_workloads.rs	474	1
src/bin/session_workload_bench.rs	488	5
```

### 13.2 Legacy Feature Complexity Table

```tsv
file	loc	callables
packages/coding-agent/src/core/extensions/index.ts	156	0
packages/coding-agent/src/core/extensions/wrapper.ts	119	8
packages/coding-agent/src/core/extensions/runner.ts	719	90
packages/coding-agent/src/core/session-manager.ts	1395	90
packages/coding-agent/src/core/model-registry.ts	600	31
packages/coding-agent/src/cli/args.ts	304	5
packages/coding-agent/src/main.ts	673	33
packages/ai/src/providers/register-builtins.ts	74	2
packages/ai/src/providers/openai-responses.ts	274	12
packages/ai/src/providers/openai-completions.ts	848	40
packages/ai/src/providers/anthropic.ts	732	31
packages/ai/src/providers/google.ts	453	12
packages/ai/src/providers/google-vertex.ts	483	14
packages/ai/src/providers/amazon-bedrock.ts	649	22
packages/ai/src/providers/azure-openai-responses.ts	257	10
packages/ai/src/providers/google-gemini-cli.ts	1023	22
packages/ai/src/providers/openai-codex-responses.ts	450	19
```

## 14) Appendix D — Primary Raw Artifacts

- Realistic E2E latency matrix
  - `/tmp/pi_token_volume_bench/realistic_e2e_comparison_results.jsonl`
  - `/tmp/pi_token_volume_bench/realistic_e2e_comparison_summary.json`
  - `/tmp/pi_token_volume_bench/realistic_e2e_comparison_ratios.json`
- Matched-state large-session footprint
  - `/tmp/pi_token_volume_bench/matched_state_footprint_large.jsonl`
  - `/tmp/pi_token_volume_bench/time_*_matched_state_*.txt`
- Realistic large-session footprint (new)
  - `/tmp/pi_token_volume_bench/realistic_footprint_large.jsonl`
  - `/tmp/pi_token_volume_bench/time_*_realistic_*.txt`
- Extension execution microbench
  - `/tmp/pi_token_volume_bench/ext_workloads_realistic_update.jsonl`
  - `/tmp/pi_token_volume_bench/legacy_ext_workloads_realistic_update.jsonl`
- Extension conformance corpus outputs
  - `tests/ext_conformance/reports/pipeline/full_validation_report.compat2.json`
  - `tests/ext_conformance/reports/pipeline/full_validation_report.compat2.md`
  - `/tmp/pi_token_volume_bench/vendored_extensions_compat2.tsv`
  - `/tmp/pi_token_volume_bench/vendored_nonpass_compat2.tsv`
- Provider inventory/parity artifacts
  - `docs/provider-canonical-id-table.json`
  - `docs/provider-parity-reconciliation-report.json`
  - `/tmp/pi_token_volume_bench/rust_provider_ids_extra_vs_legacy.txt`
- Coverage and test-surface artifacts
  - `docs/coverage-baseline-map.json`
  - `docs/TEST_COVERAGE_MATRIX.md`
  - `/tmp/pi_token_volume_bench/legacy_callable_inventory.json`

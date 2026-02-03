# Extension System (Big‑Guns Plan)

This document defines the extension architecture for **pi_agent_rust** with the
goal of **maximum compatibility**, **formal safety guarantees**, and **measurable
performance**. The system is **best‑effort** by default, but designed to
converge to full parity with legacy Pi extensions.

---

## 0. Design Goals

1. **Compatibility**: run legacy Pi extensions with best‑effort fidelity.
2. **Performance**: <2ms p95 overhead per tool call (excluding tool work).
3. **Safety**: explicit, auditable capability grants with optional strict mode.
4. **Stability**: versioned protocol + conformance fixtures.
5. **Portability**: same artifact runs on Linux/macOS/Windows.

Non‑goals:
- Custom TUI rendering from extensions (core owns the UI).
- Node‑native addons (must use hostcalls or WASM).

---

## 1. Runtime Tiers (Hybrid, Best‑of‑All Worlds)

**Tier A — WASM Component (default):**
- Fast, sandboxed, portable.
- Typed hostcalls via WIT.

**Tier B — JS Compatibility (compiled):**
- Legacy TS/JS compiled to a single bundle.
- Pre‑compiled to **QuickJS bytecode** or **JS→WASM**.
- No Node runtime required.

**Tier C — MCP (process IPC):**
- For heavy integrations: IDEs, databases, cloud services.

> WASM is the default. JS compatibility is a **compile step**, not a runtime.

---

## 1A. Node/Bun‑Free Runtime: Connectors + Event Loop

Mario’s critique is correct in the narrow sense: **QuickJS is just a JS engine**.
It intentionally does **not** ship a Node/Bun‑style OS API surface or a full
general-purpose event loop.

Our answer is: **great** — we don’t want the Node/Bun surface area.

Instead, Pi provides a tiny, capability‑gated **connector layer** and an explicit
event loop that is *tailored to Pi’s extension needs* (not the entire web/Node
ecosystem).

### 1A.1 The “Connector” Model (Minimal OS Surface)

Extensions do not get raw OS access (no `fs`, no `child_process`, no arbitrary
sockets). They get a **small set of hostcalls** that map to Pi’s already-audited
operations (tools + session/ui actions).

Core examples (names illustrative):
- `pi.tool(name, input)` → delegates to the built-in tool registry (read/write/edit/bash/grep/find/ls)
- `pi.exec(command, args, options)` → a constrained process runner (timeout + process-tree cleanup)
- `pi.fs.*` → a *capability filesystem* rooted at project/cwd (no path escape)
- `pi.http(request)` → a constrained HTTP client (policy-controlled)
- `pi.session.*`, `pi.ui.*`, `pi.events.*` → Pi-internal APIs (no OS exposure)

This is strictly smaller than Node/Bun, and it is auditable: every connector call
is an explicit, logged capability check.

### 1A.2 The Event Loop Bridge (Promises Without Node)

QuickJS supports Promises/microtasks; it just needs a host to **drive** them.

We provide a tiny “Pi event loop”:
- Drain the QuickJS job queue (microtasks)
- Poll outstanding host operations (Rust futures via tokio/asupersync)
- Resolve/reject the corresponding JS Promises
- Repeat until idle (or until a deadline/timer fires)

In other words: Node’s event loop is a *product*; ours is a *proof obligation*:
it only implements what Pi needs, with deterministic testing hooks.

### 1A.3 Why This Is Better (Security + Performance)

**Security:** Node/Bun expose an enormous ambient-authority surface by default.
Our connector layer is capability-based and narrow by construction.

**Performance:** Node/Bun pay startup/memory costs for massive compatibility.
We precompile JS to bytecode (or WASM) and the runtime only contains:
1) a JS engine + 2) a small dispatcher + 3) our connectors.

**Determinism:** With asupersync (LabRuntime) we can test extension async + time
deterministically (no “real time” flakiness).

---

## 2. Artifact Pipeline (Legacy → Optimized)

**Inputs**
- `extension.json` (manifest)
- Source files (TS/JS or Rust/WASM)

**Pipeline**
1. **SWC build**: TS/JS → bundle (tree‑shaken/minified).
2. **Compatibility scan**: static analysis for forbidden APIs.
3. **Protocol shim**: rewrite legacy extension imports to hostcalls.
4. **Artifact build**:
   - **QuickJS bytecode** (fast startup), or
   - **WASM component** (portable + sandboxed).
5. **Cache** by hash:
   ```
   hash = sha256(manifest + bundle + engine_version)
   ```

**Output**
- `extension.artifact` + `artifact.json` (metadata, engine, hash, caps)

---

## 3. Extension Protocol (v1)

All communication uses a **versioned, JSON‑encoded protocol**:
`docs/schema/extension_protocol.json`.

Core message types:
- `register`
- `tool_call` / `tool_result`
- `slash_command` / `slash_result`
- `event_hook`
- `host_call` / `host_result` (extension → core connector calls)
- `log` / `error`

WASM components use the **WIT interface** in `docs/wit/extension.wit`.

---

### 3.1 Structured Logging (ext.log.v1)

All extension-related logs across **capture**, **harness**, and **runtime** must
use the same JSONL schema. The protocol `log` message payload matches this
schema exactly. One log entry per line.

**Log entry schema (required fields marked \*):**
```json
{
  "schema": "pi.ext.log.v1",          // *
  "ts": "2026-02-03T03:01:02.123Z",   // * RFC3339
  "level": "info",                    // * debug|info|warn|error
  "event": "tool_call.start",         // * stable event name
  "message": "tool call dispatched",  // * human summary
  "correlation": {                    // * IDs for joining logs
    "extension_id": "ext.my_ext",     // *
    "scenario_id": "scn-001",         // *
    "session_id": "sess-abc123",
    "run_id": "run-20260203-0001",
    "artifact_id": "sha256:...",
    "tool_call_id": "tool-42",
    "slash_command_id": "slash-7",
    "event_id": "evt-9",
    "host_call_id": "host-13",
    "rpc_id": "rpc-55",
    "trace_id": "trace-...",
    "span_id": "span-..."
  },
  "source": {                         // optional emitter info
    "component": "runtime",           // capture|harness|runtime|extension
    "host": "host.name",
    "pid": 4242
  },
  "data": { "duration_ms": 12 }
}
```

**Event naming (examples):**
- `extension.register`, `extension.ready`
- `tool_call.start`, `tool_call.end`
- `slash_command.start`, `slash_command.end`
- `event_hook.start`, `event_hook.end`
- `host_call.start`, `host_call.end`
- `policy.decision`, `compat.warning`

**Correlation rules:**
- `extension_id` + `scenario_id` are **required** for all extension logs.
- Populate the most specific ID available (`tool_call_id`, `slash_command_id`,
  `event_id`, `host_call_id`, `rpc_id`).
- `trace_id`/`span_id` are optional but recommended for long chains.

**Redaction rules (mandatory):**
- Replace secrets/credentials with `"[REDACTED]"`.
- Always redact keys matching (case-insensitive):
  `api_key`, `token`, `authorization`, `cookie`, `password`, `secret`,
  `private_key`, `credential`, `bearer`.
- For PII (email/phone/address), either redact or hash.
- Never log full file contents; log only sizes/paths/summary.

**Normalization for fixtures (deterministic diffs):**
- Replace `ts`, `pid`, `host`, `run_id`, `session_id`, `artifact_id`,
  `trace_id`, `span_id` with placeholders.
- Normalize absolute paths to `<cwd>/...`.
- Stable IDs (like `scenario_id`) must be deterministic and **not** randomized.

**Log sinks (documented contract):**
- **Runtime:** `~/.pi/agent/logs/extensions/<session_id>.jsonl`
  (override with `PI_EXTENSION_LOG_DIR`).
- **Capture:** `tests/ext_conformance/capture/<ext>/<scenario>/extension.log.jsonl`
- **Harness:** `target/ext_conformance/logs/<scenario_id>.jsonl`

**CI consumption:**
- CI should archive `target/ext_conformance/logs/**` as artifacts.
- Harness compares normalized logs to fixtures; diffs are triaged by `event`
  and `correlation` IDs.

---

### 3.2 Hostcall ABI (`host_call` / `host_result`)

`host_call` is the **only** way an extension requests privileged I/O from core.
Every call is explicit, capability-gated, and logged.

**`host_call.payload` fields (v1):**
- `call_id` (string, required): correlates request ↔ response(s).
- `capability` (string, required): the capability key evaluated by policy. **MUST**
  match the capability core derives from `method` + `params` (prevents spoofing).
- `method` (string, required): connector method name (e.g. `tool`, `exec`, `http`,
  `session`, `ui`, `log`).
- `params` (object, required): method-specific parameters.
- `timeout_ms` (int, optional): wall-clock timeout for the host operation.
- `cancel_token` (string, optional): idempotent cancellation handle (future).
- `context` (object, optional): free-form metadata (never used for policy decisions).

Example (`tool` call):
```json
{
  "call_id": "host-1",
  "capability": "read",
  "method": "tool",
  "params": { "name": "grep", "input": { "pattern": "TODO", "path": "src/" } },
  "timeout_ms": 2500
}
```

**Capability derivation (core-defined, v1):**
- For `method="tool"`, required capability is derived from `params.name`:
  - `read|grep|find|ls` → `read`
  - `write|edit` → `write`
  - `bash` → `exec`
  - unknown tool → `tool` (forces prompt/deny depending on policy)
- For other methods, required capability is the method itself (`http`, `exec`, etc).

**`host_result.payload` fields (v1):**
- `call_id` (string, required)
- `output` (object, required): method-specific result object (may be empty on error)
- `is_error` (bool, required)
- `error` (object, optional): required when `is_error=true`, forbidden otherwise
- `chunk` (object, optional): streaming metadata (when results are chunked)

Error example:
```json
{
  "call_id": "host-1",
  "output": {},
  "is_error": true,
  "error": {
    "code": "denied",
    "message": "capability denied by policy",
    "retryable": false,
    "details": { "capability": "exec" }
  }
}
```

**Error taxonomy (v1):**
- `timeout`: deadline reached.
- `denied`: capability not granted or out of scope.
- `io`: connector I/O failure (fs/network/process).
- `invalid_request`: malformed method/params/capability mismatch.
- `internal`: bug or invariant violation in the host.

**Streaming contract (v1):**
- Core may emit multiple `host_result` messages with the same `call_id`.
- When streaming, each message includes `chunk.index` starting at 0 and increasing
  by 1, and `chunk.is_last=true` marks the final chunk.
- `chunk.backpressure` is reserved for future flow-control hints.

---

### 3.3 Capability Manifest (`pi.ext.cap.v1`)

`register.payload.capability_manifest` optionally declares the extension’s
required capabilities up front so policy can prompt/deny deterministically and
the harness can validate conformance.

Schema (v1):
```json
{
  "schema": "pi.ext.cap.v1",
  "capabilities": [
    { "capability": "read", "methods": ["tool"], "scope": { "paths": ["src/**"] } },
    { "capability": "http", "methods": ["http"], "scope": { "hosts": ["api.github.com"] } }
  ]
}
```

Fields:
- `capabilities[].capability`: capability key (the same string used by policy and
  `host_call.payload.capability`).
- `capabilities[].methods` (optional): restrict to a set of connector methods
  that may be used with this capability (defense-in-depth).
- `capabilities[].scope` (optional):
  - `paths`: glob-like patterns relative to the project root/cwd.
  - `hosts`: allow-list of hostnames/domains for network calls.
  - `env`: allow-list of env var names (future connector).

Notes:
- `register.payload.capabilities` remains the legacy, flat list; it will be
  treated as a coarse capability set until all extensions emit a manifest.

---

### 3.4 Hostcall Evidence Ledger (per-call log contract)

For every hostcall the runtime emits an append-only evidence ledger using
`pi.ext.log.v1`:
- `host_call.start`: emitted immediately before dispatch
- `host_call.end`: emitted once on completion (success, error, or timeout)

**Required ledger fields (in `log.data`):**
- `capability` / `method`
- `params_hash` (sha256 hex)
- `timeout_ms` (if present)
- `duration_ms` (end event)
- `is_error` + `error.code` (end event, if error)

**`params_hash` canonicalization (v1):**
- Hash the canonical JSON serialization of:
  `{ "method": <method>, "params": <params> }`
- Canonical JSON rules: UTF-8, no whitespace, object keys sorted
  lexicographically, arrays preserve order.
- Never write raw `params` to logs (hash-only) unless explicitly allowed by a
  fixture or debug mode.

---

## 4. Capability Policy (Configurable Modes)

`extensions.policy.mode` supports:
- `strict`: deny by default, explicit grants required.
- `prompt`: ask once per capability.
- `permissive`: allow most; warn and log.

Suggested config (document‑only for now):
```json
{
  "extensions": {
    "policy": {
      "mode": "prompt",
      "max_memory_mb": 256,
      "default_caps": ["read", "write", "http"],
      "deny_caps": ["exec", "env"]
    }
  }
}
```

Capabilities are enforced per‑hostcall and logged in an **audit ledger**.

---

## 5. Alien‑Artifact Safety (Formal Decisioning)

We apply a **loss‑aware, evidence‑driven** model to decide capability grants.

**Evidence Ledger** (example):
```
E = { uses_fs: 0.8, uses_exec: 0.1, unsigned: 0.6, size_mb: 0.2 }
```

**Loss matrix** (risk‑averse):
```
           | grant | deny |
-----------+-------+------+
benign     |   0   |   2  |
malicious  | 100   |   1  |
```

Decision rule: grant if expected loss is lower. This supports **strict** and
**prompt** modes with mathematically traceable decisions.

> This is intentionally conservative: false‑deny is cheap; false‑grant is costly.

---

## 6. Conformance Harness

**Golden fixtures** record legacy behavior and validate the compiled artifact.

Process:
1. Capture legacy extension outputs → fixtures JSON.
2. Replay with the compiled artifact.
3. Compare outputs byte‑for‑byte (or normalized where specified).

Artifacts:
- `tests/ext_conformance/fixtures/*.json`
- `tests/ext_conformance/*.rs`

---

## 7. Performance Harness (Extreme Optimization Loop)

Benchmarks:
- Startup (`pi --version`)
- First tool call latency
- Streaming tool throughput

Loop:
1. **Baseline** (hyperfine)
2. **Profile** (flamegraph)
3. **Change one lever**
4. **Prove** isomorphism (golden outputs)
5. **Re‑profile**

---

## 8. Best‑Effort Compatibility Rules

Compatibility scanner outputs:
- **compatible** (safe)
- **warning** (works but constrained)
- **blocked** (unsafe / unsupported)

The system always **tries to run** with warnings unless `strict` is set.

---

## 9. Next Implementation Steps

1. Implement the protocol structs + JSON schema validation.
2. Implement the connector dispatcher + capability checks (works for JS/WASM/MCP).
3. Add the WASM host scaffold (component model) using the same connector layer.
4. Build the SWC‑based `extc` pipeline + cache.
5. Create conformance fixtures from legacy Pi extensions.

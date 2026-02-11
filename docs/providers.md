# Providers

This document is the canonical in-repo provider baseline for `pi_agent_rust`.
It summarizes provider IDs, aliases, API families, auth behavior, and current implementation mode.

Snapshot basis:
- `src/models.rs` (`built_in_models`, `ad_hoc_provider_defaults`)
- `src/auth.rs` (`env_keys_for_provider`)
- `src/providers/mod.rs` (`create_provider`, API fallback routing)
- `src/providers/*.rs` native implementations
- Timestamp: 2026-02-10

## Implementation Modes

| Mode | Meaning |
|------|---------|
| `native-implemented` | Provider has a direct runtime path in `create_provider` and is dispatchable now. |
| `native-partial` | Native module exists, but factory wiring or required config path is not fully integrated. |
| `oai-compatible-preset` | Provider resolves through OpenAI-compatible adapter (`openai-completions`) with preset base/auth defaults. |
| `alias-only` | Provider ID is a documented synonym of a canonical ID; no distinct runtime implementation. |
| `missing` | Provider ID is recognized in enums/auth mappings but has no usable runtime dispatch path yet. |

### Machine-Readable Classification (`bd-3uqg.1.4`)

Canonical planning artifact: `docs/provider-implementation-modes.json`

This JSON is the execution source-of-truth for provider onboarding mode selection:

| Mode | Planning Meaning |
|------|------------------|
| `native-adapter-required` | Requires dedicated runtime adapter path (protocol/auth/tool semantics not safely covered by generic OAI routing). |
| `oai-compatible-preset` | Can route through OpenAI-compatible adapter with provider-specific base/auth presets. |
| `gateway-wrapper-routing` | Acts as gateway/meta-router/alias-routing surface; prioritize routing-policy and diagnostics guarantees. |
| `deferred` | Explicitly not in current implementation wave; retained for planning completeness. |

Current artifact coverage (`docs/provider-implementation-modes.json`):
- 93 upstream union IDs classified (no gaps)
- 6 supplemental Pi alias IDs classified
- 99 total entries with explicit profile, rationale, and runtime status
- 20 high-risk providers carry explicit prerequisite beads + required diagnostic artifacts

## Verification Evidence Legend

- Metadata and alias/routing lock: [`tests/provider_metadata_comprehensive.rs`](../tests/provider_metadata_comprehensive.rs)
- Factory and adapter selection lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs)
- Native provider request-shape lock: [`tests/provider_backward_lock.rs`](../tests/provider_backward_lock.rs)
- Provider streaming contract suites: [`tests/provider_streaming.rs`](../tests/provider_streaming.rs)
- Live parity smoke lane: [`tests/e2e_cross_provider_parity.rs`](../tests/e2e_cross_provider_parity.rs)
- Live provider integration lane: [`tests/e2e_live.rs`](../tests/e2e_live.rs)

## Wave A Parity Verification (`bd-3uqg.4.4`)

Unit + request-shape verification for all currently tracked Wave A OpenAI-compatible preset IDs:
`groq`, `deepinfra`, `cerebras`, `openrouter`, `mistral`, `moonshotai`, `dashscope`, `deepseek`,
`fireworks`, `togetherai`, `perplexity`, `xai`, plus migration alias `fireworks-ai`.

Verification artifacts:
- Default/factory lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_a_presets_resolve_openai_compat_defaults_and_factory_route`)
- Streaming path/auth lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_a_openai_compat_streams_use_chat_completions_path_and_bearer_auth`)
- Alias migration lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`fireworks_ai_alias_migration_matches_fireworks_canonical_defaults`)

Provider-by-provider status (local verification via `cargo test --test provider_factory -- --nocapture`):

| Provider ID | Defaults + factory route lock | Streaming path/auth lock | Status |
|-------------|-------------------------------|--------------------------|--------|
| `groq` | yes | yes | pass |
| `deepinfra` | yes | yes | pass |
| `cerebras` | yes | yes | pass |
| `openrouter` | yes | yes | pass |
| `mistral` | yes | yes | pass |
| `moonshotai` | yes | yes | pass |
| `dashscope` | yes | yes | pass |
| `deepseek` | yes | yes | pass |
| `fireworks` | yes | yes | pass |
| `togetherai` | yes | yes | pass |
| `perplexity` | yes | yes | pass |
| `xai` | yes | yes | pass |
| `fireworks-ai` (alias) | yes | yes | pass |

Migration mapping decisions:
- `fireworks-ai` remains accepted as an alias of canonical `fireworks`.
- Route and auth behavior are parity-locked between `fireworks` and `fireworks-ai`.
- No compatibility shim layer is introduced; canonical configs should use `fireworks` going forward.

## Wave B1 Onboarding Verification (`bd-3uqg.5.2`)

Batch B1 provider IDs integrated and lock-tested:
`alibaba-cn`, `kimi-for-coding`, `minimax`, `minimax-cn`, `minimax-coding-plan`, `minimax-cn-coding-plan`.

Verification artifacts:
- Metadata + factory route lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b1_presets_resolve_metadata_defaults_and_factory_route`)
- OpenAI-compatible stream path/auth lock (`alibaba-cn`): [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b1_alibaba_cn_openai_compat_streams_use_chat_completions_path_and_bearer_auth`)
- Anthropic-compatible stream path/auth lock (`kimi-for-coding`, `minimax*`): [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b1_anthropic_compat_streams_use_messages_path_and_x_api_key`)
- Family coherence lock (`moonshot`/`kimi` alias vs `kimi-for-coding`, `alibaba` vs `alibaba-cn`): [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b1_family_coherence_with_existing_moonshot_and_alibaba_mappings`)
- Representative smoke/e2e artifacts (offline VCR harness): [`tests/provider_native_verify.rs`](../tests/provider_native_verify.rs) (`wave_b1_smoke::b1_alibaba_cn_*`, `wave_b1_smoke::b1_kimi_for_coding_*`, `wave_b1_smoke::b1_minimax_*`) with fixtures under [`tests/fixtures/vcr/verify_alibaba-cn_*.json`](../tests/fixtures/vcr/verify_alibaba-cn_simple_text.json), [`tests/fixtures/vcr/verify_kimi-for-coding_*.json`](../tests/fixtures/vcr/verify_kimi-for-coding_simple_text.json), and [`tests/fixtures/vcr/verify_minimax_*.json`](../tests/fixtures/vcr/verify_minimax_simple_text.json)

Provider-by-provider status (local verification via `cargo test --test provider_factory -- --nocapture`):

| Provider ID | API family | Route lock | Stream/auth lock | Status |
|-------------|------------|------------|------------------|--------|
| `alibaba-cn` | `openai-completions` | yes | yes | pass |
| `kimi-for-coding` | `anthropic-messages` | yes | yes | pass |
| `minimax` | `anthropic-messages` | yes | yes | pass |
| `minimax-cn` | `anthropic-messages` | yes | yes | pass |
| `minimax-coding-plan` | `anthropic-messages` | yes | yes | pass |
| `minimax-cn-coding-plan` | `anthropic-messages` | yes | yes | pass |

Representative smoke/e2e verification run:
- `cargo test --test provider_native_verify b1_ -- --nocapture`
- Passed: `b1_alibaba_cn_{simple_text,tool_call_single,error_auth_401}`,
  `b1_kimi_for_coding_{simple_text,tool_call_single,error_auth_401}`,
  `b1_minimax_{simple_text,tool_call_single,error_auth_401}`.

Canonical mapping decisions:
- `kimi` remains an alias of canonical `moonshotai`.
- `kimi-for-coding` is a distinct canonical ID and does not alias to `moonshotai`.
- `alibaba-cn` is distinct from `alibaba`/`dashscope`/`qwen` and uses CN DashScope routing defaults.
- `minimax-cn`, `minimax-coding-plan`, and `minimax-cn-coding-plan` inherit representative smoke coverage via
  shared family behavior plus explicit route/auth lock tests.

## Wave B2 Onboarding Verification (`bd-3uqg.5.1`)

Batch B2 provider IDs integrated and lock-tested:
`modelscope`, `moonshotai-cn`, `nebius`, `ovhcloud`, `scaleway`.

Verification artifacts:
- Metadata + factory route lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b2_presets_resolve_metadata_defaults_and_factory_route`)
- OpenAI-compatible stream path/auth lock: [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b2_openai_compat_streams_use_chat_completions_path_and_bearer_auth`)
- Family coherence lock (`moonshotai`/`moonshot` aliases vs `moonshotai-cn`): [`tests/provider_factory.rs`](../tests/provider_factory.rs) (`wave_b2_moonshot_cn_and_global_moonshot_mapping_are_distinct`)
- Representative smoke/e2e artifacts (offline VCR harness): [`tests/provider_native_verify.rs`](../tests/provider_native_verify.rs) (`wave_b2_smoke::b2_modelscope_*`, `wave_b2_smoke::b2_moonshotai_cn_*`, `wave_b2_smoke::b2_nebius_*`, `wave_b2_smoke::b2_ovhcloud_*`, `wave_b2_smoke::b2_scaleway_*`) with fixtures under [`tests/fixtures/vcr/verify_modelscope_*.json`](../tests/fixtures/vcr/verify_modelscope_simple_text.json), [`tests/fixtures/vcr/verify_moonshotai-cn_*.json`](../tests/fixtures/vcr/verify_moonshotai-cn_simple_text.json), [`tests/fixtures/vcr/verify_nebius_*.json`](../tests/fixtures/vcr/verify_nebius_simple_text.json), [`tests/fixtures/vcr/verify_ovhcloud_*.json`](../tests/fixtures/vcr/verify_ovhcloud_simple_text.json), and [`tests/fixtures/vcr/verify_scaleway_*.json`](../tests/fixtures/vcr/verify_scaleway_simple_text.json)

Provider-by-provider status (local verification via `cargo test --test provider_factory -- --nocapture`):

| Provider ID | API family | Route lock | Stream/auth lock | Status |
|-------------|------------|------------|------------------|--------|
| `modelscope` | `openai-completions` | yes | yes | pass |
| `moonshotai-cn` | `openai-completions` | yes | yes | pass |
| `nebius` | `openai-completions` | yes | yes | pass |
| `ovhcloud` | `openai-completions` | yes | yes | pass |
| `scaleway` | `openai-completions` | yes | yes | pass |

Representative smoke/e2e verification run:
- `cargo test --test provider_native_verify b2_ -- --nocapture`
- Passed: `b2_modelscope_{simple_text,tool_call_single,error_auth_401}`,
  `b2_moonshotai_cn_{simple_text,tool_call_single,error_auth_401}`,
  `b2_nebius_{simple_text,tool_call_single,error_auth_401}`,
  `b2_ovhcloud_{simple_text,tool_call_single,error_auth_401}`,
  `b2_scaleway_{simple_text,tool_call_single,error_auth_401}`.

Canonical mapping decisions:
- `modelscope`, `nebius`, `ovhcloud`, and `scaleway` are canonical OpenAI-compatible preset IDs.
- `moonshotai-cn` is a distinct canonical regional ID and does not alias to `moonshotai`.
- `moonshotai` and `moonshotai-cn` intentionally share `MOONSHOT_API_KEY` while retaining distinct base URLs.

## Canonical Provider Matrix (Current Baseline + Evidence Links)

| Canonical ID | Aliases | Capability flags | API family | Base URL template | Auth mode | Mode | Runtime status | Verification evidence (unit + e2e) |
|--------------|---------|------------------|------------|-------------------|-----------|------|----------------|------------------------------------|
| `anthropic` | - | text + image + thinking + tool-calls | `anthropic-messages` | `https://api.anthropic.com/v1/messages` | `x-api-key` (`ANTHROPIC_API_KEY`) or `auth.json` OAuth/API key | `native-implemented` | Implemented and dispatchable | [unit](../tests/provider_streaming/anthropic.rs), [contract](../tests/provider_backward_lock.rs), [e2e](../tests/e2e_provider_streaming.rs), [cassette](../tests/fixtures/vcr/anthropic_simple_text.json) |
| `openai` | - | text + image + reasoning + tool-calls | `openai-responses` (default), `openai-completions` (compat) | `https://api.openai.com/v1` (normalized to `/responses` or `/chat/completions`) | `Authorization: Bearer` (`OPENAI_API_KEY`) | `native-implemented` | Implemented and dispatchable | [unit](../tests/provider_streaming/openai.rs), [responses](../tests/provider_streaming/openai_responses.rs), [contract](../tests/provider_backward_lock.rs), [e2e](../tests/e2e_cross_provider_parity.rs), [cassette](../tests/fixtures/vcr/openai_simple_text.json) |
| `google` | `gemini` | text + image + reasoning + tool-calls | `google-generative-ai` | `https://generativelanguage.googleapis.com/v1beta` | query key (`GOOGLE_API_KEY`, fallback `GEMINI_API_KEY`) | `native-implemented` | Implemented and dispatchable | [unit](../tests/provider_streaming/gemini.rs), [contract](../tests/provider_backward_lock.rs), [e2e](../tests/e2e_cross_provider_parity.rs), [cassette](../tests/fixtures/vcr/gemini_simple_text.json) |
| `google-vertex` | `vertexai` | text + image + reasoning + tool-calls | `google-vertex` | `https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/{publisher}/models/{model}` | `Authorization: Bearer` (`GOOGLE_CLOUD_API_KEY`, alt `VERTEX_API_KEY`) | `native-implemented` | Implemented and dispatchable; supports Google (Gemini) and Anthropic publishers | [unit](../src/providers/vertex.rs), [factory](../src/providers/mod.rs), [metadata](../tests/provider_metadata_comprehensive.rs) |
| `cohere` | - | text + tool-calls | `cohere-chat` | `https://api.cohere.com/v2` (normalized to `/chat`) | `Authorization: Bearer` (`COHERE_API_KEY`) | `native-implemented` | Implemented and dispatchable | [unit](../tests/provider_streaming/cohere.rs), [contract](../tests/provider_backward_lock.rs), [cassette](../tests/fixtures/vcr/cohere_simple_text.json), e2e expansion tracked in `bd-3uqg.8.4` |
| `azure-openai` | `azure`, `azure-cognitive-services` | text + tool-calls | Azure chat/completions path | `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}` or `https://{resource}.cognitiveservices.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}` | `api-key` header (`AZURE_OPENAI_API_KEY`) | `native-implemented` | Dispatchable through provider factory with deterministic resource/deployment/api-version resolution from env + model/base_url | [unit](../tests/provider_streaming/azure.rs), [contract](../tests/provider_backward_lock.rs), [e2e](../tests/e2e_live.rs), [cassette](../tests/fixtures/vcr/azure_simple_text.json) |
| `groq` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.groq.com/openai/v1` | `Authorization: Bearer` (`GROQ_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `deepinfra` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.deepinfra.com/v1/openai` | `Authorization: Bearer` (`DEEPINFRA_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `cerebras` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.cerebras.ai/v1` | `Authorization: Bearer` (`CEREBRAS_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `openrouter` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://openrouter.ai/api/v1` | `Authorization: Bearer` (`OPENROUTER_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [e2e](../tests/e2e_cross_provider_parity.rs) |
| `mistral` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.mistral.ai/v1` | `Authorization: Bearer` (`MISTRAL_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `moonshotai` | `moonshot`, `kimi` | text (+ OAI-compatible tools) | `openai-completions` | `https://api.moonshot.ai/v1` | `Authorization: Bearer` (`MOONSHOT_API_KEY`) | `oai-compatible-preset` (`moonshot`,`kimi` are `alias-only`) | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [alias-roundtrip](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `moonshotai-cn` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.moonshot.cn/v1` | `Authorization: Bearer` (`MOONSHOT_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_moonshotai-cn_simple_text.json) |
| `kimi-for-coding` | - | text + image (Anthropic-compatible) | `anthropic-messages` | `https://api.kimi.com/coding/v1/messages` | `x-api-key` (`KIMI_API_KEY`) | `oai-compatible-preset` (preset fallback) | Dispatchable through Anthropic API fallback route | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_kimi-for-coding_simple_text.json) |
| `dashscope` | `alibaba`, `qwen` | text (+ OAI-compatible tools) | `openai-completions` | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | `Authorization: Bearer` (`DASHSCOPE_API_KEY`) | `oai-compatible-preset` (`alibaba`,`qwen` are `alias-only`) | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `alibaba-cn` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `Authorization: Bearer` (`DASHSCOPE_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_alibaba-cn_simple_text.json) |
| `modelscope` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api-inference.modelscope.cn/v1` | `Authorization: Bearer` (`MODELSCOPE_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_modelscope_simple_text.json) |
| `nebius` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.tokenfactory.nebius.com/v1` | `Authorization: Bearer` (`NEBIUS_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_nebius_simple_text.json) |
| `ovhcloud` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://oai.endpoints.kepler.ai.cloud.ovh.net/v1` | `Authorization: Bearer` (`OVHCLOUD_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_ovhcloud_simple_text.json) |
| `scaleway` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.scaleway.ai/v1` | `Authorization: Bearer` (`SCALEWAY_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_scaleway_simple_text.json) |
| `deepseek` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.deepseek.com` | `Authorization: Bearer` (`DEEPSEEK_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [e2e](../tests/e2e_cross_provider_parity.rs) |
| `fireworks` | `fireworks-ai` | text (+ OAI-compatible tools) | `openai-completions` | `https://api.fireworks.ai/inference/v1` | `Authorization: Bearer` (`FIREWORKS_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `togetherai` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.together.xyz/v1` | `Authorization: Bearer` (`TOGETHER_API_KEY`, alt `TOGETHER_AI_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `perplexity` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.perplexity.ai` | `Authorization: Bearer` (`PERPLEXITY_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), e2e expansion tracked in `bd-3uqg.8.4` |
| `xai` | - | text (+ OAI-compatible tools) | `openai-completions` | `https://api.x.ai/v1` | `Authorization: Bearer` (`XAI_API_KEY`) | `oai-compatible-preset` | Dispatchable through OpenAI-compatible fallback | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [e2e](../tests/e2e_cross_provider_parity.rs) |
| `minimax` | - | text (Anthropic-compatible) | `anthropic-messages` | `https://api.minimax.io/anthropic/v1/messages` | `x-api-key` (`MINIMAX_API_KEY`) | `oai-compatible-preset` (preset fallback) | Dispatchable through Anthropic API fallback route | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), [native-verify harness](../tests/provider_native_verify.rs), [cassette](../tests/fixtures/vcr/verify_minimax_simple_text.json) |
| `minimax-cn` | - | text (Anthropic-compatible) | `anthropic-messages` | `https://api.minimaxi.com/anthropic/v1/messages` | `x-api-key` (`MINIMAX_CN_API_KEY`) | `oai-compatible-preset` (preset fallback) | Dispatchable through Anthropic API fallback route | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), family representative smoke via [`verify_minimax_simple_text.json`](../tests/fixtures/vcr/verify_minimax_simple_text.json) |
| `minimax-coding-plan` | - | text (Anthropic-compatible) | `anthropic-messages` | `https://api.minimax.io/anthropic/v1/messages` | `x-api-key` (`MINIMAX_API_KEY`) | `oai-compatible-preset` (preset fallback) | Dispatchable through Anthropic API fallback route | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), family representative smoke via [`verify_minimax_simple_text.json`](../tests/fixtures/vcr/verify_minimax_simple_text.json) |
| `minimax-cn-coding-plan` | - | text (Anthropic-compatible) | `anthropic-messages` | `https://api.minimaxi.com/anthropic/v1/messages` | `x-api-key` (`MINIMAX_CN_API_KEY`) | `oai-compatible-preset` (preset fallback) | Dispatchable through Anthropic API fallback route | [metadata](../tests/provider_metadata_comprehensive.rs), [factory](../tests/provider_factory.rs), family representative smoke via [`verify_minimax_simple_text.json`](../tests/fixtures/vcr/verify_minimax_simple_text.json) |

## Missing/Partial IDs in Current Runtime

Provider IDs already recognized in auth/enums but not yet fully dispatchable:

| ID | Current state | Rationale | Risk | Follow-up beads | Current evidence |
|----|---------------|-----------|------|-----------------|------------------|
| `google-vertex` (`vertexai`) | `native-implemented` (`bd-3uqg.3.1` closed) | Native Vertex AI adapter is dispatchable with streaming for both Google (Gemini) and Anthropic publishers. | Resolved | — | [unit](../src/providers/vertex.rs), [factory](../src/providers/mod.rs), [metadata](../tests/provider_metadata_comprehensive.rs) |
| `amazon-bedrock` (`bedrock`) | `missing` (enum/env mapping only) | Bedrock Converse semantics need a native adapter path and credential-chain validation. | High: AWS users cannot route through first-class runtime path. | `bd-3uqg.3.3`, `bd-3uqg.3.8.2` | [metadata](../tests/provider_metadata_comprehensive.rs), [planning profile](provider-implementation-modes.json) |
| `github-copilot` (`copilot`) | `missing` (enum/env mapping only) | Provider protocol/auth flow is not implemented in native runtime path. | High: Copilot IDs remain non-dispatchable despite auth/env mapping. | `bd-3uqg.3.2`, `bd-3uqg.3.8.2` | [metadata](../tests/provider_metadata_comprehensive.rs), [planning profile](provider-implementation-modes.json) |

Full deferred/high-risk inventory (including rationale text for all classified IDs) lives in `docs/provider-implementation-modes.json`.

## Already-Covered vs Missing Snapshot

Covered now:
- 5 native dispatchable providers: `anthropic`, `openai`, `google`, `cohere`, `azure-openai`.
- 12 OpenAI-compatible preset providers dispatchable via fallback adapters:
  `groq`, `deepinfra`, `cerebras`, `openrouter`, `mistral`, `moonshotai`, `dashscope`,
  `deepseek`, `fireworks`, `togetherai`, `perplexity`, `xai`.
- 6 Wave B1 regional/coding-plan providers are now dispatchable with preset fallback defaults:
  `alibaba-cn` via `openai-completions`; `kimi-for-coding`, `minimax`, `minimax-cn`,
  `minimax-coding-plan`, `minimax-cn-coding-plan` via `anthropic-messages`.
- 5 Wave B2 regional/cloud providers are now dispatchable with preset fallback defaults:
  `modelscope`, `moonshotai-cn`, `nebius`, `ovhcloud`, and `scaleway` via `openai-completions`.
- Alias coverage built into preset defaults:
  `moonshot`/`kimi` -> `moonshotai`, and `alibaba`/`qwen` -> `dashscope`.

Not fully covered yet:
- 3 recognized-but-missing paths: `google-vertex`, `amazon-bedrock`, `github-copilot`.
- Additional upstream IDs from `models.dev + opencode + code` remain to be classified in the
  frozen upstream snapshot workflow (`bd-3uqg.1.1`).

## Provider Selection and Configuration

Credential resolution precedence (runtime):
1. explicit CLI override (`--api-key`)
2. provider env vars from metadata (ordered; includes shared fallbacks like `GOOGLE_API_KEY` then `GEMINI_API_KEY`)
3. persisted `auth.json` credential (`ApiKey` or unexpired OAuth `access_token`)
4. inline `models.json` `apiKey` fallback (resolved from literal/env/file/shell sources)

Choose provider/model via:
- CLI flags: `pi --provider openai --model gpt-4o "Hello"`
- Env vars: `PI_PROVIDER`, `PI_MODEL`
- Settings: `default_provider`, `default_model` in `~/.pi/agent/settings.json`

Custom endpoints and overrides should be configured in `models.json`:
- See [models.md](models.md) for schema and examples.

Example key exports:

```bash
export ANTHROPIC_API_KEY="..."
export OPENAI_API_KEY="..."
export GOOGLE_API_KEY="..."
export COHERE_API_KEY="..."
```

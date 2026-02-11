//! Comprehensive provider metadata and factory routing tests.
//!
//! Covers canonical ID resolution, alias mapping, routing defaults,
//! factory dispatch, and structural invariants for every provider
//! in `PROVIDER_METADATA`.
//!
//! bd-3uqg.8.1

mod common;

use pi::provider_metadata::{
    PROVIDER_METADATA, ProviderOnboardingMode, canonical_provider_id, provider_auth_env_keys,
    provider_metadata, provider_routing_defaults,
};
use std::collections::{HashMap, HashSet};

// ═══════════════════════════════════════════════════════════════════════
// Structural invariants
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn all_canonical_ids_are_unique() {
    let mut seen = HashSet::new();
    for meta in PROVIDER_METADATA {
        assert!(
            seen.insert(meta.canonical_id),
            "duplicate canonical_id: {}",
            meta.canonical_id
        );
    }
}

#[test]
fn no_alias_collides_with_canonical_id() {
    let canonicals: HashSet<&str> = PROVIDER_METADATA.iter().map(|m| m.canonical_id).collect();
    for meta in PROVIDER_METADATA {
        for alias in meta.aliases {
            // An alias may NOT shadow a different canonical_id (it would create ambiguity).
            if let Some(other) = provider_metadata(alias) {
                assert_eq!(
                    other.canonical_id, meta.canonical_id,
                    "alias '{}' resolves to '{}' but belongs to '{}'",
                    alias, other.canonical_id, meta.canonical_id
                );
            }
        }
    }
    // Also confirm no alias is another entry's canonical_id (unless they share the same entry).
    for meta in PROVIDER_METADATA {
        for alias in meta.aliases {
            if canonicals.contains(alias) {
                // Must be the same entry's canonical_id (self-alias) - currently not used, but guard.
                assert_eq!(
                    *alias, meta.canonical_id,
                    "alias '{alias}' shadows canonical_id of a different provider"
                );
            }
        }
    }
}

#[test]
fn no_duplicate_aliases_across_entries() {
    let mut alias_to_canonical: HashMap<&str, &str> = HashMap::new();
    for meta in PROVIDER_METADATA {
        for alias in meta.aliases {
            if let Some(prev) = alias_to_canonical.insert(alias, meta.canonical_id) {
                assert_eq!(
                    prev, meta.canonical_id,
                    "alias '{}' claimed by both '{}' and '{}'",
                    alias, prev, meta.canonical_id
                );
            }
        }
    }
}

#[test]
fn every_canonical_id_is_lowercase_trimmed() {
    for meta in PROVIDER_METADATA {
        assert_eq!(
            meta.canonical_id,
            meta.canonical_id.trim(),
            "canonical_id '{}' has leading/trailing whitespace",
            meta.canonical_id
        );
        assert_eq!(
            meta.canonical_id,
            meta.canonical_id.to_lowercase(),
            "canonical_id '{}' must be lowercase",
            meta.canonical_id
        );
    }
}

#[test]
fn every_alias_is_lowercase_trimmed() {
    for meta in PROVIDER_METADATA {
        for alias in meta.aliases {
            assert_eq!(
                *alias,
                alias.trim(),
                "alias '{}' (of '{}') has whitespace",
                alias,
                meta.canonical_id
            );
            assert_eq!(
                *alias,
                alias.to_lowercase(),
                "alias '{}' (of '{}') must be lowercase",
                alias,
                meta.canonical_id
            );
        }
    }
}

#[test]
fn every_provider_has_at_least_one_auth_env_key() {
    for meta in PROVIDER_METADATA {
        assert!(
            !meta.auth_env_keys.is_empty(),
            "provider '{}' has no auth env keys",
            meta.canonical_id
        );
    }
}

#[test]
fn auth_env_keys_are_screaming_snake_case() {
    for meta in PROVIDER_METADATA {
        for key in meta.auth_env_keys {
            assert!(
                key.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "env key '{}' (provider '{}') must be SCREAMING_SNAKE_CASE",
                key,
                meta.canonical_id
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Canonical ID / alias resolution
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn every_canonical_id_resolves_to_itself() {
    for meta in PROVIDER_METADATA {
        let resolved = canonical_provider_id(meta.canonical_id);
        assert_eq!(
            resolved,
            Some(meta.canonical_id),
            "canonical_provider_id('{}') should return itself",
            meta.canonical_id
        );
    }
}

#[test]
fn every_alias_resolves_to_its_canonical_id() {
    for meta in PROVIDER_METADATA {
        for alias in meta.aliases {
            let resolved = canonical_provider_id(alias);
            assert_eq!(
                resolved,
                Some(meta.canonical_id),
                "alias '{}' should resolve to '{}'",
                alias,
                meta.canonical_id
            );
        }
    }
}

#[test]
fn auth_env_keys_accessible_via_aliases() {
    for meta in PROVIDER_METADATA {
        let canonical_keys = provider_auth_env_keys(meta.canonical_id);
        for alias in meta.aliases {
            let alias_keys = provider_auth_env_keys(alias);
            assert_eq!(
                canonical_keys, alias_keys,
                "auth env keys for alias '{}' must match canonical '{}'",
                alias, meta.canonical_id
            );
        }
    }
}

#[test]
fn unknown_provider_returns_none() {
    assert!(provider_metadata("nonexistent-provider-xyz").is_none());
    assert!(canonical_provider_id("nonexistent-provider-xyz").is_none());
    assert!(provider_routing_defaults("nonexistent-provider-xyz").is_none());
}

// ═══════════════════════════════════════════════════════════════════════
// Routing defaults invariants
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn oai_compatible_providers_have_routing_defaults() {
    for meta in PROVIDER_METADATA {
        if meta.onboarding == ProviderOnboardingMode::OpenAICompatiblePreset {
            assert!(
                meta.routing_defaults.is_some(),
                "OAI-compatible provider '{}' must have routing_defaults",
                meta.canonical_id
            );
        }
    }
}

#[test]
fn native_adapter_providers_routing_defaults_are_consistent() {
    // Native adapter providers MAY have routing_defaults (for context_window,
    // max_tokens, etc.) but their base_url is typically empty because they
    // construct URLs from provider-specific config (project/region/deployment).
    for meta in PROVIDER_METADATA {
        if meta.onboarding == ProviderOnboardingMode::NativeAdapterRequired {
            if let Some(defaults) = &meta.routing_defaults {
                // Native providers with routing_defaults should have an
                // API identifier (non-empty).
                assert!(
                    !defaults.api.is_empty(),
                    "native-adapter provider '{}' has routing_defaults but empty api",
                    meta.canonical_id
                );
            }
        }
    }
}

#[test]
fn all_oai_compatible_base_urls_are_nonempty() {
    // Native providers (BuiltInNative, NativeAdapterRequired) may have empty
    // base_url because they construct endpoints from provider-specific config.
    // Only OpenAI-compatible presets require a non-empty base_url.
    for meta in PROVIDER_METADATA {
        if meta.onboarding == ProviderOnboardingMode::OpenAICompatiblePreset {
            if let Some(defaults) = &meta.routing_defaults {
                assert!(
                    !defaults.base_url.is_empty(),
                    "OAI-compatible provider '{}' has empty base_url",
                    meta.canonical_id
                );
            }
        }
    }
}

#[test]
fn all_oai_compatible_base_urls_are_unique() {
    let mut url_to_provider: HashMap<&str, &str> = HashMap::new();
    let shared_endpoint_pairs: HashSet<(&str, &str)> = HashSet::from([
        ("minimax", "minimax-coding-plan"),
        ("minimax-coding-plan", "minimax"),
        ("minimax-cn", "minimax-cn-coding-plan"),
        ("minimax-cn-coding-plan", "minimax-cn"),
    ]);
    for meta in PROVIDER_METADATA {
        if let Some(defaults) = &meta.routing_defaults {
            // Skip empty base_urls (native providers construct URLs differently).
            if defaults.base_url.is_empty() {
                continue;
            }
            if let Some(prev) = url_to_provider.insert(defaults.base_url, meta.canonical_id) {
                assert!(
                    shared_endpoint_pairs.contains(&(prev, meta.canonical_id)),
                    "base_url '{}' used by both '{}' and '{}'",
                    defaults.base_url,
                    prev,
                    meta.canonical_id
                );
            }
        }
    }
}

#[test]
fn oai_compatible_defaults_use_known_api_family() {
    let known_apis = [
        "openai-completions",
        "openai-responses",
        "anthropic-messages",
        "cohere-chat",
        "google-generative-ai",
        // Native provider API families:
        "google-vertex",
        "bedrock-converse-stream",
        "gitlab-chat",
        "copilot-openai",
    ];
    for meta in PROVIDER_METADATA {
        if let Some(defaults) = &meta.routing_defaults {
            assert!(
                known_apis.contains(&defaults.api),
                "provider '{}' has unknown api '{}', expected one of {:?}",
                meta.canonical_id,
                defaults.api,
                known_apis
            );
        }
    }
}

#[test]
fn context_window_and_max_tokens_are_positive() {
    for meta in PROVIDER_METADATA {
        if let Some(defaults) = &meta.routing_defaults {
            assert!(
                defaults.context_window > 0,
                "provider '{}' context_window must be > 0",
                meta.canonical_id
            );
            assert!(
                defaults.max_tokens > 0,
                "provider '{}' max_tokens must be > 0",
                meta.canonical_id
            );
            assert!(
                defaults.max_tokens <= defaults.context_window,
                "provider '{}' max_tokens ({}) exceeds context_window ({})",
                meta.canonical_id,
                defaults.max_tokens,
                defaults.context_window
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Factory routing: every provider dispatches without error
// ═══════════════════════════════════════════════════════════════════════

/// Helper: build a `ModelEntry` for an OAI-compatible provider.
fn oai_entry(provider: &str, base_url: &str) -> pi::models::ModelEntry {
    use pi::provider::{InputType, Model, ModelCost};
    pi::models::ModelEntry {
        model: Model {
            id: "test-model".to_string(),
            name: "Test Model".to_string(),
            api: "openai-completions".to_string(),
            provider: provider.to_string(),
            base_url: base_url.to_string(),
            reasoning: false,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.001,
                output: 0.002,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128_000,
            max_tokens: 16_384,
            headers: std::collections::HashMap::new(),
        },
        api_key: Some("test-key".to_string()),
        headers: std::collections::HashMap::new(),
        auth_header: true,
        compat: None,
        oauth_config: None,
    }
}

#[test]
fn factory_dispatches_every_oai_compatible_provider() {
    use pi::providers::create_provider;

    for meta in PROVIDER_METADATA {
        if meta.onboarding != ProviderOnboardingMode::OpenAICompatiblePreset {
            continue;
        }
        let defaults = meta
            .routing_defaults
            .expect("OAI provider must have defaults");
        let entry = oai_entry(meta.canonical_id, defaults.base_url);
        let provider = create_provider(&entry, None)
            .unwrap_or_else(|e| panic!("factory failed for '{}': {e}", meta.canonical_id));
        assert_eq!(
            provider.api(),
            defaults.api,
            "factory api mismatch for '{}'",
            meta.canonical_id
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn factory_dispatches_native_established_providers() {
    use pi::providers::create_provider;

    // Anthropic
    let anthropic_entry = {
        use pi::provider::{InputType, Model, ModelCost};
        pi::models::ModelEntry {
            model: Model {
                id: "claude-sonnet-4-5".to_string(),
                name: "Claude Sonnet".to_string(),
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                reasoning: false,
                input: vec![InputType::Text],
                cost: ModelCost {
                    input: 0.003,
                    output: 0.015,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 200_000,
                max_tokens: 8_192,
                headers: std::collections::HashMap::new(),
            },
            api_key: Some("test-key".to_string()),
            headers: std::collections::HashMap::new(),
            auth_header: true,
            compat: None,
            oauth_config: None,
        }
    };
    let p = create_provider(&anthropic_entry, None).expect("anthropic factory");
    assert_eq!(p.api(), "anthropic-messages");

    // Google/Gemini
    let google_entry = {
        use pi::provider::{InputType, Model, ModelCost};
        pi::models::ModelEntry {
            model: Model {
                id: "gemini-2.0-flash".to_string(),
                name: "Gemini Flash".to_string(),
                api: "google-generative-ai".to_string(),
                provider: "google".to_string(),
                base_url: "https://generativelanguage.googleapis.com".to_string(),
                reasoning: false,
                input: vec![InputType::Text],
                cost: ModelCost {
                    input: 0.001,
                    output: 0.004,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 1_000_000,
                max_tokens: 8_192,
                headers: std::collections::HashMap::new(),
            },
            api_key: Some("test-key".to_string()),
            headers: std::collections::HashMap::new(),
            auth_header: true,
            compat: None,
            oauth_config: None,
        }
    };
    let p = create_provider(&google_entry, None).expect("google factory");
    assert_eq!(p.api(), "google-generative-ai");

    // Cohere
    let cohere_entry = {
        use pi::provider::{InputType, Model, ModelCost};
        pi::models::ModelEntry {
            model: Model {
                id: "command-r-plus".to_string(),
                name: "Command R+".to_string(),
                api: "cohere-chat".to_string(),
                provider: "cohere".to_string(),
                base_url: "https://api.cohere.com".to_string(),
                reasoning: false,
                input: vec![InputType::Text],
                cost: ModelCost {
                    input: 0.003,
                    output: 0.015,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 128_000,
                max_tokens: 4_096,
                headers: std::collections::HashMap::new(),
            },
            api_key: Some("test-key".to_string()),
            headers: std::collections::HashMap::new(),
            auth_header: true,
            compat: None,
            oauth_config: None,
        }
    };
    let p = create_provider(&cohere_entry, None).expect("cohere factory");
    assert_eq!(p.api(), "cohere-chat");

    // Amazon Bedrock
    let bedrock_entry = {
        use pi::provider::{InputType, Model, ModelCost};
        pi::models::ModelEntry {
            model: Model {
                id: "anthropic.claude-3-5-sonnet-20240620-v1:0".to_string(),
                name: "Claude Sonnet via Bedrock".to_string(),
                api: "bedrock-converse-stream".to_string(),
                provider: "amazon-bedrock".to_string(),
                base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
                reasoning: true,
                input: vec![InputType::Text],
                cost: ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 200_000,
                max_tokens: 8_192,
                headers: std::collections::HashMap::new(),
            },
            api_key: Some("test-bedrock-token".to_string()),
            headers: std::collections::HashMap::new(),
            auth_header: false,
            compat: None,
            oauth_config: None,
        }
    };
    let p = create_provider(&bedrock_entry, None).expect("bedrock factory");
    assert_eq!(p.api(), "bedrock-converse-stream");
}

// ═══════════════════════════════════════════════════════════════════════
// Coverage assertion: provider count
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn provider_metadata_count_is_at_least_56() {
    // Guard against accidental removal of entries.
    assert!(
        PROVIDER_METADATA.len() >= 56,
        "expected at least 56 provider entries, found {}",
        PROVIDER_METADATA.len()
    );
}

#[test]
fn total_aliases_count_is_consistent() {
    let total_aliases: usize = PROVIDER_METADATA.iter().map(|m| m.aliases.len()).sum();
    // Sanity check: we have known aliases (gemini, fireworks-ai, kimi, moonshot,
    // dashscope, qwen, vertexai, bedrock, sap, azure, azure-cognitive-services,
    // copilot, gitlab-duo). At least 13.
    assert!(
        total_aliases >= 13,
        "expected at least 13 aliases, found {total_aliases}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Artifact generation: canonical ID + alias table (JSON)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn generate_canonical_id_alias_table_json() {
    use serde_json::{Value, json};

    let mut entries: Vec<Value> = Vec::new();
    for meta in PROVIDER_METADATA {
        let onboarding = match meta.onboarding {
            ProviderOnboardingMode::BuiltInNative => "built-in-native",
            ProviderOnboardingMode::NativeAdapterRequired => "native-adapter-required",
            ProviderOnboardingMode::OpenAICompatiblePreset => "oai-compatible-preset",
        };

        let mut entry = json!({
            "canonical_id": meta.canonical_id,
            "aliases": meta.aliases,
            "onboarding_mode": onboarding,
            "auth_env_keys": meta.auth_env_keys,
        });

        if let Some(defaults) = &meta.routing_defaults {
            entry["routing"] = json!({
                "api": defaults.api,
                "base_url": defaults.base_url,
                "auth_header": defaults.auth_header,
                "context_window": defaults.context_window,
                "max_tokens": defaults.max_tokens,
            });
        }

        entries.push(entry);
    }

    let table = json!({
        "schema_version": "1.0",
        "bead_id": "bd-3uqg.9.1.1",
        "description": "Canonical provider ID + alias table generated from PROVIDER_METADATA",
        "total_providers": PROVIDER_METADATA.len(),
        "total_aliases": PROVIDER_METADATA.iter().map(|m| m.aliases.len()).sum::<usize>(),
        "providers": entries,
    });

    let json_str = serde_json::to_string_pretty(&table).expect("JSON serialization");

    // Write to docs directory
    let path = std::path::Path::new("docs/provider-canonical-id-table.json");
    std::fs::write(path, &json_str).expect("write canonical ID table");

    // Verify the file round-trips
    let readback = std::fs::read_to_string(path).expect("read back");
    let parsed: Value = serde_json::from_str(&readback).expect("parse back");
    assert_eq!(
        usize::try_from(parsed["total_providers"].as_u64().unwrap()).unwrap(),
        PROVIDER_METADATA.len()
    );
}

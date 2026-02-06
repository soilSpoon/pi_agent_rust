//! Model registry: built-in + models.json overrides.

use crate::auth::AuthStorage;
use crate::error::Error;
use crate::provider::{Api, InputType, Model, ModelCost};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub model: Model,
    pub api_key: Option<String>,
    pub headers: HashMap<String, String>,
    pub auth_header: bool,
    pub compat: Option<CompatConfig>,
    /// OAuth config for extension-registered providers that require browser-based auth.
    pub oauth_config: Option<OAuthConfig>,
}

/// OAuth configuration for extension-registered providers.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsConfig {
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub base_url: Option<String>,
    pub api: Option<String>,
    pub api_key: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub auth_header: Option<bool>,
    pub compat: Option<CompatConfig>,
    pub models: Option<Vec<ModelConfig>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub id: String,
    pub name: Option<String>,
    pub api: Option<String>,
    pub reasoning: Option<bool>,
    pub input: Option<Vec<String>>,
    pub cost: Option<ModelCost>,
    pub context_window: Option<u32>,
    pub max_tokens: Option<u32>,
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<CompatConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatConfig {
    pub supports_store: Option<bool>,
    pub supports_developer_role: Option<bool>,
    pub supports_reasoning_effort: Option<bool>,
    pub supports_usage_in_streaming: Option<bool>,
    pub max_tokens_field: Option<String>,
    pub open_router_routing: Option<serde_json::Value>,
    pub vercel_gateway_routing: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: Vec<ModelEntry>,
    error: Option<String>,
}

impl ModelRegistry {
    pub fn load(auth: &AuthStorage, models_path: Option<PathBuf>) -> Self {
        let mut models = built_in_models(auth);
        let mut error = None;

        if let Some(path) = models_path {
            if path.exists() {
                match std::fs::read_to_string(&path)
                    .map_err(|e| Error::config(format!("Failed to read models.json: {e}")))
                    .and_then(|s| serde_json::from_str::<ModelsConfig>(&s).map_err(Error::from))
                {
                    Ok(config) => {
                        apply_custom_models(auth, &mut models, &config);
                    }
                    Err(e) => {
                        error = Some(format!("{e}\n\nFile: {}", path.display()));
                    }
                }
            }
        }

        Self { models, error }
    }

    pub fn models(&self) -> &[ModelEntry] {
        &self.models
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn get_available(&self) -> Vec<ModelEntry> {
        self.models
            .iter()
            .filter(|&m| m.api_key.is_some())
            .cloned()
            .collect()
    }

    pub fn find(&self, provider: &str, id: &str) -> Option<ModelEntry> {
        self.models
            .iter()
            .find(|m| m.model.provider == provider && m.model.id == id)
            .cloned()
    }

    /// Find a model by ID alone (ignoring provider), useful for extension models
    /// where the provider name may be custom.
    pub fn find_by_id(&self, id: &str) -> Option<ModelEntry> {
        self.models.iter().find(|m| m.model.id == id).cloned()
    }

    /// Merge extension-provided model entries into the registry.
    pub fn merge_entries(&mut self, entries: Vec<ModelEntry>) {
        for entry in entries {
            // Skip duplicates (same provider + id).
            let exists = self
                .models
                .iter()
                .any(|m| m.model.provider == entry.model.provider && m.model.id == entry.model.id);
            if !exists {
                self.models.push(entry);
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn built_in_models(auth: &AuthStorage) -> Vec<ModelEntry> {
    let mut models = Vec::new();

    let anthropic_key = auth.resolve_api_key("anthropic", None);
    for (id, name, reasoning) in [
        ("claude-sonnet-4-20250514", "Claude Sonnet 4", true),
        ("claude-sonnet-4-5", "Claude Sonnet 4.5", true),
        ("claude-opus-4-5", "Claude Opus 4.5", true),
        ("claude-haiku-4-5", "Claude Haiku 4.5", false),
        ("claude-3-5-sonnet-20241022", "Claude Sonnet 3.5", true),
        ("claude-3-5-haiku-20241022", "Claude Haiku 3.5", false),
        ("claude-3-opus-20240229", "Claude Opus 3", true),
    ] {
        models.push(ModelEntry {
            model: Model {
                id: id.to_string(),
                name: name.to_string(),
                api: Api::AnthropicMessages.to_string(),
                provider: "anthropic".to_string(),
                base_url: "https://api.anthropic.com/v1/messages".to_string(),
                reasoning,
                input: vec![InputType::Text, InputType::Image],
                cost: ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 200_000,
                max_tokens: 8192,
                headers: HashMap::new(),
            },
            api_key: anthropic_key.clone(),
            headers: HashMap::new(),
            auth_header: false,
            compat: None,
            oauth_config: None,
        });
    }

    let openai_key = auth.resolve_api_key("openai", None);
    for (id, name) in [
        ("gpt-5.1-codex", "GPT-5.1 Codex"),
        ("gpt-4o", "GPT-4o"),
        ("gpt-4o-mini", "GPT-4o Mini"),
    ] {
        models.push(ModelEntry {
            model: Model {
                id: id.to_string(),
                name: name.to_string(),
                api: Api::OpenAIResponses.to_string(),
                provider: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                reasoning: true,
                input: vec![InputType::Text, InputType::Image],
                cost: ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 128_000,
                max_tokens: 16384,
                headers: HashMap::new(),
            },
            api_key: openai_key.clone(),
            headers: HashMap::new(),
            auth_header: true,
            compat: None,
            oauth_config: None,
        });
    }

    let google_key = auth.resolve_api_key("google", None);
    for (id, name) in [
        ("gemini-2.5-pro", "Gemini 2.5 Pro"),
        ("gemini-2.5-flash", "Gemini 2.5 Flash"),
        ("gemini-1.5-pro", "Gemini 1.5 Pro"),
        ("gemini-1.5-flash", "Gemini 1.5 Flash"),
    ] {
        models.push(ModelEntry {
            model: Model {
                id: id.to_string(),
                name: name.to_string(),
                api: Api::GoogleGenerativeAI.to_string(),
                provider: "google".to_string(),
                base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                reasoning: true,
                input: vec![InputType::Text, InputType::Image],
                cost: ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 128_000,
                max_tokens: 8192,
                headers: HashMap::new(),
            },
            api_key: google_key.clone(),
            headers: HashMap::new(),
            auth_header: false,
            compat: None,
            oauth_config: None,
        });
    }

    models
}

#[allow(clippy::too_many_lines)]
fn apply_custom_models(auth: &AuthStorage, models: &mut Vec<ModelEntry>, config: &ModelsConfig) {
    for (provider_id, provider_cfg) in &config.providers {
        let provider_api = provider_cfg.api.as_deref().unwrap_or("openai-completions");
        let provider_api_parsed: Api = provider_api
            .parse()
            .unwrap_or_else(|_| Api::Custom(provider_api.to_string()));
        let provider_api_string = provider_api_parsed.to_string();
        let provider_base = provider_cfg
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let provider_headers = resolve_headers(provider_cfg.headers.as_ref());
        let provider_key = provider_cfg
            .api_key
            .as_deref()
            .and_then(resolve_value)
            .or_else(|| auth.resolve_api_key(provider_id, None));

        let auth_header = provider_cfg.auth_header.unwrap_or(false);

        let has_models = provider_cfg.models.as_ref().is_some();
        let is_override = !has_models;

        if is_override {
            for entry in models
                .iter_mut()
                .filter(|m| m.model.provider == *provider_id)
            {
                // Only override base_url and api if explicitly set in models.json.
                // Otherwise keep the built-in defaults (e.g. anthropic's /v1/messages URL).
                if provider_cfg.base_url.is_some() {
                    entry.model.base_url.clone_from(&provider_base);
                }
                if provider_cfg.api.is_some() {
                    entry.model.api.clone_from(&provider_api_string);
                }
                entry.headers.clone_from(&provider_headers);
                if provider_key.is_some() {
                    entry.api_key.clone_from(&provider_key);
                }
                if provider_cfg.compat.is_some() {
                    entry.compat.clone_from(&provider_cfg.compat);
                }
                if provider_cfg.auth_header.is_some() {
                    entry.auth_header = auth_header;
                }
            }
            continue;
        }

        // Remove built-in provider models if fully overridden
        models.retain(|m| m.model.provider != *provider_id);

        for model_cfg in provider_cfg.models.clone().unwrap_or_default() {
            let model_api = model_cfg.api.as_deref().unwrap_or(provider_api);
            let model_api_parsed: Api = model_api
                .parse()
                .unwrap_or_else(|_| Api::Custom(model_api.to_string()));
            let model_headers = merge_headers(
                &provider_headers,
                resolve_headers(model_cfg.headers.as_ref()),
            );
            let input = model_cfg
                .input
                .clone()
                .unwrap_or_else(|| vec!["text".to_string()]);

            let input_types = input
                .iter()
                .filter_map(|i| match i.as_str() {
                    "text" => Some(InputType::Text),
                    "image" => Some(InputType::Image),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let model = Model {
                id: model_cfg.id.clone(),
                name: model_cfg
                    .name
                    .clone()
                    .unwrap_or_else(|| model_cfg.id.clone()),
                api: model_api_parsed.to_string(),
                provider: provider_id.clone(),
                base_url: provider_base.clone(),
                reasoning: model_cfg.reasoning.unwrap_or(false),
                input: if input_types.is_empty() {
                    vec![InputType::Text]
                } else {
                    input_types
                },
                cost: model_cfg.cost.clone().unwrap_or(ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                }),
                context_window: model_cfg.context_window.unwrap_or(128_000),
                max_tokens: model_cfg.max_tokens.unwrap_or(16_384),
                headers: HashMap::new(),
            };

            models.push(ModelEntry {
                model,
                api_key: provider_key.clone(),
                headers: model_headers,
                auth_header,
                compat: model_cfg
                    .compat
                    .clone()
                    .or_else(|| provider_cfg.compat.clone()),
                oauth_config: None,
            });
        }
    }
}

fn merge_headers(
    base: &HashMap<String, String>,
    override_headers: HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = base.clone();
    for (k, v) in override_headers {
        merged.insert(k, v);
    }
    merged
}

fn resolve_headers(headers: Option<&HashMap<String, String>>) -> HashMap<String, String> {
    let mut resolved = HashMap::new();
    if let Some(headers) = headers {
        for (k, v) in headers {
            if let Some(val) = resolve_value(v) {
                resolved.insert(k.clone(), val);
            }
        }
    }
    resolved
}

fn resolve_value(value: &str) -> Option<String> {
    if let Some(rest) = value.strip_prefix('!') {
        return resolve_shell(rest);
    }

    if let Some(var_name) = value.strip_prefix("env:") {
        if var_name.is_empty() {
            return None;
        }
        return std::env::var(var_name).ok().filter(|v| !v.is_empty());
    }

    if let Some(file_path) = value.strip_prefix("file:") {
        if file_path.is_empty() {
            return None;
        }
        return std::fs::read_to_string(file_path)
            .ok()
            .map(|contents| contents.trim().to_string())
            .filter(|v| !v.is_empty());
    }

    if let Ok(env_val) = std::env::var(value) {
        if !env_val.is_empty() {
            return Some(env_val);
        }
    }

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn resolve_shell(cmd: &str) -> Option<String> {
    let output = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", cmd])
            .output()
            .ok()?
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .ok()?
    };

    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

/// Convenience for default models.json path.
pub fn default_models_path(agent_dir: &Path) -> PathBuf {
    agent_dir.join("models.json")
}

// === Ad-hoc model support ===

#[derive(Debug, Clone, Copy)]
struct AdHocProviderDefaults {
    api: &'static str,
    base_url: &'static str,
    reasoning: bool,
    input: &'static [InputType],
    context_window: u32,
    max_tokens: u32,
}

const INPUT_TEXT: [InputType; 1] = [InputType::Text];
const INPUT_TEXT_IMAGE: [InputType; 2] = [InputType::Text, InputType::Image];

#[allow(clippy::too_many_lines)]
fn ad_hoc_provider_defaults(provider: &str) -> Option<AdHocProviderDefaults> {
    match provider {
        // Built-in providers.
        "anthropic" => Some(AdHocProviderDefaults {
            api: "anthropic-messages",
            base_url: "https://api.anthropic.com/v1/messages",
            reasoning: true,
            input: &INPUT_TEXT_IMAGE,
            context_window: 200_000,
            max_tokens: 8192,
        }),
        "openai" => Some(AdHocProviderDefaults {
            api: "openai-responses",
            base_url: "https://api.openai.com/v1",
            reasoning: true,
            input: &INPUT_TEXT_IMAGE,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "google" => Some(AdHocProviderDefaults {
            api: "google-generative-ai",
            base_url: "https://generativelanguage.googleapis.com/v1beta",
            reasoning: true,
            input: &INPUT_TEXT_IMAGE,
            context_window: 128_000,
            max_tokens: 8192,
        }),
        "cohere" => Some(AdHocProviderDefaults {
            api: "cohere-chat",
            base_url: "https://api.cohere.com/v2",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 8192,
        }),

        // OpenAI-compatible providers (chat/completions).
        // Sources: Vercel AI SDK + opencode fixture.
        "groq" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.groq.com/openai/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "deepinfra" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.deepinfra.com/v1/openai",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "cerebras" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.cerebras.ai/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "openrouter" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://openrouter.ai/api/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "mistral" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.mistral.ai/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        // MoonshotAI is the API behind "Kimi".
        "moonshotai" | "moonshot" | "kimi" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.moonshot.ai/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        // Qwen models via DashScope OpenAI-compatible endpoint.
        "alibaba" | "dashscope" | "qwen" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "deepseek" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.deepseek.com",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "fireworks" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.fireworks.ai/inference/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "togetherai" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.together.xyz/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "perplexity" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.perplexity.ai",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        "xai" => Some(AdHocProviderDefaults {
            api: "openai-completions",
            base_url: "https://api.x.ai/v1",
            reasoning: true,
            input: &INPUT_TEXT,
            context_window: 128_000,
            max_tokens: 16_384,
        }),
        _ => None,
    }
}

pub(crate) fn ad_hoc_model_entry(provider: &str, model_id: &str) -> Option<ModelEntry> {
    let defaults = ad_hoc_provider_defaults(provider)?;
    Some(ModelEntry {
        model: Model {
            id: model_id.to_string(),
            name: model_id.to_string(),
            api: defaults.api.to_string(),
            provider: provider.to_string(),
            base_url: defaults.base_url.to_string(),
            reasoning: defaults.reasoning,
            input: defaults.input.to_vec(),
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: defaults.context_window,
            max_tokens: defaults.max_tokens,
            headers: HashMap::new(),
        },
        api_key: None,
        headers: HashMap::new(),
        auth_header: defaults.api.starts_with("openai-"),
        compat: None,
        oauth_config: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthCredential, AuthStorage};
    use tempfile::tempdir;

    fn test_auth_storage() -> (tempfile::TempDir, AuthStorage) {
        let dir = tempdir().expect("tempdir");
        let auth_path = dir.path().join("auth.json");
        let mut auth = AuthStorage::load(auth_path).expect("load auth");
        auth.set(
            "anthropic",
            AuthCredential::ApiKey {
                key: "anthropic-auth-key".to_string(),
            },
        );
        auth.set(
            "openai",
            AuthCredential::ApiKey {
                key: "openai-auth-key".to_string(),
            },
        );
        auth.set(
            "google",
            AuthCredential::ApiKey {
                key: "google-auth-key".to_string(),
            },
        );
        auth.set(
            "acme",
            AuthCredential::ApiKey {
                key: "acme-auth-key".to_string(),
            },
        );
        (dir, auth)
    }

    fn expected_env_pair() -> (String, String) {
        let key = ["PATH", "HOME", "PWD"]
            .iter()
            .find_map(|k| {
                std::env::var(k)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .map(|v| ((*k).to_string(), v))
            })
            .expect("expected at least one non-empty environment variable");
        (key.0, key.1)
    }

    #[test]
    fn built_in_models_include_core_provider_entries() {
        let (_dir, auth) = test_auth_storage();
        let models = built_in_models(&auth);

        assert!(
            models.iter().any(
                |m| m.model.provider == "anthropic" && m.model.id == "claude-sonnet-4-20250514"
            )
        );
        assert!(
            models
                .iter()
                .any(|m| m.model.provider == "openai" && m.model.id == "gpt-4o")
        );
        assert!(
            models
                .iter()
                .any(|m| m.model.provider == "google" && m.model.id == "gemini-2.5-pro")
        );

        let anthropic = models
            .iter()
            .find(|m| m.model.provider == "anthropic")
            .expect("anthropic model");
        let openai = models
            .iter()
            .find(|m| m.model.provider == "openai")
            .expect("openai model");
        let google = models
            .iter()
            .find(|m| m.model.provider == "google")
            .expect("google model");
        assert_eq!(anthropic.api_key.as_deref(), Some("anthropic-auth-key"));
        assert_eq!(openai.api_key.as_deref(), Some("openai-auth-key"));
        assert_eq!(google.api_key.as_deref(), Some("google-auth-key"));
    }

    #[test]
    fn apply_custom_models_overrides_provider_fields() {
        let (_dir, auth) = test_auth_storage();
        let mut models = built_in_models(&auth);
        let (env_key, env_val) = expected_env_pair();
        let mut provider_headers = HashMap::new();
        provider_headers.insert("x-provider".to_string(), "provider-header".to_string());

        let config = ModelsConfig {
            providers: HashMap::from([(
                "anthropic".to_string(),
                ProviderConfig {
                    base_url: Some("https://proxy.example/v1/messages".to_string()),
                    api: Some("anthropic-messages".to_string()),
                    api_key: Some(format!("env:{env_key}")),
                    headers: Some(provider_headers),
                    auth_header: Some(true),
                    compat: Some(CompatConfig {
                        supports_store: Some(true),
                        ..CompatConfig::default()
                    }),
                    models: None,
                },
            )]),
        };

        apply_custom_models(&auth, &mut models, &config);

        for entry in models.iter().filter(|m| m.model.provider == "anthropic") {
            assert_eq!(entry.model.base_url, "https://proxy.example/v1/messages");
            assert_eq!(entry.model.api, "anthropic-messages");
            assert_eq!(entry.api_key.as_deref(), Some(env_val.as_str()));
            assert_eq!(
                entry.headers.get("x-provider").map(String::as_str),
                Some("provider-header")
            );
            assert!(entry.auth_header);
            assert_eq!(
                entry
                    .compat
                    .as_ref()
                    .and_then(|c| c.supports_store)
                    .unwrap_or(false),
                true
            );
        }
    }

    #[test]
    fn model_registry_find_and_find_by_id_work() {
        let (_dir, auth) = test_auth_storage();
        let registry = ModelRegistry::load(&auth, None);

        let by_provider_and_id = registry
            .find("openai", "gpt-4o")
            .expect("openai/gpt-4o should exist");
        assert_eq!(by_provider_and_id.model.provider, "openai");
        assert_eq!(by_provider_and_id.model.id, "gpt-4o");

        let by_id = registry
            .find_by_id("gemini-2.5-pro")
            .expect("gemini-2.5-pro should exist");
        assert_eq!(by_id.model.provider, "google");
        assert_eq!(by_id.model.id, "gemini-2.5-pro");

        assert!(registry.find("openai", "does-not-exist").is_none());
        assert!(registry.find_by_id("does-not-exist").is_none());
    }

    #[test]
    fn model_registry_merge_entries_deduplicates() {
        let (_dir, auth) = test_auth_storage();
        let mut registry = ModelRegistry::load(&auth, None);
        let before = registry.models().len();
        let duplicate = registry
            .find("openai", "gpt-4o")
            .expect("expected built-in openai model");

        let new_entry = ModelEntry {
            model: Model {
                id: "acme-chat".to_string(),
                name: "Acme Chat".to_string(),
                api: "openai-completions".to_string(),
                provider: "acme".to_string(),
                base_url: "https://acme.example/v1".to_string(),
                reasoning: true,
                input: vec![InputType::Text],
                cost: ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 64_000,
                max_tokens: 4096,
                headers: HashMap::new(),
            },
            api_key: Some("acme-auth-key".to_string()),
            headers: HashMap::new(),
            auth_header: true,
            compat: None,
            oauth_config: None,
        };

        registry.merge_entries(vec![duplicate, new_entry]);
        assert_eq!(registry.models().len(), before + 1);
        assert!(registry.find("acme", "acme-chat").is_some());
    }

    #[test]
    fn resolve_value_supports_env_and_file_prefixes() {
        let (env_key, env_val) = expected_env_pair();
        assert_eq!(
            resolve_value(&format!("env:{env_key}")).as_deref(),
            Some(env_val.as_str())
        );
        assert_eq!(resolve_value(&env_key).as_deref(), Some(env_val.as_str()));

        let dir = tempdir().expect("tempdir");
        let key_path = dir.path().join("api_key.txt");
        std::fs::write(&key_path, "file-key\n").expect("write key file");
        assert_eq!(
            resolve_value(&format!("file:{}", key_path.display())).as_deref(),
            Some("file-key")
        );
        assert!(resolve_value("file:/definitely/missing/path").is_none());
    }

    #[test]
    fn model_registry_load_reads_models_json_and_applies_config() {
        let (dir, auth) = test_auth_storage();
        let models_path = dir.path().join("models.json");
        let key_path = dir.path().join("custom_key.txt");
        std::fs::write(&key_path, "acme-file-key\n").expect("write custom key");

        let models_json = serde_json::json!({
            "providers": {
                "acme": {
                    "baseUrl": "https://acme.example/v1",
                    "api": "openai-completions",
                    "apiKey": format!("file:{}", key_path.display()),
                    "headers": {
                        "x-provider": "provider-level"
                    },
                    "authHeader": true,
                    "models": [
                        {
                            "id": "acme-chat",
                            "name": "Acme Chat",
                            "input": ["text", "image"],
                            "reasoning": true,
                            "contextWindow": 64000,
                            "maxTokens": 4096,
                            "headers": {
                                "x-model": "model-level"
                            }
                        }
                    ]
                }
            }
        });

        std::fs::write(
            &models_path,
            serde_json::to_string_pretty(&models_json).expect("serialize models json"),
        )
        .expect("write models.json");

        let registry = ModelRegistry::load(&auth, Some(models_path));
        let acme = registry
            .find("acme", "acme-chat")
            .expect("custom acme model should load from models.json");

        assert_eq!(acme.model.name, "Acme Chat");
        assert_eq!(acme.model.api, "openai-completions");
        assert_eq!(acme.model.base_url, "https://acme.example/v1");
        assert_eq!(acme.model.context_window, 64_000);
        assert_eq!(acme.model.max_tokens, 4096);
        assert_eq!(acme.api_key.as_deref(), Some("acme-file-key"));
        assert!(acme.auth_header);
        assert_eq!(
            acme.headers.get("x-provider").map(String::as_str),
            Some("provider-level")
        );
        assert_eq!(
            acme.headers.get("x-model").map(String::as_str),
            Some("model-level")
        );
        assert_eq!(acme.model.input, vec![InputType::Text, InputType::Image]);
    }
}

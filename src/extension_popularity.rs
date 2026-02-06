//! Popularity signal snapshotting for extension candidates.
//!
//! This module is intentionally "evidence-first":
//! - Fetch concrete metrics (GitHub stars/downloads/etc).
//! - Normalize missing/unavailable metrics to `null` (never `0`).
//! - Persist evidence onto the canonical candidate pool JSON so scoring can be auditable.

use crate::error::{Error, Result};
use crate::http::client::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidatePool {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub generated_at: String,
    pub source_inputs: SourceInputs,
    pub total_candidates: u64,
    pub items: Vec<CandidateItem>,
    pub alias_notes: Vec<AliasNote>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceInputs {
    pub artifact_provenance: String,
    pub artifact_root: String,
    pub extra_npm_packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AliasNote {
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidateItem {
    pub id: String,
    pub name: String,
    pub source_tier: String,
    pub status: String,
    pub license: String,
    pub retrieved: String,
    pub artifact_path: String,
    pub checksum: Sha256Checksum,
    pub source: CandidateSource,
    pub repository_url: String,
    #[serde(default)]
    pub popularity: PopularityEvidence,
    pub aliases: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sha256Checksum {
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CandidateSource {
    Git {
        repo: String,
        #[serde(default)]
        path: Option<String>,
    },
    Npm {
        package: String,
        version: String,
        url: String,
    },
    Url {
        url: String,
    },
}

/// Popularity evidence schema.
///
/// This is the machine-joinable surface used by scoring (see `docs/EXTENSION_POPULARITY_CRITERIA.md`).
/// When a metric is unknown/unavailable, it should be persisted as explicit `null`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PopularityEvidence {
    pub snapshot_at: Option<String>,

    // GitHub
    pub github_repo: Option<String>,
    pub github_stars: Option<u64>,
    pub github_forks: Option<u64>,
    pub github_watchers: Option<u64>,
    pub github_open_issues: Option<u64>,
    pub github_last_commit: Option<String>,

    // npm
    pub npm_downloads_weekly: Option<u64>,
    pub npm_downloads_monthly: Option<u64>,
    pub npm_last_publish: Option<String>,
    pub npm_dependents: Option<u64>,

    // Marketplace (OpenClaw / ClawHub) - not currently populated by the candidate pool.
    pub marketplace_rank: Option<u32>,
    pub marketplace_installs_monthly: Option<u64>,
    pub marketplace_featured: Option<bool>,

    // Mentions / references - not currently populated by the candidate pool.
    pub mentions_count: Option<u32>,
    pub mentions_sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GitHubRepoRef {
    pub owner: String,
    pub repo: String,
}

impl GitHubRepoRef {
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GitHubRepoMetrics {
    pub full_name: String,
    pub stars: u64,
    pub forks: u64,
    pub watchers: Option<u64>,
    pub open_issues: u64,
    pub pushed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpmDownloads {
    pub weekly: Option<u64>,
    pub monthly: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpmRegistryMeta {
    pub latest_version: Option<String>,
    pub last_publish: Option<String>,
    pub repository_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitHubRepoCandidate {
    Repo(GitHubRepoRef),
    /// A malformed GitHub URL that only included a single path segment, e.g. `https://github.com/foo-bar`.
    Slug(String),
}

/// Best-effort parse of a GitHub repository reference from a URL-like string.
///
/// Supports:
/// - `https://github.com/owner/repo`
/// - `git+https://github.com/owner/repo.git`
/// - `git@github.com:owner/repo.git`
/// - `github.com/owner/repo`
///
/// For malformed single-segment URLs (e.g. `https://github.com/foo-bar`) returns a `Slug` candidate.
#[must_use]
pub fn github_repo_candidate_from_url(input: &str) -> Option<GitHubRepoCandidate> {
    let raw = input.trim();
    if raw.is_empty() {
        return None;
    }

    let raw = raw.strip_prefix("git+").unwrap_or(raw);

    if let Some(rest) = raw.strip_prefix("git@") {
        // SCP-like: git@github.com:owner/repo(.git)
        let (_host, path) = rest.split_once(':')?;
        return parse_owner_repo_from_path(path).map(GitHubRepoCandidate::Repo);
    }

    let url_str = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };

    let Ok(url) = url::Url::parse(&url_str) else {
        return None;
    };
    if url.host_str()? != "github.com" {
        return None;
    }

    let mut segments = url.path_segments()?.filter(|seg| !seg.is_empty());
    let ownerish = segments.next()?.to_string();
    let repo = segments.next().map(|s| s.to_string());

    match repo {
        Some(repo) => parse_owner_repo(ownerish, repo).map(GitHubRepoCandidate::Repo),
        None => Some(GitHubRepoCandidate::Slug(ownerish)),
    }
}

#[must_use]
pub fn github_repo_guesses_from_slug(slug: &str) -> Vec<GitHubRepoRef> {
    let slug = slug.trim().trim_matches('/');
    if slug.is_empty() {
        return Vec::new();
    }

    let mut seen = HashSet::<GitHubRepoRef>::new();
    let mut out = Vec::new();

    // Common case for our third-party imports: `owner-pi-foo` should be `owner/pi-foo`.
    if let Some((owner, suffix)) = slug.split_once("-pi-") {
        let owner = owner.to_string();
        let repo = format!("pi-{suffix}");
        if let Some(r) = parse_owner_repo(owner, repo) {
            if seen.insert(r.clone()) {
                out.push(r);
            }
        }
    }

    // Try first hyphen split: `owner-rest...` -> `owner/rest...`
    if let Some((owner, repo)) = slug.split_once('-') {
        if let Some(r) = parse_owner_repo(owner.to_string(), repo.to_string()) {
            if seen.insert(r.clone()) {
                out.push(r);
            }
        }
    }

    // Try last hyphen split: `owner...-repo` -> `owner.../repo`
    if let Some((owner, repo)) = slug.rsplit_once('-') {
        if let Some(r) = parse_owner_repo(owner.to_string(), repo.to_string()) {
            if seen.insert(r.clone()) {
                out.push(r);
            }
        }
    }

    out
}

pub fn parse_github_repo_response(text: &str) -> Result<GitHubRepoMetrics> {
    #[derive(Debug, Deserialize)]
    struct RepoResponse {
        full_name: String,
        stargazers_count: u64,
        forks_count: u64,
        #[serde(default)]
        subscribers_count: Option<u64>,
        open_issues_count: u64,
        #[serde(default)]
        pushed_at: Option<String>,
    }

    let parsed: RepoResponse = serde_json::from_str(text)
        .map_err(|err| Error::api(format!("GitHub repo response parse error: {err}")))?;

    Ok(GitHubRepoMetrics {
        full_name: parsed.full_name,
        stars: parsed.stargazers_count,
        forks: parsed.forks_count,
        watchers: parsed.subscribers_count,
        open_issues: parsed.open_issues_count,
        pushed_at: parsed.pushed_at,
    })
}

pub async fn fetch_github_repo_metrics_optional(
    client: &Client,
    token: &str,
    repo: &GitHubRepoRef,
) -> Result<Option<GitHubRepoMetrics>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}",
        repo.owner, repo.repo
    );
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    match status {
        200 => Ok(Some(parse_github_repo_response(&text)?)),
        404 => Ok(None),
        other => Err(Error::api(format!("GitHub API error {other}: {text}"))),
    }
}

pub fn parse_npm_downloads_response(text: &str) -> Result<Option<u64>> {
    #[derive(Debug, Deserialize)]
    struct DownloadsResponse {
        #[serde(default)]
        downloads: Option<u64>,
        #[serde(default)]
        error: Option<String>,
    }

    let parsed: DownloadsResponse = serde_json::from_str(text)
        .map_err(|err| Error::api(format!("npm downloads response parse error: {err}")))?;

    if parsed.error.is_some() {
        return Ok(None);
    }

    Ok(parsed.downloads)
}

pub async fn fetch_npm_downloads(client: &Client, package: &str) -> Result<NpmDownloads> {
    async fn fetch_range(client: &Client, package: &str, range: &str) -> Result<Option<u64>> {
        let encoded = url::form_urlencoded::byte_serialize(package.as_bytes()).collect::<String>();
        let url = format!("https://api.npmjs.org/downloads/point/{range}/{encoded}");
        let response = client.get(&url).send().await?;
        let text = response.text().await?;
        parse_npm_downloads_response(&text)
    }

    let weekly = fetch_range(client, package, "last-week").await?;
    let monthly = fetch_range(client, package, "last-month").await?;

    Ok(NpmDownloads { weekly, monthly })
}

pub fn parse_npm_registry_response(text: &str) -> Result<NpmRegistryMeta> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|err| Error::api(format!("npm registry response parse error: {err}")))?;

    let latest_version = value
        .get("dist-tags")
        .and_then(|tags| tags.get("latest"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let last_publish = latest_version
        .as_deref()
        .and_then(|latest| value.get("time").and_then(|t| t.get(latest)))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let repository_url = match value.get("repository") {
        Some(serde_json::Value::String(url)) => Some(url.clone()),
        Some(serde_json::Value::Object(obj)) => obj
            .get("url")
            .and_then(|url| url.as_str())
            .map(ToString::to_string),
        _ => None,
    };

    Ok(NpmRegistryMeta {
        latest_version,
        last_publish,
        repository_url,
    })
}

pub async fn fetch_npm_registry_meta(client: &Client, package: &str) -> Result<Option<NpmRegistryMeta>> {
    let encoded = url::form_urlencoded::byte_serialize(package.as_bytes()).collect::<String>();
    let url = format!("https://registry.npmjs.org/{encoded}");
    let response = client.get(&url).send().await?;
    let status = response.status();
    let text = response.text().await?;

    match status {
        200 => Ok(Some(parse_npm_registry_response(&text)?)),
        404 => Ok(None),
        other => Err(Error::api(format!("npm registry error {other}: {text}"))),
    }
}

fn parse_owner_repo(owner: String, repo: String) -> Option<GitHubRepoRef> {
    let owner = owner.trim().trim_matches('/').to_string();
    let repo = repo.trim().trim_matches('/').trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(GitHubRepoRef { owner, repo })
}

fn parse_owner_repo_from_path(path: &str) -> Option<GitHubRepoRef> {
    let path = path.trim().trim_matches('/');
    let mut parts = path.split('/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    parse_owner_repo(owner, repo)
}

/// Fetch all referenced GitHub repos (deduped) and return a `full_name -> metrics` map.
pub async fn snapshot_github_repos(
    client: &Client,
    token: &str,
    repos: &[GitHubRepoRef],
) -> Result<HashMap<String, GitHubRepoMetrics>> {
    let mut out = HashMap::new();
    for repo in repos {
        if let Some(metrics) = fetch_github_repo_metrics_optional(client, token, repo).await? {
            out.insert(repo.full_name(), metrics);
        }
    }
    Ok(out)
}

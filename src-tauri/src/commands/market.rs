use crate::types::{
    DownloadRequest, DownloadResult, MarketStatus, MarketStatusType, RemoteSkill, RemoteSkillView,
    RemoteSkillsResponse, RemoteSkillsViewResponse,
};
use crate::utils::download::{download_market_bytes, download_skill_to_dir};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const USER_AGENT: &str = "skills-manager-gui/0.1";
const SKILLS_HUB_INDEX_URL: &str =
    "https://raw.githubusercontent.com/qufei1993/skills-hub/main/featured-skills.json";
const SKILLS_HUB_CDN_URL: &str =
    "https://cdn.jsdelivr.net/gh/qufei1993/skills-hub@main/featured-skills.json";
const MARKET_FAILURE_COOLDOWN: Duration = Duration::from_secs(90);

static MARKET_FAILURES: OnceLock<Mutex<HashMap<&'static str, Instant>>> = OnceLock::new();
const MARKET_SKILL_METADATA: &str = ".skills-manager.json";

#[derive(Serialize)]
struct InstalledSkillMetadata<'a> {
    source_url: &'a str,
}

#[derive(Deserialize, Debug)]
struct SkillsHubResponse {
    #[allow(dead_code)]
    updated_at: Option<String>,
    #[allow(dead_code)]
    total: Option<u64>,
    skills: Vec<SkillsHubSkill>,
}

#[derive(Deserialize, Debug)]
struct SkillsHubSkill {
    slug: String,
    name: String,
    summary: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    stars: u64,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    source_url: String,
}

fn map_claude_skill(skill: RemoteSkill, market_id: &str, market_label: &str) -> RemoteSkillView {
    RemoteSkillView {
        id: format!("{}:{}", market_id, skill.id),
        name: skill.name,
        namespace: skill.namespace,
        source_url: skill.source_url,
        description: skill.description,
        author: skill.author,
        installs: skill.installs,
        stars: skill.stars,
        market_id: market_id.to_string(),
        market_label: market_label.to_string(),
    }
}

fn build_github_source_url(owner: &str, repo: &str) -> String {
    format!("https://github.com/{owner}/{repo}")
}

fn get_value_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(s) = found.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn get_value_u64(value: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(n) = found.as_u64() {
                return Some(n);
            }
            if let Some(n) = found.as_i64() {
                if n >= 0 {
                    return Some(n as u64);
                }
            }
        }
    }
    None
}

fn extract_github_owner(source_url: &str) -> String {
    source_url
        .strip_prefix("https://github.com/")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default()
        .to_string()
}

fn matches_skills_hub_query(skill: &SkillsHubSkill, query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }

    let keyword = trimmed.to_ascii_lowercase();
    let tags_text = skill.tags.join(" ");
    [
        skill.name.as_str(),
        skill.slug.as_str(),
        skill.summary.as_str(),
        skill.category.as_str(),
        tags_text.as_str(),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(&keyword))
}

fn parse_skillsllm(
    buf: &[u8],
    market_id: &str,
    market_label: &str,
) -> Result<(Vec<RemoteSkillView>, u64), String> {
    let value: serde_json::Value = serde_json::from_slice(buf).map_err(|err| err.to_string())?;

    let list = value.get("skills").and_then(|v| v.as_array());

    let mut skills = Vec::new();
    if let Some(items) = list {
        for item in items {
            let github_owner =
                get_value_string(item, &["githubOwner", "github_owner", "owner", "repoOwner"]);
            let github_repo =
                get_value_string(item, &["githubRepo", "github_repo", "repo", "repoName"]);
            let source_url =
                get_value_string(item, &["githubUrl", "sourceUrl", "source_url", "repoUrl"])
                    .or_else(|| match (github_owner.as_deref(), github_repo.as_deref()) {
                        (Some(o), Some(r)) => Some(build_github_source_url(o, r)),
                        _ => None,
                    })
                    .unwrap_or_default();

            let name = get_value_string(item, &["name", "title"])
                .or_else(|| github_repo.clone())
                .unwrap_or_else(|| "skill".to_string());
            let description =
                get_value_string(item, &["description", "summary"]).unwrap_or_default();
            let author = get_value_string(item, &["githubOwner", "github_owner", "author"])
                .unwrap_or_default();
            let namespace = get_value_string(item, &["namespace"])
                .or_else(|| github_owner.clone())
                .unwrap_or_default();
            let stars = get_value_u64(item, &["stars", "githubStars", "github_stars"]).unwrap_or(0);
            let installs = get_value_u64(item, &["installs", "downloads"]).unwrap_or(0);
            let raw_id = get_value_string(item, &["id", "slug"])
                .or_else(|| match (github_owner.as_deref(), github_repo.as_deref()) {
                    (Some(o), Some(r)) => Some(format!("{o}/{r}")),
                    _ => None,
                })
                .unwrap_or_else(|| name.clone());

            skills.push(RemoteSkillView {
                id: format!("{}:{}", market_id, raw_id),
                name,
                namespace,
                source_url,
                description,
                author,
                installs,
                stars,
                market_id: market_id.to_string(),
                market_label: market_label.to_string(),
            });
        }
    }

    let total = value
        .get("pagination")
        .and_then(|p| get_value_u64(p, &["total", "count"]))
        .unwrap_or(skills.len() as u64);

    Ok((skills, total))
}

fn parse_skillsmp(
    buf: &[u8],
    market_id: &str,
    market_label: &str,
) -> Result<(Vec<RemoteSkillView>, u64), String> {
    let value: serde_json::Value = serde_json::from_slice(buf).map_err(|err| err.to_string())?;

    let list = value
        .get("data")
        .and_then(|d| d.get("skills"))
        .and_then(|v| v.as_array());

    let mut skills = Vec::new();
    if let Some(items) = list {
        for item in items {
            let source_url = get_value_string(item, &["githubUrl", "sourceUrl", "source_url"])
                .unwrap_or_default();
            let author = get_value_string(item, &["author"]).unwrap_or_default();
            let namespace = get_value_string(item, &["namespace"]).unwrap_or_default();
            let name = get_value_string(item, &["name", "title", "slug"])
                .unwrap_or_else(|| "skill".to_string());
            let description =
                get_value_string(item, &["description", "summary"]).unwrap_or_default();
            let installs = get_value_u64(item, &["downloads", "installs"]).unwrap_or(0);
            let stars = get_value_u64(item, &["stars", "githubStars", "github_stars"]).unwrap_or(0);
            let raw_id = get_value_string(item, &["id", "slug"]).unwrap_or_else(|| name.clone());

            skills.push(RemoteSkillView {
                id: format!("{}:{}", market_id, raw_id),
                name,
                namespace,
                source_url,
                description,
                author,
                installs,
                stars,
                market_id: market_id.to_string(),
                market_label: market_label.to_string(),
            });
        }
    }

    let total = value
        .get("data")
        .and_then(|d| d.get("pagination"))
        .and_then(|p| get_value_u64(p, &["total", "count"]))
        .unwrap_or(skills.len() as u64);

    Ok((skills, total))
}

fn parse_skills_hub(
    buf: &[u8],
    market_id: &str,
    market_label: &str,
    query: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<RemoteSkillView>, u64), String> {
    let response: SkillsHubResponse = serde_json::from_slice(buf).map_err(|err| err.to_string())?;
    let filtered: Vec<SkillsHubSkill> = response
        .skills
        .into_iter()
        .filter(|skill| matches_skills_hub_query(skill, query))
        .collect();
    let total = filtered.len() as u64;

    let skills = filtered
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|skill| RemoteSkillView {
            id: format!("{}:{}", market_id, skill.slug),
            name: skill.name,
            namespace: skill.category,
            source_url: skill.source_url.clone(),
            description: skill.summary,
            author: extract_github_owner(&skill.source_url),
            installs: skill.downloads,
            stars: skill.stars,
            market_id: market_id.to_string(),
            market_label: market_label.to_string(),
        })
        .collect();

    Ok((skills, total))
}

fn write_installed_skill_metadata(installed_dir: &std::path::Path, source_url: &str) -> Result<(), String> {
    let metadata = InstalledSkillMetadata { source_url };
    let raw = serde_json::to_string_pretty(&metadata).map_err(|err| err.to_string())?;
    fs::write(installed_dir.join(MARKET_SKILL_METADATA), raw).map_err(|err| err.to_string())
}

struct MarketFetchResult {
    skills: Vec<RemoteSkillView>,
    total: u64,
    status: MarketStatus,
}

fn build_status(id: &str, name: &str, status: MarketStatusType, error: Option<String>) -> MarketStatus {
    MarketStatus {
        id: id.to_string(),
        name: name.to_string(),
        status,
        error,
    }
}

fn market_failures() -> &'static Mutex<HashMap<&'static str, Instant>> {
    MARKET_FAILURES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn recently_failed(market_id: &'static str) -> bool {
    let Ok(guard) = market_failures().lock() else {
        return false;
    };

    guard
        .get(market_id)
        .is_some_and(|instant| instant.elapsed() < MARKET_FAILURE_COOLDOWN)
}

fn mark_market_failure(market_id: &'static str) {
    if let Ok(mut guard) = market_failures().lock() {
        guard.insert(market_id, Instant::now());
    }
}

fn clear_market_failure(market_id: &'static str) {
    if let Ok(mut guard) = market_failures().lock() {
        guard.remove(market_id);
    }
}

fn cooldown_result(id: &str, name: &str) -> MarketFetchResult {
    error_result(
        id,
        name,
        format!(
            "Temporarily skipped after a recent network failure. Retrying in {} seconds.",
            MARKET_FAILURE_COOLDOWN.as_secs()
        ),
    )
}

fn online_result(id: &str, name: &str) -> MarketFetchResult {
    MarketFetchResult {
        skills: Vec::new(),
        total: 0,
        status: build_status(id, name, MarketStatusType::Online, None),
    }
}

fn needs_key_result(id: &str, name: &str) -> MarketFetchResult {
    MarketFetchResult {
        skills: Vec::new(),
        total: 0,
        status: build_status(id, name, MarketStatusType::NeedsKey, None),
    }
}

fn error_result(id: &str, name: &str, error: impl Into<String>) -> MarketFetchResult {
    MarketFetchResult {
        skills: Vec::new(),
        total: 0,
        status: build_status(id, name, MarketStatusType::Error, Some(error.into())),
    }
}

fn fetch_claude_plugins(trimmed: &str, limit: u64, offset: u64) -> MarketFetchResult {
    let market_id = "claude-plugins";
    let market_label = "Claude Plugins";
    if recently_failed(market_id) {
        return cooldown_result(market_id, market_label);
    }

    let query_param = if trimmed.is_empty() {
        String::new()
    } else {
        format!("q={}", urlencoding::encode(trimmed))
    };

    let mut url = String::from("https://claude-plugins.dev/api/skills?");
    if !query_param.is_empty() {
        url.push_str(&query_param);
        url.push('&');
    }
    url.push_str(&format!("limit={limit}&offset={offset}"));

    match download_market_bytes(&url, &[("Accept", "application/json"), ("User-Agent", USER_AGENT)]) {
        Ok(buf) => match serde_json::from_slice::<RemoteSkillsResponse>(&buf) {
            Ok(parsed) => {
                clear_market_failure(market_id);
                MarketFetchResult {
                    total: parsed.total,
                    skills: parsed
                        .skills
                        .into_iter()
                        .map(|skill| map_claude_skill(skill, market_id, market_label))
                        .collect(),
                    status: build_status(market_id, market_label, MarketStatusType::Online, None),
                }
            }
            Err(_) => {
                mark_market_failure(market_id);
                error_result(market_id, market_label, "Failed to parse response")
            }
        },
        Err(err) => {
            println!("Error fetching from Claude Plugins: {err}");
            mark_market_failure(market_id);
            error_result(market_id, market_label, err)
        }
    }
}

fn fetch_skillsllm(trimmed: &str, limit: u64, offset: u64) -> MarketFetchResult {
    let market_id = "skillsllm";
    let market_label = "SkillsLLM";
    if recently_failed(market_id) {
        return cooldown_result(market_id, market_label);
    }

    let mut url = String::from("https://api.skills-llm.com/skill?sort=stars");
    if !trimmed.is_empty() {
        url.push_str("&q=");
        url.push_str(&urlencoding::encode(trimmed));
    }
    url.push_str(&format!("&limit={limit}&offset={offset}"));

    match download_market_bytes(
        &url,
        &[
            ("Accept", "application/json"),
            ("X-GitHub-Api-Version", "2022-11-28"),
            ("User-Agent", USER_AGENT),
        ],
    ) {
        Ok(buf) => match parse_skillsllm(&buf, market_id, market_label) {
            Ok((skills, total)) => {
                clear_market_failure(market_id);
                MarketFetchResult {
                    skills,
                    total,
                    status: build_status(market_id, market_label, MarketStatusType::Online, None),
                }
            }
            Err(_) => {
                mark_market_failure(market_id);
                error_result(market_id, market_label, "Failed to parse response")
            }
        },
        Err(err) => {
            println!("Error fetching from SkillsLLM: {err}");
            mark_market_failure(market_id);
            error_result(market_id, market_label, err)
        }
    }
}

fn fetch_skills_hub(trimmed: &str, limit: u64, offset: u64) -> MarketFetchResult {
    let market_id = "skills-hub";
    let market_label = "Skills Hub";
    if recently_failed(market_id) {
        return cooldown_result(market_id, market_label);
    }

    let headers = [("Accept", "application/json"), ("User-Agent", USER_AGENT)];
    let buf = match download_market_bytes(SKILLS_HUB_INDEX_URL, &headers) {
        Ok(buf) => Ok(buf),
        Err(primary_err) => {
            println!("Error fetching from Skills Hub primary URL: {primary_err}");
            download_market_bytes(SKILLS_HUB_CDN_URL, &headers).map_err(|fallback_err| {
                format!("Primary failed: {primary_err}; CDN failed: {fallback_err}")
            })
        }
    };

    match buf {
        Ok(buf) => match parse_skills_hub(&buf, market_id, market_label, trimmed, limit, offset) {
            Ok((skills, total)) => {
                clear_market_failure(market_id);
                MarketFetchResult {
                    skills,
                    total,
                    status: build_status(market_id, market_label, MarketStatusType::Online, None),
                }
            }
            Err(err) => {
                mark_market_failure(market_id);
                error_result(market_id, market_label, err)
            }
        },
        Err(err) => {
            println!("Error fetching from Skills Hub: {err}");
            mark_market_failure(market_id);
            error_result(market_id, market_label, err)
        }
    }
}

fn fetch_skillsmp(trimmed: &str, limit: u64, offset: u64, api_key: Option<String>) -> MarketFetchResult {
    let market_id = "skillsmp";
    let market_label = "SkillsMP";

    let Some(api_key) = api_key.filter(|key| !key.is_empty()) else {
        return needs_key_result(market_id, market_label);
    };

    if recently_failed(market_id) {
        return cooldown_result(market_id, market_label);
    }

    if trimmed.is_empty() {
        return online_result(market_id, market_label);
    }

    let page = (offset / limit).saturating_add(1);
    let url = format!(
        "https://skillsmp.com/api/v1/skills/search?q={}&page={page}&limit={limit}",
        urlencoding::encode(trimmed)
    );
    let auth_header = format!("Bearer {api_key}");

    match download_market_bytes(
        &url,
        &[
            ("Accept", "application/json"),
            ("User-Agent", USER_AGENT),
            ("Authorization", &auth_header),
        ],
    ) {
        Ok(buf) => match parse_skillsmp(&buf, market_id, market_label) {
            Ok((skills, total)) => {
                clear_market_failure(market_id);
                MarketFetchResult {
                    skills,
                    total,
                    status: build_status(market_id, market_label, MarketStatusType::Online, None),
                }
            }
            Err(_) => {
                mark_market_failure(market_id);
                error_result(market_id, market_label, "Failed to parse response")
            }
        },
        Err(err) => {
            println!("Error fetching from SkillsMP: {err}");
            mark_market_failure(market_id);
            error_result(market_id, market_label, err)
        }
    }
}

#[tauri::command]
pub async fn search_marketplaces(
    query: String,
    limit: u64,
    offset: u64,
    api_keys: HashMap<String, String>,
    enabled_markets: HashMap<String, bool>,
) -> Result<RemoteSkillsViewResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut skills: Vec<RemoteSkillView> = Vec::new();
        let mut total: u64 = 0;
        let mut market_statuses: Vec<MarketStatus> = Vec::new();

        let trimmed = query.trim();
        let limit = if limit == 0 { 20 } else { limit };
        let trimmed = trimmed.to_string();

        let claude_enabled = *enabled_markets.get("claude-plugins").unwrap_or(&true);
        let skillsllm_enabled = *enabled_markets.get("skillsllm").unwrap_or(&true);
        let skills_hub_enabled = *enabled_markets.get("skills-hub").unwrap_or(&true);
        let skillsmp_enabled = *enabled_markets.get("skillsmp").unwrap_or(&false);
        let skillsmp_api_key = api_keys.get("skillsmp").cloned();

        let claude_handle = claude_enabled.then(|| {
            let trimmed = trimmed.clone();
            thread::spawn(move || fetch_claude_plugins(&trimmed, limit, offset))
        });
        let skillsllm_handle = skillsllm_enabled.then(|| {
            let trimmed = trimmed.clone();
            thread::spawn(move || fetch_skillsllm(&trimmed, limit, offset))
        });
        let skills_hub_handle = skills_hub_enabled.then(|| {
            let trimmed = trimmed.clone();
            thread::spawn(move || fetch_skills_hub(&trimmed, limit, offset))
        });
        let skillsmp_handle = skillsmp_enabled.then(|| {
            let trimmed = trimmed.clone();
            let api_key = skillsmp_api_key.clone();
            thread::spawn(move || fetch_skillsmp(&trimmed, limit, offset, api_key))
        });

        let mut merge_result = |result: MarketFetchResult| {
            total += result.total;
            skills.extend(result.skills);
            market_statuses.push(result.status);
        };

        if let Some(handle) = claude_handle {
            match handle.join() {
                Ok(result) => merge_result(result),
                Err(_) => merge_result(error_result(
                    "claude-plugins",
                    "Claude Plugins",
                    "Marketplace worker panicked",
                )),
            }
        } else {
            merge_result(online_result("claude-plugins", "Claude Plugins"));
        }

        if let Some(handle) = skillsllm_handle {
            match handle.join() {
                Ok(result) => merge_result(result),
                Err(_) => merge_result(error_result("skillsllm", "SkillsLLM", "Marketplace worker panicked")),
            }
        } else {
            merge_result(online_result("skillsllm", "SkillsLLM"));
        }

        if let Some(handle) = skills_hub_handle {
            match handle.join() {
                Ok(result) => merge_result(result),
                Err(_) => merge_result(error_result("skills-hub", "Skills Hub", "Marketplace worker panicked")),
            }
        } else {
            merge_result(online_result("skills-hub", "Skills Hub"));
        }

        if let Some(handle) = skillsmp_handle {
            match handle.join() {
                Ok(result) => merge_result(result),
                Err(_) => merge_result(error_result("skillsmp", "SkillsMP", "Marketplace worker panicked")),
            }
        } else {
            merge_result(needs_key_result("skillsmp", "SkillsMP"));
        }

        Ok(RemoteSkillsViewResponse {
            skills,
            total,
            limit,
            offset,
            market_statuses,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_marketplace_skill(
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    if request.install_base_dir.trim().is_empty() {
        return Err("安装目录不能为空".to_string());
    }

    let source_url = request.source_url.clone();
    let skill_name = request.skill_name.clone();
    let install_base_dir = PathBuf::from(&request.install_base_dir);

    let result = tauri::async_runtime::spawn_blocking(move || {
        let installed_dir = download_skill_to_dir(&source_url, &skill_name, &install_base_dir, false)?;
        write_installed_skill_metadata(&installed_dir, &source_url)?;
        Ok::<PathBuf, String>(installed_dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(DownloadResult {
        installed_path: result.display().to_string(),
    })
}

#[tauri::command]
pub async fn update_marketplace_skill(request: DownloadRequest) -> Result<DownloadResult, String> {
    if request.install_base_dir.trim().is_empty() {
        return Err("安装目录不能为空".to_string());
    }
    if request.source_url.trim().is_empty() {
        return Err("缺少有效的源码地址 (Source URL)，无法更新".to_string());
    }

    let source_url = request.source_url.clone();
    let skill_name = request.skill_name.clone();
    let install_base_dir = PathBuf::from(&request.install_base_dir);

    let result = tauri::async_runtime::spawn_blocking(move || {
        let installed_dir = download_skill_to_dir(&source_url, &skill_name, &install_base_dir, true)?;
        write_installed_skill_metadata(&installed_dir, &source_url)?;
        Ok::<PathBuf, String>(installed_dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(DownloadResult {
        installed_path: result.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_skills_hub;

    #[test]
    fn filters_and_maps_skills_hub_results() {
        let raw = br#"
        {
          "updated_at": "2026-03-27T01:05:42.621Z",
          "total": 2,
          "skills": [
            {
              "slug": "docx",
              "name": "docx",
              "summary": "Manipulate Word documents",
              "downloads": 0,
              "stars": 10,
              "category": "ai-assistant",
              "tags": ["agent-skills", "documents"],
              "source_url": "https://github.com/anthropics/skills/tree/main/skills/docx"
            },
            {
              "slug": "bug-hunter",
              "name": "bug-hunter",
              "summary": "Debug production issues",
              "downloads": 3,
              "stars": 20,
              "category": "development",
              "tags": ["debugging"],
              "source_url": "https://github.com/acme/skills/tree/main/skills/bug-hunter"
            }
          ]
        }
        "#;

        let (skills, total) = parse_skills_hub(raw, "skills-hub", "Skills Hub", "doc", 20, 0).unwrap();
        assert_eq!(total, 1);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "skills-hub:docx");
        assert_eq!(skills[0].author, "anthropics");
        assert_eq!(skills[0].market_label, "Skills Hub");
    }
}

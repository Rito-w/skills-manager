use crate::types::{
    DownloadRequest, DownloadResult, MarketStatus, RemoteSkill, RemoteSkillView,
    RemoteSkillsResponse, RemoteSkillsViewResponse,
};
use crate::utils::download::{download_bytes, download_skill_to_dir};
use std::collections::HashMap;
use std::path::PathBuf;

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
    format!("https://github.com/{}/{}", owner, repo)
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
            let github_owner = get_value_string(item, &["githubOwner", "github_owner", "owner"]);
            let github_repo = get_value_string(item, &["githubRepo", "github_repo", "repo"]);
            let source_url = get_value_string(item, &["githubUrl", "sourceUrl", "source_url"])
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
                    (Some(o), Some(r)) => Some(format!("{}/{}", o, r)),
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

            let name =
                get_value_string(item, &["name", "title"]).unwrap_or_else(|| "skill".to_string());
            let description =
                get_value_string(item, &["description", "summary"]).unwrap_or_default();
            let namespace = author.clone();
            let stars = get_value_u64(item, &["stars", "githubStars", "github_stars"]).unwrap_or(0);
            let installs = get_value_u64(item, &["installs", "downloads"]).unwrap_or(0);
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

#[allow(dead_code)]
fn parse_agent_skills_index(
    buf: &[u8],
    market_id: &str,
    market_label: &str,
) -> Result<(Vec<RemoteSkillView>, u64), String> {
    let value: serde_json::Value = serde_json::from_slice(buf).map_err(|err| err.to_string())?;

    let list = value
        .get("skills")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("data").and_then(|v| v.as_array()))
        .or_else(|| value.get("items").and_then(|v| v.as_array()))
        .or_else(|| {
            value
                .get("data")
                .and_then(|v| v.get("skills"))
                .and_then(|v| v.as_array())
        })
        .or_else(|| {
            value
                .get("data")
                .and_then(|v| v.get("items"))
                .and_then(|v| v.as_array())
        });

    let mut skills = Vec::new();
    if let Some(items) = list {
        for item in items {
            let owner = get_value_string(item, &["github_owner", "githubOwner", "owner"]);
            let repo = get_value_string(item, &["github_repo", "githubRepo", "repo"]);
            let source_url = get_value_string(
                item,
                &[
                    "sourceUrl",
                    "source_url",
                    "githubUrl",
                    "github_url",
                    "html_url",
                ],
            )
            .or_else(|| match (owner.as_deref(), repo.as_deref()) {
                (Some(o), Some(r)) => Some(build_github_source_url(o, r)),
                _ => None,
            })
            .unwrap_or_default();

            let name = get_value_string(item, &["name", "title"])
                .or_else(|| repo.clone())
                .unwrap_or_else(|| "skill".to_string());
            let description =
                get_value_string(item, &["description", "summary"]).unwrap_or_default();
            let author =
                get_value_string(item, &["author", "owner", "github_owner", "githubOwner"])
                    .unwrap_or_default();
            let namespace = get_value_string(item, &["namespace"])
                .or_else(|| owner.clone())
                .unwrap_or_default();
            let installs =
                get_value_u64(item, &["installs", "downloads", "download_count"]).unwrap_or(0);
            let stars =
                get_value_u64(item, &["stars", "stargazers_count", "github_stars"]).unwrap_or(0);
            let raw_id = get_value_string(item, &["id", "slug"])
                .or_else(|| match (owner.as_deref(), repo.as_deref()) {
                    (Some(o), Some(r)) => Some(format!("{}/{}", o, r)),
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

    let total = get_value_u64(&value, &["total", "count", "totalCount"])
        .or_else(|| {
            value
                .get("meta")
                .and_then(|meta| get_value_u64(meta, &["total", "count", "totalCount"]))
        })
        .unwrap_or(skills.len() as u64);

    Ok((skills, total))
}

#[tauri::command]
pub fn search_marketplaces(
    query: String,
    limit: u64,
    offset: u64,
    api_keys: HashMap<String, String>,
    enabled_markets: HashMap<String, bool>,
) -> Result<RemoteSkillsViewResponse, String> {
    let mut skills: Vec<RemoteSkillView> = Vec::new();
    let mut total: u64 = 0;
    let mut market_statuses: Vec<MarketStatus> = Vec::new();

    let trimmed = query.trim();
    let query_param = if trimmed.is_empty() {
        String::new()
    } else {
        format!("q={}", urlencoding::encode(trimmed))
    };

    let claude_market_id = "claude-plugins";
    let claude_market_label = "Claude Plugins";

    if *enabled_markets.get(claude_market_id).unwrap_or(&true) {
        let mut url = String::from("https://claude-plugins.dev/api/skills?");
        if !query_param.is_empty() {
            url.push_str(&query_param);
            url.push('&');
        }
        url.push_str(&format!("limit={}&offset={}", limit, offset));

        match download_bytes(
            &url,
            &[
                ("Accept", "application/json"),
                ("User-Agent", "skills-manager-gui/0.1"),
            ],
        ) {
            Ok(buf) => {
                if let Ok(parsed) = serde_json::from_slice::<RemoteSkillsResponse>(&buf) {
                    total += parsed.total;
                    skills.extend(parsed.skills.into_iter().map(|skill| {
                        map_claude_skill(skill, claude_market_id, claude_market_label)
                    }));
                    market_statuses.push(MarketStatus {
                        id: claude_market_id.to_string(),
                        name: claude_market_label.to_string(),
                        status: "online".to_string(),
                        error: None,
                    });
                } else {
                    market_statuses.push(MarketStatus {
                        id: claude_market_id.to_string(),
                        name: claude_market_label.to_string(),
                        status: "error".to_string(),
                        error: Some("Failed to parse response".to_string()),
                    });
                }
            }
            Err(e) => {
                println!("Error fetching from Claude Plugins: {}", e);
                market_statuses.push(MarketStatus {
                    id: claude_market_id.to_string(),
                    name: claude_market_label.to_string(),
                    status: "error".to_string(),
                    error: Some(e),
                });
            }
        }
    } else {
        market_statuses.push(MarketStatus {
            id: claude_market_id.to_string(),
            name: claude_market_label.to_string(),
            status: "online".to_string(),
            error: None,
        });
    }

    let skillsllm_market_id = "skillsllm";
    let skillsllm_market_label = "SkillsLLM";

    if *enabled_markets.get(skillsllm_market_id).unwrap_or(&true) {
        let skillsllm_page = (offset / limit).saturating_add(1);
        let mut skillsllm_url = String::from("https://skillsllm.com/api/skills?");
        if !query_param.is_empty() {
            skillsllm_url.push_str(&format!("search={}&", urlencoding::encode(trimmed)));
        }
        skillsllm_url.push_str(&format!("page={}&limit={}", skillsllm_page, limit));

        match download_bytes(
            &skillsllm_url,
            &[
                ("Accept", "application/json"),
                ("User-Agent", "skills-manager-gui/0.1"),
            ],
        ) {
            Ok(buf) => {
                if let Ok((parsed_skills, parsed_total)) =
                    parse_skillsllm(&buf, skillsllm_market_id, skillsllm_market_label)
                {
                    total += parsed_total;
                    skills.extend(parsed_skills);
                    market_statuses.push(MarketStatus {
                        id: skillsllm_market_id.to_string(),
                        name: skillsllm_market_label.to_string(),
                        status: "online".to_string(),
                        error: None,
                    });
                } else {
                    market_statuses.push(MarketStatus {
                        id: skillsllm_market_id.to_string(),
                        name: skillsllm_market_label.to_string(),
                        status: "error".to_string(),
                        error: Some("Failed to parse response".to_string()),
                    });
                }
            }
            Err(e) => {
                println!("Error fetching from SkillsLLM: {}", e);
                market_statuses.push(MarketStatus {
                    id: skillsllm_market_id.to_string(),
                    name: skillsllm_market_label.to_string(),
                    status: "error".to_string(),
                    error: Some(e),
                });
            }
        }
    } else {
        market_statuses.push(MarketStatus {
            id: skillsllm_market_id.to_string(),
            name: skillsllm_market_label.to_string(),
            status: "online".to_string(),
            error: None,
        });
    }

    let skillsmp_market_id = "skillsmp";
    let skillsmp_market_label = "SkillsMP";

    if *enabled_markets.get(skillsmp_market_id).unwrap_or(&false) {
        if let Some(api_key) = api_keys.get(skillsmp_market_id).filter(|k| !k.is_empty()) {
            let skillsmp_page = (offset / limit).saturating_add(1);
            let skillsmp_url = format!(
                "https://skillsmp.com/api/v1/skills/search?q={}&page={}&limit={}",
                urlencoding::encode(trimmed),
                skillsmp_page,
                limit
            );

            let auth_header = format!("Bearer {}", api_key);

            match download_bytes(
                &skillsmp_url,
                &[
                    ("Accept", "application/json"),
                    ("User-Agent", "skills-manager-gui/0.1"),
                    ("Authorization", &auth_header),
                ],
            ) {
                Ok(buf) => {
                    if let Ok((parsed_skills, parsed_total)) =
                        parse_skillsmp(&buf, skillsmp_market_id, skillsmp_market_label)
                    {
                        total += parsed_total;
                        skills.extend(parsed_skills);
                        market_statuses.push(MarketStatus {
                            id: skillsmp_market_id.to_string(),
                            name: skillsmp_market_label.to_string(),
                            status: "online".to_string(),
                            error: None,
                        });
                    } else {
                        market_statuses.push(MarketStatus {
                            id: skillsmp_market_id.to_string(),
                            name: skillsmp_market_label.to_string(),
                            status: "error".to_string(),
                            error: Some("Failed to parse response".to_string()),
                        });
                    }
                }
                Err(e) => {
                    println!("Error fetching from SkillsMP: {}", e);
                    market_statuses.push(MarketStatus {
                        id: skillsmp_market_id.to_string(),
                        name: skillsmp_market_label.to_string(),
                        status: "error".to_string(),
                        error: Some(e),
                    });
                }
            }
        } else {
            market_statuses.push(MarketStatus {
                id: skillsmp_market_id.to_string(),
                name: skillsmp_market_label.to_string(),
                status: "needs_key".to_string(),
                error: None,
            });
        }
    } else {
        market_statuses.push(MarketStatus {
            id: skillsmp_market_id.to_string(),
            name: skillsmp_market_label.to_string(),
            status: "needs_key".to_string(),
            error: None,
        });
    }

    Ok(RemoteSkillsViewResponse {
        skills,
        total,
        limit,
        offset,
        market_statuses,
    })
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
        download_skill_to_dir(&source_url, &skill_name, &install_base_dir, false)
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

    let source_url = request.source_url.clone();
    let skill_name = request.skill_name.clone();
    let install_base_dir = PathBuf::from(&request.install_base_dir);

    let result = tauri::async_runtime::spawn_blocking(move || {
        download_skill_to_dir(&source_url, &skill_name, &install_base_dir, true)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(DownloadResult {
        installed_path: result.display().to_string(),
    })
}

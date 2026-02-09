use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RemoteSkill {
    id: String,
    name: String,
    namespace: String,
    source_url: String,
    description: String,
    author: String,
    installs: u64,
    stars: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoteSkillsResponse {
    skills: Vec<RemoteSkill>,
    total: u64,
    limit: u64,
    offset: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RemoteSkillView {
    id: String,
    name: String,
    namespace: String,
    source_url: String,
    description: String,
    author: String,
    installs: u64,
    stars: u64,
    market_id: String,
    market_label: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MarketStatus {
    id: String,
    name: String,
    status: String, // "online" | "error"
    error: Option<String>,
}

#[derive(Serialize, Debug)]
struct RemoteSkillsViewResponse {
    skills: Vec<RemoteSkillView>,
    total: u64,
    limit: u64,
    offset: u64,
    market_statuses: Vec<MarketStatus>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LinkTarget {
    name: String,
    path: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InstallResult {
    installed_path: String,
    linked: Vec<String>,
    skipped: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DownloadRequest {
    source_url: String,
    skill_name: String,
    install_base_dir: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DownloadResult {
    installed_path: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LinkRequest {
    skill_path: String,
    skill_name: String,
    link_targets: Vec<LinkTarget>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LocalSkill {
    id: String,
    name: String,
    description: String,
    path: String,
    source: String,
    ide: Option<String>,
    used_by: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LocalScanRequest {
    project_dir: Option<String>,
    ide_dirs: Vec<IdeDir>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdeSkill {
    id: String,
    name: String,
    path: String,
    ide: String,
    source: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Overview {
    manager_skills: Vec<LocalSkill>,
    ide_skills: Vec<IdeSkill>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UninstallRequest {
    target_path: String,
    project_dir: Option<String>,
    ide_dirs: Vec<IdeDir>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdeDir {
    label: String,
    relative_dir: String,
}

fn sanitize_dir_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '.' {
            out.push('-');
        }
    }
    if out.is_empty() {
        "skill".to_string()
    } else {
        out.trim_matches('-').to_string()
    }
}

fn download_bytes(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new().redirects(5).build();
    let mut request = agent.get(url);
    for (key, value) in headers {
        request = request.set(*key, *value);
    }

    let response = request.call().map_err(|err| err.to_string())?;
    let mut buf = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|err| err.to_string())?;
    Ok(buf)
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

fn build_github_source_url(owner: &str, repo: &str) -> String {
    format!("https://github.com/{}/{}", owner, repo)
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

    // SkillsMP puts skills under data.skills
    let list = value
        .get("data")
        .and_then(|d| d.get("skills"))
        .and_then(|v| v.as_array());

    let mut skills = Vec::new();
    if let Some(items) = list {
        for item in items {
            let source_url = get_value_string(item, &["githubUrl", "sourceUrl", "source_url"])
                .unwrap_or_default();

            // Extract owner from author field
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

    // SkillsMP puts pagination under data.pagination
    let total = value
        .get("data")
        .and_then(|d| d.get("pagination"))
        .and_then(|p| get_value_u64(p, &["total", "count"]))
        .unwrap_or(skills.len() as u64);

    Ok((skills, total))
}

fn download_skill_to_dir(
    source_url: &str,
    skill_name: &str,
    install_base_dir: &Path,
    overwrite: bool,
) -> Result<PathBuf, String> {
    fs::create_dir_all(install_base_dir).map_err(|err| err.to_string())?;

    let safe_name = sanitize_dir_name(skill_name);
    let target_dir = install_base_dir.join(&safe_name);
    if target_dir.exists() {
        if overwrite {
            fs::remove_dir_all(&target_dir).map_err(|err| err.to_string())?;
        } else {
            return Err("目标目录已存在，请更换名称或先清理".to_string());
        }
    }

    let zip_url = format!(
        "https://github-zip-api.val.run/zip?source={}",
        urlencoding::encode(source_url)
    );
    let zip_buf = download_bytes(&zip_url, &[])?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("skills-manager-{}", timestamp));
    let extract_dir = temp_dir.join("extract");
    fs::create_dir_all(&extract_dir).map_err(|err| err.to_string())?;

    let result = (|| -> Result<PathBuf, String> {
        extract_zip(&zip_buf, &extract_dir)?;
        let selected_root = find_skill_root(&extract_dir, &safe_name)?;
        copy_dir_recursive(&selected_root, &target_dir)?;
        Ok(target_dir)
    })();

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|err| err.to_string())?;
    }

    result
}

fn extract_zip(buf: &[u8], extract_dir: &Path) -> Result<(), String> {
    let cursor = Cursor::new(buf);
    let mut zip = ZipArchive::new(cursor).map_err(|err| err.to_string())?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|err| err.to_string())?;
        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let out_path = extract_dir.join(enclosed);
        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|err| err.to_string())?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let mut outfile = fs::File::create(&out_path).map_err(|err| err.to_string())?;
        std::io::copy(&mut file, &mut outfile).map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn find_skill_root(extract_dir: &Path, expected: &str) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(extract_dir).max_depth(5) {
        let entry = entry.map_err(|err| err.to_string())?;
        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            if let Some(parent) = entry.path().parent() {
                candidates.push(parent.to_path_buf());
            }
        }
    }

    if candidates.is_empty() {
        return Ok(extract_dir.to_path_buf());
    }

    let expected_lower = expected.to_ascii_lowercase();
    if let Some(best) = candidates.iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase() == expected_lower)
            .unwrap_or(false)
    }) {
        return Ok(best.clone());
    }

    Ok(candidates[0].clone())
}

fn read_skill_metadata(skill_dir: &Path) -> (String, String) {
    let name = skill_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_string();

    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return (name, String::new());
    }

    let content = fs::read_to_string(&skill_file).unwrap_or_default();
    let mut lines = content.lines();

    let mut frontmatter_name: Option<String> = None;
    let mut description = String::new();

    let mut in_frontmatter = false;
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            }
            break;
        }
        if in_frontmatter {
            if let Some(value) = trimmed.strip_prefix("name:") {
                frontmatter_name = Some(value.trim().to_string());
            }
            continue;
        }
        if description.is_empty() && !trimmed.is_empty() && !trimmed.starts_with('#') {
            description = trimmed.to_string();
        }
    }

    let final_name = frontmatter_name.unwrap_or(name);
    (final_name, description)
}

fn resolve_canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in WalkDir::new(src) {
        let entry = entry.map_err(|err| err.to_string())?;
        let rel_path = entry
            .path()
            .strip_prefix(src)
            .map_err(|err| err.to_string())?;
        let target = dst.join(rel_path);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|err| err.to_string())?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            fs::copy(entry.path(), &target).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn is_symlink_to(path: &Path, target: &Path) -> bool {
    match fs::read_link(path) {
        Ok(link) => link == target,
        Err(_) => false,
    }
}

fn create_symlink_dir(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(target, link).map_err(|err| err.to_string())
    }
    #[cfg(target_family = "windows")]
    {
        std::os::windows::fs::symlink_dir(target, link).map_err(|err| err.to_string())
    }
}

fn is_safe_relative_dir(rel: &str) -> bool {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return false;
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return false;
    }
    for comp in path.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

#[cfg(target_family = "windows")]
fn create_junction_dir(target: &Path, link: &Path) -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            link.to_string_lossy().as_ref(),
            target.to_string_lossy().as_ref(),
        ])
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("mklink /J failed".to_string())
    }
}

#[tauri::command]
fn search_marketplaces(
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

    // --- Claude Plugins ---
    let claude_market_id = "claude-plugins";
    let claude_market_label = "Claude Plugins";

    // Check if market is enabled
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
    // --- SkillsLLM ---
    let skillsllm_market_id = "skillsllm";
    let skillsllm_market_label = "SkillsLLM";

    // Check if market is enabled
    if *enabled_markets.get(skillsllm_market_id).unwrap_or(&true) {
        let skillsllm_page = (offset / limit).saturating_add(1);
        let mut skillsllm_url = String::from("https://skillsllm.com/api/skills?");
        if !query_param.is_empty() {
            // SkillsLLM uses 'search' parameter
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

    // --- SkillsMP (requires API Key) ---
    let skillsmp_market_id = "skillsmp";
    let skillsmp_market_label = "SkillsMP";

    // Check if market is enabled AND API key is provided
    if *enabled_markets.get(skillsmp_market_id).unwrap_or(&false) {
        if let Some(api_key) = api_keys.get(skillsmp_market_id).filter(|k| !k.is_empty()) {
            let skillsmp_page = (offset / limit).saturating_add(1);
            // Correct API endpoint: /api/v1/skills/search
            // Note: q parameter is REQUIRED by SkillsMP API
            let skillsmp_url = format!(
                "https://skillsmp.com/api/v1/skills/search?q={}&page={}&limit={}",
                urlencoding::encode(trimmed),
                skillsmp_page,
                limit
            );

            // Use Authorization: Bearer header
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
            // No API key provided - show as needs configuration
            market_statuses.push(MarketStatus {
                id: skillsmp_market_id.to_string(),
                name: skillsmp_market_label.to_string(),
                status: "needs_key".to_string(),
                error: None,
            });
        }
    } else {
        // Market is disabled
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
fn download_marketplace_skill(request: DownloadRequest) -> Result<DownloadResult, String> {
    if request.install_base_dir.trim().is_empty() {
        return Err("安装目录不能为空".to_string());
    }

    let install_base_dir = PathBuf::from(&request.install_base_dir);
    let target_dir = download_skill_to_dir(
        &request.source_url,
        &request.skill_name,
        &install_base_dir,
        false,
    )?;

    Ok(DownloadResult {
        installed_path: target_dir.display().to_string(),
    })
}

#[tauri::command]
fn update_marketplace_skill(request: DownloadRequest) -> Result<DownloadResult, String> {
    if request.install_base_dir.trim().is_empty() {
        return Err("安装目录不能为空".to_string());
    }

    let install_base_dir = PathBuf::from(&request.install_base_dir);
    let target_dir = download_skill_to_dir(
        &request.source_url,
        &request.skill_name,
        &install_base_dir,
        true,
    )?;

    Ok(DownloadResult {
        installed_path: target_dir.display().to_string(),
    })
}

#[tauri::command]
fn link_local_skill(request: LinkRequest) -> Result<InstallResult, String> {
    let skill_path = PathBuf::from(&request.skill_path);
    if !skill_path.exists() {
        return Err("本地 skill 路径不存在".to_string());
    }
    let safe_name = sanitize_dir_name(&request.skill_name);

    let mut linked = Vec::new();
    let mut skipped = Vec::new();

    for target in request.link_targets {
        let target_base = PathBuf::from(&target.path);
        fs::create_dir_all(&target_base).map_err(|err| err.to_string())?;
        let link_path = target_base.join(&safe_name);

        if link_path.exists() {
            if is_symlink_to(&link_path, &skill_path) {
                skipped.push(format!("{}: 已链接", target.name));
                continue;
            }
            skipped.push(format!("{}: 目标已存在", target.name));
            continue;
        }

        let mut linked_done = false;
        if create_symlink_dir(&skill_path, &link_path).is_ok() {
            linked.push(format!("{}: {}", target.name, link_path.display()));
            linked_done = true;
        }

        #[cfg(target_family = "windows")]
        if !linked_done {
            if create_junction_dir(&skill_path, &link_path).is_ok() {
                linked.push(format!("{}: junction {}", target.name, link_path.display()));
                linked_done = true;
            }
        }

        if !linked_done {
            copy_dir_recursive(&skill_path, &link_path)?;
            linked.push(format!("{}: copy {}", target.name, link_path.display()));
        }
    }

    Ok(InstallResult {
        installed_path: skill_path.display().to_string(),
        linked,
        skipped,
    })
}

fn collect_skills_from_dir(base: &Path, source: &str, ide: Option<&str>) -> Vec<LocalSkill> {
    let mut skills = Vec::new();
    if !base.exists() {
        return skills;
    }

    for entry in WalkDir::new(base).max_depth(4) {
        let entry = match entry {
            Ok(item) => item,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
            continue;
        }
        let Some(skill_dir) = entry.path().parent() else {
            continue;
        };
        let (name, description) = read_skill_metadata(skill_dir);
        let path = skill_dir.to_path_buf();
        skills.push(LocalSkill {
            id: path.display().to_string(),
            name,
            description,
            path: path.display().to_string(),
            source: source.to_string(),
            ide: ide.map(|value| value.to_string()),
            used_by: Vec::new(),
        });
    }

    skills
}

#[tauri::command]
fn scan_overview(request: LocalScanRequest) -> Result<Overview, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;

    let manager_dir = home.join(".skills-manager/skills");
    let mut manager_skills = collect_skills_from_dir(&manager_dir, "manager", None);

    let ide_dirs = if request.ide_dirs.is_empty() {
        vec![
            ("Antigravity".to_string(), ".gemini/antigravity/skills".to_string()),
            ("Claude".to_string(), ".claude/skills".to_string()),
            ("CodeBuddy".to_string(), ".codebuddy/skills".to_string()),
            ("Codex".to_string(), ".codex/skills".to_string()),
            ("Cursor".to_string(), ".cursor/skills".to_string()),
            ("Kiro".to_string(), ".kiro/skills".to_string()),
            ("Qoder".to_string(), ".qoder/skills".to_string()),
            ("Trae".to_string(), ".trae/skills".to_string()),
            ("VSCode".to_string(), ".github/skills".to_string()),
            ("Windsurf".to_string(), ".windsurf/skills".to_string()),
        ]
    } else {
        request
            .ide_dirs
            .iter()
            .map(|item| (item.label.clone(), item.relative_dir.clone()))
            .collect()
    };

    let mut ide_skills: Vec<IdeSkill> = Vec::new();

    let mut manager_map: Vec<(PathBuf, usize)> = Vec::new();
    for (idx, skill) in manager_skills.iter().enumerate() {
        if let Some(path) = resolve_canonical(Path::new(&skill.path)) {
            manager_map.push((path, idx));
        }
    }

    for (label, rel) in &ide_dirs {
        let dir = home.join(rel);
        ide_skills.extend(collect_ide_skills(
            &dir,
            label,
            &manager_map,
            &mut manager_skills,
        ));
    }

    if let Some(project) = request.project_dir {
        let base = PathBuf::from(project);
        for (label, rel) in &ide_dirs {
            let dir = base.join(rel);
            ide_skills.extend(collect_ide_skills(
                &dir,
                label,
                &manager_map,
                &mut manager_skills,
            ));
        }
    }

    Ok(Overview {
        manager_skills,
        ide_skills,
    })
}

fn collect_ide_skills(
    base: &Path,
    ide_label: &str,
    manager_map: &[(PathBuf, usize)],
    manager_skills: &mut [LocalSkill],
) -> Vec<IdeSkill> {
    let mut skills = Vec::new();
    if !base.exists() {
        return skills;
    }

    for entry in WalkDir::new(base).max_depth(3).follow_links(false) {
        let entry = match entry {
            Ok(item) => item,
            Err(_) => continue,
        };
        let file_type = entry.file_type();
        if !file_type.is_dir() && !file_type.is_symlink() {
            continue;
        }
        let skill_dir = entry.path();
        if !skill_dir.join("SKILL.md").exists() {
            continue;
        }

        let (name, _) = read_skill_metadata(skill_dir);
        let path = skill_dir.to_path_buf();
        let source = if let Ok(link_target) = fs::read_link(&path) {
            if let Some(target) = resolve_canonical(&link_target) {
                for (manager_path, idx) in manager_map {
                    if *manager_path == target {
                        if let Some(skill) = manager_skills.get_mut(*idx) {
                            if !skill.used_by.contains(&ide_label.to_string()) {
                                skill.used_by.push(ide_label.to_string());
                            }
                        }
                        break;
                    }
                }
            }
            "link"
        } else {
            "local"
        };

        skills.push(IdeSkill {
            id: path.display().to_string(),
            name,
            path: path.display().to_string(),
            ide: ide_label.to_string(),
            source: source.to_string(),
        });
    }

    skills
}

#[tauri::command]
fn uninstall_skill(request: UninstallRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let mut allowed_roots = vec![home.join(".skills-manager/skills")];

    let ide_dirs = if request.ide_dirs.is_empty() {
        vec![
            ".gemini/antigravity/skills".to_string(),
            ".claude/skills".to_string(),
            ".codebuddy/skills".to_string(),
            ".codex/skills".to_string(),
            ".cursor/skills".to_string(),
            ".kiro/skills".to_string(),
            ".qoder/skills".to_string(),
            ".trae/skills".to_string(),
            ".github/skills".to_string(),
            ".windsurf/skills".to_string(),
        ]
    } else {
        request
            .ide_dirs
            .iter()
            .map(|item| item.relative_dir.clone())
            .collect()
    };

    for rel in ide_dirs {
        if !is_safe_relative_dir(&rel) {
            return Err("IDE 目录非法".to_string());
        }
        allowed_roots.push(home.join(rel));
    }
    if let Some(project) = request.project_dir {
        let base = PathBuf::from(project);
        allowed_roots.push(base.join(".codex/skills"));
        allowed_roots.push(base.join(".trae/skills"));
        allowed_roots.push(base.join(".opencode/skill"));
        allowed_roots.push(base.join(".skills-manager/skills"));
    }

    let target = PathBuf::from(&request.target_path);
    let parent = target.parent().unwrap_or(Path::new(&request.target_path));
    let parent_canon = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    let allowed_roots_canon: Vec<PathBuf> = allowed_roots
        .iter()
        .map(|root| fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()))
        .collect();
    let allowed = allowed_roots_canon
        .iter()
        .any(|root| parent_canon.starts_with(root));
    if !allowed {
        return Err("目标路径不在允许范围内".to_string());
    }

    let metadata = fs::symlink_metadata(&target).map_err(|err| err.to_string())?;
    if metadata.file_type().is_symlink() {
        if target.is_dir() {
            fs::remove_dir(&target).map_err(|err| err.to_string())?;
        } else {
            fs::remove_file(&target).map_err(|err| err.to_string())?;
        }
        return Ok("已移除链接".to_string());
    }

    fs::remove_dir_all(&target).map_err(|err| err.to_string())?;
    Ok("已卸载目录".to_string())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    source_path: String,
}

#[tauri::command]
fn import_local_skill(request: ImportRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let manager_dir = home.join(".skills-manager/skills");

    let source_path = PathBuf::from(&request.source_path);
    if !source_path.exists() {
        return Err("源路径不存在".to_string());
    }

    if !source_path.join("SKILL.md").exists() {
        return Err("该目录下缺少 SKILL.md 文件，不是有效的 Skill".to_string());
    }

    let (name, _) = read_skill_metadata(&source_path);
    let safe_name = sanitize_dir_name(&name);
    let target_dir = manager_dir.join(&safe_name);

    if target_dir.exists() {
        return Err(format!("目标 Skill 已存在: {}", safe_name));
    }

    fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
    copy_dir_recursive(&source_path, &target_dir)?;

    Ok(format!("已导入 Skill: {}", name))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            search_marketplaces,
            download_marketplace_skill,
            update_marketplace_skill,
            link_local_skill,
            scan_overview,
            uninstall_skill,
            import_local_skill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

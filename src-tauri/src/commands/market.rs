use crate::types::{DownloadRequest, DownloadResult, RemoteSkillView, RemoteSkillsViewResponse};
use crate::utils::download::download_skill_to_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const MARKET_SKILL_METADATA: &str = ".skills-manager.json";

static SKILLS_INDEX: OnceLock<Vec<CachedSkill>> = OnceLock::new();

#[derive(Deserialize, Debug, Clone)]
struct CachedSkill {
    slug: String,
    name: String,
    summary: String,
    #[serde(default)]
    summary_zh: String,
    source_url: String,
    category: String,
    author: String,
}

#[derive(Deserialize, Debug)]
struct SkillsIndex {
    skills: Vec<CachedSkill>,
}

#[derive(Serialize)]
struct InstalledSkillMetadata<'a> {
    source_url: &'a str,
}

fn load_skills_index() -> &'static Vec<CachedSkill> {
    SKILLS_INDEX.get_or_init(|| {
        let raw = include_str!("../../data/skills-index.json");
        let index: SkillsIndex =
            serde_json::from_str(raw).unwrap_or(SkillsIndex { skills: vec![] });
        index.skills
    })
}

fn matches_query(skill: &CachedSkill, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let keyword = query.to_ascii_lowercase();
    skill.name.to_ascii_lowercase().contains(&keyword)
        || skill.slug.to_ascii_lowercase().contains(&keyword)
        || skill.summary.to_ascii_lowercase().contains(&keyword)
        || skill.summary_zh.to_ascii_lowercase().contains(&keyword)
        || skill.category.to_ascii_lowercase().contains(&keyword)
        || skill.author.to_ascii_lowercase().contains(&keyword)
}

fn is_supported_market_source_url(source_url: &str) -> bool {
    let trimmed = source_url.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    let base_url = lower
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');

    if base_url.ends_with(".zip") && (lower.starts_with("https://") || lower.starts_with("http://"))
    {
        return true;
    }

    if !trimmed.starts_with("https://github.com/") {
        return false;
    }

    let path = trimmed
        .trim_start_matches("https://github.com/")
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_matches('/');
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    if parts.len() < 2 {
        return false;
    }

    !matches!(parts.get(2), Some(&"blob"))
}

fn write_installed_skill_metadata(
    installed_dir: &std::path::Path,
    source_url: &str,
) -> Result<(), String> {
    let metadata = InstalledSkillMetadata { source_url };
    let raw = serde_json::to_string_pretty(&metadata).map_err(|err| err.to_string())?;
    fs::write(installed_dir.join(MARKET_SKILL_METADATA), raw).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn search_marketplaces(
    query: String,
    limit: u64,
    offset: u64,
) -> Result<RemoteSkillsViewResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let all_skills = load_skills_index();
        let trimmed = query.trim();
        let limit = if limit == 0 { 20 } else { limit };

        let filtered: Vec<&CachedSkill> = all_skills
            .iter()
            .filter(|skill| is_supported_market_source_url(&skill.source_url))
            .filter(|skill| matches_query(skill, trimmed))
            .collect();

        let total = filtered.len() as u64;

        let skills: Vec<RemoteSkillView> = filtered
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|skill| RemoteSkillView {
                id: format!("cached:{}", skill.slug),
                name: skill.name.clone(),
                namespace: skill.category.clone(),
                source_url: skill.source_url.clone(),
                description: skill.summary.clone(),
                description_zh: skill.summary_zh.clone(),
                author: skill.author.clone(),
                installs: 0,
                stars: 0,
                market_id: "cached".to_string(),
                market_label: "Skills Index".to_string(),
            })
            .collect();

        Ok(RemoteSkillsViewResponse {
            skills,
            total,
            limit,
            offset,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{is_supported_market_source_url, matches_query, CachedSkill};

    #[test]
    fn recognizes_installable_market_sources() {
        assert!(is_supported_market_source_url(
            "https://github.com/owner/repo"
        ));
        assert!(is_supported_market_source_url(
            "https://github.com/owner/repo/tree/main/skills/example"
        ));
        assert!(is_supported_market_source_url(
            "https://example.com/files/skill.zip?download=1"
        ));
    }

    #[test]
    fn rejects_sources_the_downloader_cannot_install() {
        assert!(!is_supported_market_source_url(
            "https://officialskills.sh/anthropics/skills/docx"
        ));
        assert!(!is_supported_market_source_url(
            "https://github.com/owner/repo/blob/main/SKILL.md"
        ));
        assert!(!is_supported_market_source_url(
            "https://catalog.redhat.com/en/ai/skills/detail/example"
        ));
    }

    #[test]
    fn matches_chinese_summary_text() {
        let skill = CachedSkill {
            slug: "anthropics-docx".to_string(),
            name: "anthropics/docx".to_string(),
            summary: "Create, edit, and analyze Word documents".to_string(),
            summary_zh: "创建、编辑和分析Word文档".to_string(),
            source_url: "https://github.com/anthropics/skills/tree/main/skills/docx".to_string(),
            category: "Official Claude Skills".to_string(),
            author: "anthropics".to_string(),
        };

        assert!(matches_query(&skill, "文档"));
    }
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
        let installed_dir =
            download_skill_to_dir(&source_url, &skill_name, &install_base_dir, false)?;
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
        let installed_dir =
            download_skill_to_dir(&source_url, &skill_name, &install_base_dir, true)?;
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

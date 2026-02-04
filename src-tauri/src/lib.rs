use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
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
        let rel_path = entry.path().strip_prefix(src).map_err(|err| err.to_string())?;
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

#[tauri::command]
fn search_marketplace(query: String, limit: u64, offset: u64) -> Result<RemoteSkillsResponse, String> {
    let mut url = String::from("https://claude-plugins.dev/api/skills?");
    if !query.trim().is_empty() {
        url.push_str("q=");
        url.push_str(&urlencoding::encode(query.trim()));
        url.push('&');
    }
    url.push_str(&format!("limit={}&offset={}", limit, offset));

    let buf = download_bytes(
        &url,
        &[("Accept", "application/json"), ("User-Agent", "skills-manager-gui/0.1")],
    )?;
    let parsed: RemoteSkillsResponse =
        serde_json::from_slice(&buf).map_err(|err| err.to_string())?;
    if parsed.skills.is_empty() && parsed.total > 0 && parsed.offset >= parsed.total {
        return Err("远程返回结果为空，请重试".to_string());
    }
    Ok(parsed)
}

#[tauri::command]
fn download_marketplace_skill(request: DownloadRequest) -> Result<DownloadResult, String> {
    if request.install_base_dir.trim().is_empty() {
        return Err("安装目录不能为空".to_string());
    }

    let install_base_dir = PathBuf::from(&request.install_base_dir);
    let target_dir =
        download_skill_to_dir(&request.source_url, &request.skill_name, &install_base_dir, false)?;

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
    let target_dir =
        download_skill_to_dir(&request.source_url, &request.skill_name, &install_base_dir, true)?;

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

        create_symlink_dir(&skill_path, &link_path)?;
        linked.push(format!("{}: {}", target.name, link_path.display()));
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
        let Some(skill_dir) = entry.path().parent() else { continue };
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
            ("Antigravity".to_string(), ".agent/skills".to_string()),
            ("Claude".to_string(), ".claude/skills".to_string()),
            ("CodeBuddy".to_string(), ".codebuddy/skills".to_string()),
            ("Codex".to_string(), ".codex/skills".to_string()),
            ("Cursor".to_string(), ".cursor/skills".to_string()),
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
        ide_skills.extend(collect_ide_skills(&dir, label, &manager_map, &mut manager_skills));
    }

    if let Some(project) = request.project_dir {
        let base = PathBuf::from(project);
        for (label, rel) in &ide_dirs {
            let dir = base.join(rel);
            ide_skills.extend(collect_ide_skills(&dir, label, &manager_map, &mut manager_skills));
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
            ".agent/skills".to_string(),
            ".claude/skills".to_string(),
            ".codebuddy/skills".to_string(),
            ".codex/skills".to_string(),
            ".cursor/skills".to_string(),
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
    let allowed = allowed_roots.iter().any(|root| parent_canon.starts_with(root));
    if !allowed {
        return Err("目标路径不在允许范围内".to_string());
    }

    let metadata = fs::symlink_metadata(&target).map_err(|err| err.to_string())?;
    if metadata.file_type().is_symlink() {
        fs::remove_file(&target).map_err(|err| err.to_string())?;
        return Ok("已移除链接".to_string());
    }

    fs::remove_dir_all(&target).map_err(|err| err.to_string())?;
    Ok("已卸载目录".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            search_marketplace,
            download_marketplace_skill,
            update_marketplace_skill,
            link_local_skill,
            scan_overview,
            uninstall_skill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

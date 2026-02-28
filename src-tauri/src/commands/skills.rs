use crate::types::{
    IdeSkill, ImportRequest, InstallResult, LinkRequest, LocalScanRequest, LocalSkill,
    Overview, UninstallRequest,
};
use crate::utils::download::copy_dir_recursive;
use crate::utils::path::{normalize_path, resolve_canonical, sanitize_dir_name};
use crate::utils::security::is_safe_relative_dir;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
            let absolute_target = if link_target.is_relative() {
                if let Some(parent) = path.parent() {
                    parent.join(&link_target)
                } else {
                    link_target.clone()
                }
            } else {
                link_target
            };
            if let Some(target) = resolve_canonical(&absolute_target) {
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

#[cfg(target_family = "windows")]
fn create_junction_dir(target: &Path, link: &Path) -> Result<(), String> {
    use std::process::Command;

    fn validate_path(path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy();
        let dangerous_chars = ['|', '^', '<', '>', '%', '!', '"'];
        for ch in dangerous_chars {
            if path_str.contains(ch) {
                return Err(format!("Path contains dangerous character: '{}'", ch));
            }
        }
        Ok(())
    }

    validate_path(target)?;
    validate_path(link)?;

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
pub fn link_local_skill(request: LinkRequest) -> Result<InstallResult, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let normalized_home = normalize_path(&home);

    let skill_path = PathBuf::from(&request.skill_path);
    let skill_canon =
        fs::canonicalize(&skill_path).map_err(|_| "本地 skill 路径不存在".to_string())?;
    if !skill_canon.starts_with(&normalized_home) {
        return Err("本地 skill 路径必须位于用户目录下".to_string());
    }
    let skill_path = skill_canon;

    let safe_name = sanitize_dir_name(&request.skill_name);

    let mut linked = Vec::new();
    let mut skipped = Vec::new();

    for target in request.link_targets {
        let target_base = PathBuf::from(&target.path);
        let normalized_target = normalize_path(&target_base);
        if !normalized_target.starts_with(&normalized_home) {
            return Err(format!("目标目录超出用户目录：{}", target.name));
        }
        // 对每个目标路径进行符号链接攻击防护验证
        let target_canon = fs::canonicalize(&target_base).unwrap_or_else(|_| normalized_target.clone());
        if !target_canon.starts_with(&normalized_home) {
            return Err(format!("目标目录存在符号链接攻击风险：{}", target.name));
        }

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

#[tauri::command]
pub fn scan_overview(request: LocalScanRequest) -> Result<Overview, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;

    let manager_dir = home.join(".skills-manager/skills");
    let mut manager_skills = collect_skills_from_dir(&manager_dir, "manager", None);

    let ide_dirs = if request.ide_dirs.is_empty() {
        vec![
            (
                "Antigravity".to_string(),
                ".gemini/antigravity/skills".to_string(),
            ),
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

#[tauri::command]
pub fn uninstall_skill(request: UninstallRequest) -> Result<String, String> {
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

#[tauri::command]
pub fn import_local_skill(request: ImportRequest) -> Result<String, String> {
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
        return Err(format!("目标 Skill 已存在：{}", safe_name));
    }

    fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
    copy_dir_recursive(&source_path, &target_dir)?;

    Ok(format!("已导入 Skill: {}", name))
}

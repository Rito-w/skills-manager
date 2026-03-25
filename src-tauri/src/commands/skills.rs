use crate::types::{
    AdoptIdeSkillRequest, DeleteLocalSkillRequest, ExportSkillsRequest, IdeSkill, ImportRequest,
    InstallResult, LinkRequest, LocalScanRequest, LocalSkill, Overview, ProjectIdeDir,
    ProjectScanRequest, ProjectScanResult, UninstallRequest,
};
use crate::utils::download::copy_dir_recursive;
use crate::utils::path::{normalize_path, resolve_canonical, sanitize_dir_name};
use crate::utils::security::{is_absolute_ide_path, is_valid_ide_path};
use std::fs::File;
use std::io;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

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
    let lines = content.lines();

    let mut frontmatter_name: Option<String> = None;
    let mut description = String::new();

    let mut in_frontmatter = false;
    for line in lines {
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

    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return skills,
    };

    for entry in entries {
        let entry = match entry {
            Ok(item) => item,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() || !path.join("SKILL.md").exists() {
            continue;
        }
        let (name, description) = read_skill_metadata(&path);
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

    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return skills,
    };

    for entry in entries {
        let entry = match entry {
            Ok(item) => item,
            Err(_) => continue,
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let link_target = fs::read_link(&path).ok();
        if !metadata.is_dir() && link_target.is_none() {
            continue;
        }

        let skill_dir = path.as_path();
        let has_skill_file = skill_dir.join("SKILL.md").exists();
        if !has_skill_file && link_target.is_none() {
            continue;
        }

        let name = if has_skill_file {
            read_skill_metadata(skill_dir).0
        } else {
            skill_dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill")
                .to_string()
        };

        let path = skill_dir.to_path_buf();
        let mut managed = false;
        let source = if let Some(link_target) = link_target {
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
                        managed = true;
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
            managed,
        });
    }

    skills
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|err| err.to_string())?;
    if metadata.file_type().is_symlink() {
        // `path.is_dir()` follows symlinks and may report true for a symlink-to-dir.
        // Removing such a symlink with `remove_dir` triggers ENOTDIR on macOS.
        fs::remove_file(path)
            .or_else(|_| fs::remove_dir(path))
            .map_err(|err| err.to_string())
    } else if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|err| err.to_string())
    } else {
        fs::remove_file(path).map_err(|err| err.to_string())
    }
}

fn is_symlink_to(path: &Path, target: &Path) -> bool {
    match (resolve_canonical(path), resolve_canonical(target)) {
        (Some(link_target), Some(expected_target)) => link_target == expected_target,
        _ => false,
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

fn validate_manager_skill_path(target: &Path, manager_root: &Path) -> Result<PathBuf, String> {
    let canonical =
        resolve_canonical(target).ok_or_else(|| "Target skill does not exist".to_string())?;
    if !canonical.starts_with(manager_root) {
        return Err("Only Skills Manager local skills can be exported".to_string());
    }
    if canonical == manager_root {
        return Err("Refusing to export the skills root directory".to_string());
    }
    if !canonical.join("SKILL.md").exists() {
        return Err("Refusing to export a directory without SKILL.md".to_string());
    }
    Ok(canonical)
}

fn ensure_export_path_is_safe(export_path: &Path, skill_paths: &[PathBuf]) -> Result<(), String> {
    let file_name = export_path
        .file_name()
        .ok_or_else(|| "Export path must include a file name".to_string())?;
    let export_parent = export_path
        .parent()
        .ok_or_else(|| "Export path must include a parent directory".to_string())?;
    let normalized_export_parent =
        resolve_canonical(export_parent).unwrap_or_else(|| normalize_path(export_parent));
    let normalized_export = normalized_export_parent.join(file_name);
    for skill_path in skill_paths {
        if normalized_export.starts_with(skill_path) {
            return Err("Export path cannot be inside a selected skill directory".to_string());
        }
    }
    Ok(())
}

fn zip_skill_directory(
    zip: &mut ZipWriter<File>,
    skill_path: &Path,
    root_name: &str,
) -> Result<(), String> {
    let dir_options = || {
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755)
    };
    let file_options = || {
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644)
    };

    let root_dir = format!("{}/", root_name);
    zip.add_directory(&root_dir, dir_options())
        .map_err(|err| err.to_string())?;

    for entry in WalkDir::new(skill_path) {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type();

        if file_type.is_symlink() {
            return Err(format!(
                "Refusing to export symlinked content: {}",
                path.display()
            ));
        }
        if path == skill_path {
            continue;
        }

        let rel_path = path
            .strip_prefix(skill_path)
            .map_err(|err| err.to_string())?;
        let zip_path = format!(
            "{}/{}",
            root_name,
            rel_path.to_string_lossy().replace('\\', "/")
        );

        if file_type.is_dir() {
            zip.add_directory(format!("{}/", zip_path), dir_options())
                .map_err(|err| err.to_string())?;
            continue;
        }

        let mut file = File::open(path).map_err(|err| err.to_string())?;
        zip.start_file(zip_path, file_options())
            .map_err(|err| err.to_string())?;
        io::copy(&mut file, zip).map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[cfg(target_family = "windows")]
fn create_junction_dir(target: &Path, link: &Path) -> Result<(), String> {
    use std::process::Command;

    fn to_cmd_path(path: &Path) -> String {
        path.to_string_lossy().replace('/', "\\")
    }

    fn validate_path(path: &str) -> Result<(), String> {
        let dangerous_chars = ['|', '^', '<', '>', '%', '!', '"', '&', '(', ')', ';'];
        for ch in dangerous_chars {
            if path.contains(ch) {
                return Err(format!("Path contains dangerous character: '{}'", ch));
            }
        }
        Ok(())
    }

    let target = to_cmd_path(target);
    let link = to_cmd_path(link);

    validate_path(&target)?;
    validate_path(&link)?;

    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", &link, &target])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "unknown error".to_string()
        };
        Err(format!("mklink /J failed: {}", detail))
    }
}

#[tauri::command]
pub fn link_local_skill(request: LinkRequest) -> Result<InstallResult, String> {
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;
    let normalized_home = normalize_path(&home);
    let manager_root_raw = home.join(".skills-manager/skills");
    let manager_root =
        resolve_canonical(&manager_root_raw).unwrap_or_else(|| normalize_path(&manager_root_raw));

    let skill_path = PathBuf::from(&request.skill_path);
    let skill_canon = resolve_canonical(&skill_path)
        .ok_or_else(|| "Local skill path does not exist".to_string())?;
    if !skill_canon.starts_with(&manager_root) {
        return Err("Local skill path must stay inside Skills Manager storage".to_string());
    }
    let skill_path = skill_canon;

    let safe_name = sanitize_dir_name(&request.skill_name);

    let mut linked = Vec::new();
    let mut skipped = Vec::new();

    for target in request.link_targets {
        let target_base = PathBuf::from(&target.path);
        let normalized_target = normalize_path(&target_base);
        if !normalized_target.starts_with(&normalized_home) {
            return Err(format!(
                "Target directory is outside the home directory: {}",
                target.name
            ));
        }

        // Normalize resolved paths before comparison so Windows verbatim prefixes do not
        // trigger false-positive symlink attack errors.
        let target_canon =
            resolve_canonical(&target_base).unwrap_or_else(|| normalized_target.clone());
        if !target_canon.starts_with(&normalized_home) {
            return Err(format!(
                "Target directory failed the symlink safety check: {}",
                target.name
            ));
        }

        fs::create_dir_all(&target_base).map_err(|err| err.to_string())?;
        let link_path = target_base.join(&safe_name);

        if fs::symlink_metadata(&link_path).is_ok() {
            if is_symlink_to(&link_path, &skill_path) {
                skipped.push(format!("{}: already linked", target.name));
                continue;
            }
            skipped.push(format!("{}: target already exists", target.name));
            continue;
        }

        let mut linked_done = false;
        let mut link_errors = Vec::new();

        match create_symlink_dir(&skill_path, &link_path) {
            Ok(()) => {
                linked.push(format!("{}: {}", target.name, link_path.display()));
                linked_done = true;
            }
            Err(err) => link_errors.push(format!("symlink: {}", err)),
        }

        #[cfg(target_family = "windows")]
        if !linked_done {
            match create_junction_dir(&skill_path, &link_path) {
                Ok(()) => {
                    linked.push(format!("{}: junction {}", target.name, link_path.display()));
                    linked_done = true;
                }
                Err(err) => link_errors.push(format!("junction: {}", err)),
            }
        }

        if !linked_done {
            let detail = if link_errors.is_empty() {
                "unknown error".to_string()
            } else {
                link_errors.join("; ")
            };
            return Err(format!(
                "Failed to create a link for {} in {}: {}",
                request.skill_name, target.name, detail
            ));
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
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;

    let manager_dir = home.join(".skills-manager/skills");
    let mut manager_skills = collect_skills_from_dir(&manager_dir, "manager", None);

    // Resolve IDE directories: absolute paths are used directly, relative paths are joined with home
    let ide_dirs: Vec<(String, PathBuf)> = if request.ide_dirs.is_empty() {
        vec![
            (
                "Antigravity".to_string(),
                home.join(".gemini/antigravity/skills"),
            ),
            ("Claude".to_string(), home.join(".claude/skills")),
            ("CodeBuddy".to_string(), home.join(".codebuddy/skills")),
            ("Codex".to_string(), home.join(".codex/skills")),
            ("Cursor".to_string(), home.join(".cursor/skills")),
            ("Kiro".to_string(), home.join(".kiro/skills")),
            ("Qoder".to_string(), home.join(".qoder/skills")),
            ("Trae".to_string(), home.join(".trae/skills")),
            ("VSCode".to_string(), home.join(".github/skills")),
            ("Windsurf".to_string(), home.join(".windsurf/skills")),
        ]
    } else {
        request
            .ide_dirs
            .iter()
            .map(|item| {
                if !is_valid_ide_path(&item.relative_dir) {
                    return Err(format!("Invalid IDE directory: {}", item.label));
                }
                // Absolute path: use directly
                if is_absolute_ide_path(&item.relative_dir) {
                    Ok((item.label.clone(), PathBuf::from(&item.relative_dir)))
                } else {
                    // Relative path: join with home directory
                    Ok((item.label.clone(), home.join(&item.relative_dir)))
                }
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    let mut ide_skills: Vec<IdeSkill> = Vec::new();

    let mut manager_map: Vec<(PathBuf, usize)> = Vec::new();
    for (idx, skill) in manager_skills.iter().enumerate() {
        if let Some(path) = resolve_canonical(Path::new(&skill.path)) {
            manager_map.push((path, idx));
        }
    }

    for (label, dir) in &ide_dirs {
        ide_skills.extend(collect_ide_skills(
            dir,
            label,
            &manager_map,
            &mut manager_skills,
        ));
    }

    if let Some(project) = request.project_dir {
        let base = PathBuf::from(project);
        for (label, dir) in &ide_dirs {
            // For absolute paths, also check the same path under project
            // For relative paths, join with project directory
            let project_dir = if dir.is_absolute() {
                dir.clone()
            } else {
                base.join(dir)
            };
            ide_skills.extend(collect_ide_skills(
                &project_dir,
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
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;
    let mut allowed_roots = vec![home.join(".skills-manager/skills")];

    let ide_dirs: Vec<String> = if request.ide_dirs.is_empty() {
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

    for dir in &ide_dirs {
        if !is_valid_ide_path(dir) {
            return Err("Invalid IDE directory".to_string());
        }
        // Absolute path: add directly to allowed roots
        if is_absolute_ide_path(dir) {
            allowed_roots.push(PathBuf::from(dir));
        } else {
            // Relative path: join with home directory
            allowed_roots.push(home.join(dir));
        }
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
    let parent_canon = resolve_canonical(parent).unwrap_or_else(|| normalize_path(parent));
    let allowed_roots_canon: Vec<PathBuf> = allowed_roots
        .iter()
        .map(|root| resolve_canonical(root).unwrap_or_else(|| normalize_path(root)))
        .collect();
    let allowed = allowed_roots_canon
        .iter()
        .any(|root| parent_canon.starts_with(root));
    if !allowed {
        return Err("Target path is outside the allowed directories".to_string());
    }

    let metadata = fs::symlink_metadata(&target).map_err(|err| err.to_string())?;
    if metadata.file_type().is_symlink() {
        // `target.is_dir()` follows symlinks and may report true for a symlink-to-dir.
        // Removing such a symlink with `remove_dir` triggers ENOTDIR/ENOTEMPTY on macOS.
        fs::remove_file(&target)
            .or_else(|_| fs::remove_dir(&target))
            .map_err(|err| err.to_string())?;
        return Ok("Link removed".to_string());
    }

    fs::remove_dir_all(&target).map_err(|err| err.to_string())?;
    Ok("Directory removed".to_string())
}

#[tauri::command]
pub fn import_local_skill(request: ImportRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;
    let manager_dir = home.join(".skills-manager/skills");

    let source_path = PathBuf::from(&request.source_path);
    if !source_path.exists() {
        return Err("Source path does not exist".to_string());
    }

    if !source_path.join("SKILL.md").exists() {
        return Err("The selected directory does not contain SKILL.md".to_string());
    }

    let (name, _) = read_skill_metadata(&source_path);
    let safe_name = sanitize_dir_name(&name);
    let target_dir = manager_dir.join(&safe_name);

    if target_dir.exists() {
        return Err(format!("Target skill already exists: {}", safe_name));
    }

    fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
    copy_dir_recursive(&source_path, &target_dir)?;

    Ok(format!("Imported skill: {}", name))
}

#[tauri::command]
pub fn adopt_ide_skill(request: AdoptIdeSkillRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Unable to determine the home directory".to_string())?;
    let normalized_home = normalize_path(&home);
    let manager_root = home.join(".skills-manager/skills");
    fs::create_dir_all(&manager_root).map_err(|err| err.to_string())?;

    let target = PathBuf::from(&request.target_path);
    let normalized_target = normalize_path(&target);
    if !normalized_target.starts_with(&normalized_home) {
        return Err("IDE skill path must stay inside the home directory".to_string());
    }

    fs::symlink_metadata(&target).map_err(|_| "IDE skill path does not exist".to_string())?;
    let target_canon = resolve_canonical(&target);

    let (name, has_skill_file) = if let Some(path) = target_canon.as_ref() {
        (read_skill_metadata(path).0, path.join("SKILL.md").exists())
    } else {
        (
            target
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill")
                .to_string(),
            false,
        )
    };

    let safe_name = sanitize_dir_name(&name);
    let manager_target = manager_root.join(&safe_name);

    if manager_target.exists() {
        let manager_canon = resolve_canonical(&manager_target)
            .ok_or_else(|| "Managed skill path does not exist".to_string())?;
        if target_canon
            .as_ref()
            .is_some_and(|target_path| *target_path == manager_canon)
        {
            return Ok(format!("{} is already managed", name));
        }
    } else {
        let source_dir = target_canon
            .as_ref()
            .ok_or_else(|| "IDE skill path does not exist".to_string())?;
        if !has_skill_file {
            return Err("Target directory does not contain SKILL.md".to_string());
        }
        copy_dir_recursive(source_dir, &manager_target)?;
    }

    remove_path(&target)?;

    let mut linked_done = false;
    let mut link_errors = Vec::new();

    match create_symlink_dir(&manager_target, &target) {
        Ok(()) => linked_done = true,
        Err(err) => link_errors.push(format!("symlink: {}", err)),
    }

    #[cfg(target_family = "windows")]
    if !linked_done {
        match create_junction_dir(&manager_target, &target) {
            Ok(()) => linked_done = true,
            Err(err) => link_errors.push(format!("junction: {}", err)),
        }
    }

    if !linked_done {
        copy_dir_recursive(&manager_target, &target)?;
        let detail = if link_errors.is_empty() {
            "unknown error".to_string()
        } else {
            link_errors.join("; ")
        };
        return Err(format!(
            "Managed {} in Skills Manager, but failed to create a link for {}. Restored a local copy instead. {}",
            name, request.ide_label, detail
        ));
    }

    Ok(format!(
        "Managed {} and re-linked it to {}",
        name, request.ide_label
    ))
}

#[tauri::command]
pub fn delete_local_skills(request: DeleteLocalSkillRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;
    let manager_root = resolve_canonical(&home.join(".skills-manager/skills"))
        .unwrap_or_else(|| normalize_path(&home.join(".skills-manager/skills")));

    if request.target_paths.is_empty() {
        return Err("No skills were provided for deletion".to_string());
    }

    let mut deleted = 0usize;

    for raw_path in request.target_paths {
        let target = PathBuf::from(&raw_path);
        let canonical =
            resolve_canonical(&target).ok_or_else(|| "Target skill does not exist".to_string())?;
        if !canonical.starts_with(&manager_root) {
            return Err("Only Skills Manager local skills can be deleted".to_string());
        }
        if canonical == manager_root {
            return Err("Refusing to delete the skills root directory".to_string());
        }
        if !canonical.join("SKILL.md").exists() {
            return Err("Refusing to delete a directory without SKILL.md".to_string());
        }

        fs::remove_dir_all(&canonical).map_err(|err| err.to_string())?;
        deleted += 1;
    }

    Ok(format!("Deleted {} skills", deleted))
}

#[tauri::command]
pub fn export_local_skills(request: ExportSkillsRequest) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Unable to determine the home directory")?;
    let manager_root = resolve_canonical(&home.join(".skills-manager/skills"))
        .unwrap_or_else(|| normalize_path(&home.join(".skills-manager/skills")));

    if request.target_paths.is_empty() {
        return Err("No skills were provided for export".to_string());
    }
    if request.export_path.trim().is_empty() {
        return Err("Export path is required".to_string());
    }

    let export_path = PathBuf::from(&request.export_path);
    let export_parent = export_path
        .parent()
        .ok_or_else(|| "Export path must include a parent directory".to_string())?;
    fs::create_dir_all(export_parent).map_err(|err| err.to_string())?;

    let mut skill_paths = Vec::new();
    for raw_path in request.target_paths {
        let canonical = validate_manager_skill_path(&PathBuf::from(raw_path), &manager_root)?;
        skill_paths.push(canonical);
    }

    ensure_export_path_is_safe(&export_path, &skill_paths)?;

    let file = File::create(&export_path).map_err(|err| err.to_string())?;
    let mut zip = ZipWriter::new(file);

    for skill_path in &skill_paths {
        let root_name = skill_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("skill");
        if let Err(err) = zip_skill_directory(&mut zip, skill_path, root_name) {
            let _ = zip.finish();
            let _ = fs::remove_file(&export_path);
            return Err(err);
        }
    }

    zip.finish().map_err(|err| err.to_string())?;
    Ok(export_path.display().to_string())
}

#[tauri::command]
pub fn scan_project_ide_dirs(request: ProjectScanRequest) -> Result<ProjectScanResult, String> {
    let project_dir = PathBuf::from(&request.project_dir);

    if !project_dir.exists() {
        return Err("Project directory does not exist".to_string());
    }

    let ide_dir_patterns = [
        (".gemini/antigravity/skills", "Antigravity"),
        (".claude/skills", "Claude Code"),
        (".codebuddy/skills", "CodeBuddy"),
        (".codex/skills", "Codex"),
        (".cursor/skills", "Cursor"),
        (".kiro/skills", "Kiro"),
        (".openclaw/skills", "OpenClaw"),
        (".config/opencode/skills", "OpenCode"),
        (".qoder/skills", "Qoder"),
        (".trae/skills", "Trae"),
        (".github/skills", "VSCode"),
        (".windsurf/skills", "Windsurf"),
    ];

    let mut detected_ide_dirs = Vec::new();

    for (relative_path, label) in ide_dir_patterns.iter() {
        let ide_path = project_dir.join(relative_path);
        if ide_path.exists() && ide_path.is_dir() {
            detected_ide_dirs.push(ProjectIdeDir {
                label: label.to_string(),
                relative_dir: relative_path.to_string(),
                absolute_path: ide_path.display().to_string(),
            });
        }
    }

    Ok(ProjectScanResult {
        project_dir: request.project_dir,
        detected_ide_dirs,
    })
}

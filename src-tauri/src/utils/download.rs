use crate::utils::path::{normalize_path, sanitize_dir_name};
use crate::utils::security::is_within_directory;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn download_bytes(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .redirects(5)
        .timeout(std::time::Duration::from_secs(60))
        .build();
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

pub fn download_skill_to_dir(
    source_url: &str,
    skill_name: &str,
    install_base_dir: &Path,
    overwrite: bool,
) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let allowed_base = normalize_path(&home.join(".skills-manager/skills"));
    let requested_base = normalize_path(install_base_dir);
    if !requested_base.starts_with(&allowed_base) {
        return Err("安装目录不在允许范围内".to_string());
    }

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

    // 使用 defer 模式确保 temp 目录总是被清理
    let _temp_dir_guard = TempDirGuard::new(&temp_dir);

    extract_zip(&zip_buf, &extract_dir)?;
    let selected_root = find_skill_root(&extract_dir, &safe_name)?;
    copy_dir_recursive(&selected_root, &target_dir)?;

    Ok(target_dir)
}

/// RAII guard for automatic temp directory cleanup
struct TempDirGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl<'a> TempDirGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, armed: true }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl<'a> Drop for TempDirGuard<'a> {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(self.path);
        }
    }
}

pub fn extract_zip(buf: &[u8], extract_dir: &Path) -> Result<(), String> {
    let cursor = Cursor::new(buf);
    let mut zip = ZipArchive::new(cursor).map_err(|err| err.to_string())?;

    let canonical_extract = extract_dir
        .canonicalize()
        .unwrap_or_else(|_| extract_dir.to_path_buf());

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|err| err.to_string())?;
        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let out_path = extract_dir.join(&enclosed);

        if !is_within_directory(&canonical_extract, &out_path) {
            return Err(format!(
                "Zip Slip attack detected: {} attempts to write outside of {}",
                enclosed.display(),
                extract_dir.display()
            ));
        }

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

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in WalkDir::new(src) {
        let entry = entry.map_err(|err| err.to_string())?;
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            return Err(format!(
                "检测到符号链接，已拒绝复制: {}",
                entry.path().display()
            ));
        }
        let rel_path = entry
            .path()
            .strip_prefix(src)
            .map_err(|err| err.to_string())?;
        let target = dst.join(rel_path);
        if file_type.is_dir() {
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

use crate::utils::path::{normalize_path, sanitize_dir_name};
use crate::utils::security::is_within_directory;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use zip::ZipArchive;

const GITHUB_WEB_PREFIX: &str = "https://github.com/";
const USER_AGENT: &str = "skills-manager-gui/0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadSource {
    GitHubRepo {
        owner: String,
        repo: String,
    },
    GitHubTree {
        owner: String,
        repo: String,
        git_ref: String,
        subpath: PathBuf,
    },
    ZipUrl {
        url: String,
    },
}

pub fn download_bytes(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    download_bytes_with_timeout(url, headers, 60)
}

pub fn download_market_bytes(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    download_bytes_with_timeout(url, headers, 4)
}

fn download_bytes_with_timeout(
    url: &str,
    headers: &[(&str, &str)],
    timeout_secs: u64,
) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .redirects(5)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build();
    let mut request = agent.get(url);
    for (key, value) in headers {
        request = request.set(key, value);
    }

    let response = request.call().map_err(|err| err.to_string())?;
    let mut buf = Vec::new();

    const MAX_DOWNLOAD_SIZE: u64 = 50 * 1024 * 1024;
    response
        .into_reader()
        .take(MAX_DOWNLOAD_SIZE)
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

    let parsed_source = parse_download_source(source_url)?;
    let preferred_subpath = parsed_source.preferred_subpath();
    let zip_buf = download_archive_bytes(&parsed_source)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("skills-manager-{}", timestamp));
    let extract_dir = temp_dir.join("extract");
    fs::create_dir_all(&extract_dir).map_err(|err| err.to_string())?;

    let _temp_dir_guard = TempDirGuard::new(&temp_dir);

    extract_zip(&zip_buf, &extract_dir)?;
    let selected_root = find_skill_root(&extract_dir, &safe_name, preferred_subpath.as_deref())?;
    copy_dir_recursive(&selected_root, &target_dir)?;

    Ok(target_dir)
}

fn download_archive_bytes(source: &DownloadSource) -> Result<Vec<u8>, String> {
    match source {
        DownloadSource::GitHubRepo { owner, repo } => {
            let archive_url = format!("https://api.github.com/repos/{owner}/{repo}/zipball/HEAD");
            download_bytes(
                &archive_url,
                &[
                    ("Accept", "application/vnd.github+json"),
                    ("X-GitHub-Api-Version", "2022-11-28"),
                    ("User-Agent", USER_AGENT),
                ],
            )
        }
        DownloadSource::GitHubTree {
            owner,
            repo,
            git_ref,
            ..
        } => {
            let archive_url = format!(
                "https://api.github.com/repos/{owner}/{repo}/zipball/{}",
                urlencoding::encode(git_ref)
            );
            download_bytes(
                &archive_url,
                &[
                    ("Accept", "application/vnd.github+json"),
                    ("X-GitHub-Api-Version", "2022-11-28"),
                    ("User-Agent", USER_AGENT),
                ],
            )
        }
        DownloadSource::ZipUrl { url } => download_bytes(url, &[("User-Agent", USER_AGENT)]),
    }
}

fn parse_download_source(source_url: &str) -> Result<DownloadSource, String> {
    let trimmed = source_url.trim();
    if trimmed.is_empty() {
        return Err("缺少有效的源码地址 (Source URL)".to_string());
    }

    if let Some(github) = parse_github_source(trimmed)? {
        return Ok(github);
    }

    if is_supported_zip_url(trimmed) {
        return Ok(DownloadSource::ZipUrl {
            url: trimmed.to_string(),
        });
    }

    Err("仅支持 GitHub 仓库链接、GitHub 子目录链接或 ZIP 下载链接".to_string())
}

fn parse_github_source(source_url: &str) -> Result<Option<DownloadSource>, String> {
    let Some(stripped) = source_url.strip_prefix(GITHUB_WEB_PREFIX) else {
        return Ok(None);
    };

    let path_without_query = stripped
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_matches('/');
    let parts: Vec<&str> = path_without_query
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    if parts.len() < 2 {
        return Err("GitHub 链接格式无效，至少需要 owner/repo".to_string());
    }

    let owner = parts[0].to_string();
    let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]).to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err("GitHub 链接格式无效，缺少 owner 或 repo".to_string());
    }

    if parts.len() == 2 {
        return Ok(Some(DownloadSource::GitHubRepo { owner, repo }));
    }

    match parts[2] {
        "tree" => {
            if parts.len() < 5 {
                return Err("GitHub 子目录链接格式无效，缺少分支或路径".to_string());
            }
            let git_ref = parts[3].to_string();
            let subpath = sanitize_relative_subpath(&parts[4..].join("/"))?;
            Ok(Some(DownloadSource::GitHubTree {
                owner,
                repo,
                git_ref,
                subpath,
            }))
        }
        "blob" => Err("暂不支持 GitHub 文件链接，请改用仓库、目录或 ZIP 链接".to_string()),
        _ => Ok(Some(DownloadSource::GitHubRepo { owner, repo })),
    }
}

fn sanitize_relative_subpath(raw: &str) -> Result<PathBuf, String> {
    let mut output = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(value) => output.push(value),
            Component::CurDir => {}
            _ => return Err("GitHub 子目录路径无效".to_string()),
        }
    }

    if output.as_os_str().is_empty() {
        return Err("GitHub 子目录路径不能为空".to_string());
    }

    Ok(output)
}

fn is_supported_zip_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .ends_with(".zip")
}

impl DownloadSource {
    fn preferred_subpath(&self) -> Option<PathBuf> {
        match self {
            DownloadSource::GitHubTree { subpath, .. } => Some(subpath.clone()),
            _ => None,
        }
    }
}

struct TempDirGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl<'a> TempDirGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, armed: true }
    }

    #[allow(dead_code)]
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
        let file = zip.by_index(i).map_err(|err| err.to_string())?;
        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let out_path = canonical_extract.join(&enclosed);

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

        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
        std::io::copy(&mut file.take(MAX_FILE_SIZE), &mut outfile).map_err(|err| err.to_string())?;
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

fn find_skill_root(
    extract_dir: &Path,
    expected: &str,
    preferred_subpath: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(preferred) = preferred_subpath {
        if let Some(found) = find_preferred_root(extract_dir, preferred)? {
            return Ok(found);
        }
    }

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

fn find_preferred_root(extract_dir: &Path, preferred_subpath: &Path) -> Result<Option<PathBuf>, String> {
    let direct = extract_dir.join(preferred_subpath);
    if direct.exists() && direct.is_dir() {
        return Ok(Some(direct));
    }

    let entries = fs::read_dir(extract_dir).map_err(|err| err.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let candidate = path.join(preferred_subpath);
        if candidate.exists() && candidate.is_dir() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{find_skill_root, parse_download_source, DownloadSource};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_github_repo_url() {
        let parsed = parse_download_source("https://github.com/owner/repo").unwrap();
        assert_eq!(
            parsed,
            DownloadSource::GitHubRepo {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
            }
        );
    }

    #[test]
    fn parses_github_tree_url() {
        let parsed = parse_download_source("https://github.com/anthropics/skills/tree/main/skills/docx").unwrap();
        assert_eq!(
            parsed,
            DownloadSource::GitHubTree {
                owner: "anthropics".to_string(),
                repo: "skills".to_string(),
                git_ref: "main".to_string(),
                subpath: PathBuf::from("skills/docx"),
            }
        );
    }

    #[test]
    fn parses_zip_url() {
        let parsed = parse_download_source("https://example.com/files/skill-pack.zip?download=1").unwrap();
        assert_eq!(
            parsed,
            DownloadSource::ZipUrl {
                url: "https://example.com/files/skill-pack.zip?download=1".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unsupported_url() {
        let error = parse_download_source("https://example.com/skill-page").unwrap_err();
        assert!(error.contains("仅支持"));
    }

    #[test]
    fn prioritizes_preferred_subpath() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("skills-manager-test-{timestamp}"));
        let extract_dir = temp_root.join("extract");
        let repo_root = extract_dir.join("repo-main");
        let preferred = repo_root.join("skills/docx");
        let fallback = repo_root.join("skills/other-skill");

        fs::create_dir_all(&preferred).unwrap();
        fs::create_dir_all(&fallback).unwrap();
        fs::write(preferred.join("SKILL.md"), "# docx").unwrap();
        fs::write(fallback.join("SKILL.md"), "# other").unwrap();

        let selected = find_skill_root(&extract_dir, "other-skill", Some(PathBuf::from("skills/docx").as_path())).unwrap();
        assert_eq!(selected, preferred);

        let _ = fs::remove_dir_all(temp_root);
    }
}

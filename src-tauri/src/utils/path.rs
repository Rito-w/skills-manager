use std::fs;
use std::path::{Component, Path, PathBuf};

#[cfg(windows)]
fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", rest));
    }
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        return PathBuf::from(rest.to_string());
    }
    path.to_path_buf()
}

#[cfg(not(windows))]
fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    path.to_path_buf()
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let path = strip_windows_verbatim_prefix(path);
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(comp),
        }
    }
    normalized
}

/// Windows reserved names that cannot be used as file/directory names
#[cfg(target_os = "windows")]
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Check if a name is a Windows reserved name
#[cfg(target_os = "windows")]
fn is_windows_reserved_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    // Check exact match
    if WINDOWS_RESERVED_NAMES.contains(&upper.as_str()) {
        return true;
    }
    // Check with extension (e.g., CON.txt, NUL.md)
    if let Some(base) = upper.split('.').next() {
        if WINDOWS_RESERVED_NAMES.contains(&base) {
            return true;
        }
    }
    false
}

pub fn sanitize_dir_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '.' {
            out.push('-');
        }
    }
    let mut result = if out.is_empty() {
        "skill".to_string()
    } else {
        out.trim_matches('-').to_string()
    };

    // Windows reserved names check - prefix with underscore to make it safe
    #[cfg(target_os = "windows")]
    if is_windows_reserved_name(&result) {
        result = format!("_{}", result);
    }

    result
}

fn short_stable_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:08x}")[..8].to_string()
}

pub fn sanitize_skill_dir_name(name: &str, fallback_key: &str) -> String {
    let sanitized = sanitize_dir_name(name);
    if sanitized != "skill" {
        return sanitized;
    }

    if name.trim().eq_ignore_ascii_case("skill") {
        return sanitized;
    }

    let hash_source = if name.trim().is_empty() {
        fallback_key
    } else {
        name
    };
    format!("skill-{}", short_stable_hash(hash_source))
}

pub fn resolve_canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path)
        .ok()
        .map(|canon| normalize_path(&canon))
}

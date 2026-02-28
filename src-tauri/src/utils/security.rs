use std::path::{Component, Path, PathBuf};

pub fn is_safe_relative_dir(rel: &str) -> bool {
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

pub fn is_within_directory(base: &Path, target: &Path) -> bool {
    let canonical_base = base
        .canonicalize()
        .unwrap_or_else(|_| base.to_path_buf());

    let normalized_target = target.components().fold(PathBuf::new(), |mut acc, part| {
        if part == Component::ParentDir {
            acc.pop();
        } else if part != Component::CurDir {
            acc.push(part);
        }
        acc
    });

    let resolved_target = if target.is_absolute() {
        normalized_target
    } else {
        canonical_base.join(normalized_target)
    };

    resolved_target.starts_with(&canonical_base)
}

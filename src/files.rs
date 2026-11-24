//! File discovery and output path resolution.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use glob::{MatchOptions, glob_with};

use crate::registry;

fn candidate_bases(base_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(base_dir) = base_dir {
        let resolved = std::path::absolute(base_dir)
            .or_else(|_| base_dir.canonicalize())
            .unwrap_or_else(|_| base_dir.to_path_buf());
        candidates.push(resolved.clone());
        if let Some(parent) = resolved.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

/// Expand a file, directory, or glob pattern into source files.
pub fn collect_files(root: &str, base_dir: Option<&Path>) -> Vec<PathBuf> {
    let absolute = Path::new(root).is_absolute();
    let search_bases: Vec<Option<PathBuf>> = if absolute {
        vec![None]
    } else {
        candidate_bases(base_dir).into_iter().map(Some).collect()
    };

    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    for base in search_bases {
        let expanded = match &base {
            Some(base) => base.join(root),
            None => PathBuf::from(root),
        };
        let pattern = expanded.to_string_lossy();
        let Ok(matches) = glob_with(&pattern, options) else {
            continue;
        };
        let found: Vec<PathBuf> = matches.filter_map(Result::ok).collect();
        if !found.is_empty() {
            candidates = found;
            break;
        }
    }

    let mut files: HashSet<String> = HashSet::new();
    for candidate in candidates {
        if candidate.is_file() {
            if registry::is_supported(&candidate) {
                files.insert(candidate.to_string_lossy().into_owned());
            }
        } else if candidate.is_dir() {
            collect_directory(&candidate, &mut files);
        }
    }

    let mut out: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    out.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    out
}

fn collect_directory(directory: &Path, files: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_directory(&path, files);
        } else if file_type.is_file() && registry::is_supported(&path) {
            files.insert(path.to_string_lossy().into_owned());
        }
    }
}

/// Choose an output path and resolve it against the config directory.
pub fn resolve_output(value: Option<&Path>, default: &Path, base_dir: &Path) -> PathBuf {
    let chosen = value.unwrap_or(default);
    if chosen.is_absolute() {
        chosen.to_path_buf()
    } else {
        base_dir.join(chosen)
    }
}

/// Resolve the input or config glob into source files.
pub fn select_files(spec: &str, base_dir: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let files = collect_files(spec, base_dir);
    if files.is_empty() {
        anyhow::bail!("No supported source files found for {spec:?}");
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_output_against_base_dir() {
        assert_eq!(
            resolve_output(
                Some(Path::new("out.md")),
                Path::new("default.md"),
                Path::new("/tmp/base")
            ),
            PathBuf::from("/tmp/base/out.md")
        );
    }

    #[test]
    fn keeps_absolute_outputs() {
        assert_eq!(
            resolve_output(
                Some(Path::new("/tmp/elsewhere/out.md")),
                Path::new("default.md"),
                Path::new("/tmp/base")
            ),
            PathBuf::from("/tmp/elsewhere/out.md")
        );
    }

    #[test]
    fn collects_directory_recursively_and_skips_unsupported_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("a.py"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        std::fs::write(dir.path().join("nested/c.rs"), "").unwrap();

        let found = collect_files(dir.path().to_str().unwrap(), None);
        let names: Vec<String> = found
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.py", "c.rs"]);
    }

    #[test]
    fn falls_back_to_parent_base_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(dir.path().join("marker.py"), "").unwrap();

        let found = collect_files("marker.py", Some(&nested));
        assert_eq!(found, vec![dir.path().join("marker.py")]);
    }

    #[test]
    fn expands_globs_in_sorted_order() {
        let found = collect_files("tests/fixtures/animal.*pp", Some(&crate_root()));
        let names: Vec<String> = found
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["animal.cpp", "animal.hpp"]);
    }

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }
}

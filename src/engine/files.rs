//! @brief File discovery and output path resolution.
//! @details Locates supported source files and resolves relative output paths
//! exactly as the Python reference does.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use glob::{MatchOptions, glob_with};

use crate::lang::registry;

/// Match semantics used by `dox.py`: `*` never crosses `/`, and dot files are
/// only matched explicitly.
///
/// @return Match options matching Python-style glob behaviour.
fn match_options() -> MatchOptions {
    MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    }
}

/// @brief Expand a file, directory, or glob pattern into supported source files.
///
/// @details Relative patterns are tried against configured and working
/// directories in contract order. Absolute patterns are searched directly.
pub struct FileFinder<'a> {
    /// Directory used to resolve relative patterns.
    ///
    /// @note `None` disables configured-base candidate lookup.
    base_dir: Option<&'a Path>,
}

impl<'a> FileFinder<'a> {
    /// Create a finder that resolves relative patterns against `base_dir`.
    ///
    /// @param[in] base_dir Directory against which relative patterns are resolved.
    /// @return A finder with the requested base directory.
    pub fn new(base_dir: Option<&'a Path>) -> Self {
        Self { base_dir }
    }

    /// Expand `root` into a sorted, de-duplicated list of supported files.
    ///
    /// @param[in] root File, directory, or glob pattern to collect.
    /// @return Sorted, de-duplicated supported source paths.
    pub fn collect(&self, root: &str) -> Vec<PathBuf> {
        let files: HashSet<String> = self
            .expand(root)
            .into_iter()
            .flat_map(|candidate| self.candidate_files(&candidate))
            .map(|path| path.to_string_lossy().into_owned())
            .collect();

        let mut out: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
        out.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
        out
    }

    /// Expand `root` and fail when nothing supported was found.
    ///
    /// @param[in] root File, directory, or glob pattern to select.
    /// @return The supported paths, or the contract's no-files error.
    /// @throws Any error when the selection contains no supported source file.
    pub fn select(&self, root: &str) -> anyhow::Result<Vec<PathBuf>> {
        let files = self.collect(root);
        if files.is_empty() {
            anyhow::bail!("No supported source files found for {root:?}");
        }
        Ok(files)
    }

    /// Candidate base directories, in the order `dox.py` tries them.
    ///
    /// @return Unique base directories in contract lookup order.
    fn candidate_bases(&self) -> Vec<PathBuf> {
        let configured = self
            .base_dir
            .map(|base_dir| {
                let resolved = std::path::absolute(base_dir)
                    .or_else(|_| base_dir.canonicalize())
                    .unwrap_or_else(|_| base_dir.to_path_buf());
                let mut bases = vec![resolved.clone()];
                bases.extend(resolved.parent().map(Path::to_path_buf));
                bases
            })
            .unwrap_or_default();

        let cwd = std::env::current_dir().ok();
        configured
            .into_iter()
            .chain(cwd)
            .fold(Vec::new(), |mut unique, candidate| {
                if !unique.contains(&candidate) {
                    unique.push(candidate);
                }
                unique
            })
    }

    /// Pair each base directory with the absolute/relative form to glob.
    ///
    /// @param[in] absolute Whether `root` is an absolute glob pattern.
    /// @return Search bases to try, or no base for absolute patterns.
    fn search_bases(&self, absolute: bool) -> Vec<Option<PathBuf>> {
        if absolute {
            vec![None]
        } else {
            self.candidate_bases().into_iter().map(Some).collect()
        }
    }

    /// Glob `root` from the first base directory that produces matches.
    ///
    /// @param[in] root File, directory, or glob pattern.
    /// @return All matches from the first candidate base with matches.
    fn expand(&self, root: &str) -> Vec<PathBuf> {
        let options = match_options();
        self.search_bases(Path::new(root).is_absolute())
            .into_iter()
            .map(|base| base.map_or_else(|| PathBuf::from(root), |base| base.join(root)))
            .filter_map(|candidate| {
                let pattern = candidate.to_string_lossy().into_owned();
                glob_with(&pattern, options)
                    .ok()
                    .map(|matches| matches.filter_map(Result::ok).collect::<Vec<_>>())
                    .filter(|found| !found.is_empty())
            })
            .next()
            .unwrap_or_default()
    }

    /// Turn one glob hit into the files it represents.
    ///
    /// @param[in] candidate A file or directory produced by glob expansion.
    /// @return The candidate itself or recursively discovered source files.
    fn candidate_files(&self, candidate: &Path) -> Vec<PathBuf> {
        if candidate.is_file() {
            return registry::REGISTRY
                .is_supported(candidate)
                .then(|| candidate.to_path_buf())
                .into_iter()
                .collect();
        }
        if candidate.is_dir() {
            return self.directory_files(candidate);
        }
        Vec::new()
    }

    /// Recursively collect supported files under `directory`.
    ///
    /// @param[in] directory Directory to walk recursively.
    /// @return Supported files found below the directory.
    fn directory_files(&self, directory: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(directory)
            .map(|entries| entries.flatten().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .flat_map(|entry| {
                let path = entry.path();
                match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => self.directory_files(&path),
                    Ok(file_type)
                        if file_type.is_file() && registry::REGISTRY.is_supported(&path) =>
                    {
                        vec![path]
                    }
                    _ => Vec::new(),
                }
            })
            .collect()
    }
}

/// @brief Resolve an output path against the configuration directory.
///
/// @details Absolute paths remain unchanged; relative paths are joined to the
/// configured base directory.
pub struct OutputResolver<'a> {
    /// Directory that relative outputs are joined to.
    ///
    /// @note This is normally the directory containing `ini.cfg`.
    base_dir: &'a Path,
}

impl<'a> OutputResolver<'a> {
    /// Create a resolver rooted at `base_dir`.
    ///
    /// @param[in] base_dir Directory used for relative output paths.
    /// @return An output resolver rooted at `base_dir`.
    pub fn new(base_dir: &'a Path) -> Self {
        Self { base_dir }
    }

    /// Resolve `value`, falling back to `default`, against the base directory.
    ///
    /// @param[in] value Explicit output path, if supplied.
    /// @param[in] default Output path used when `value` is absent.
    /// @return The absolute or base-relative output path.
    pub fn resolve(&self, value: Option<&Path>, default: &Path) -> PathBuf {
        let chosen = value.unwrap_or(default);
        if chosen.is_absolute() {
            chosen.to_path_buf()
        } else {
            self.base_dir.join(chosen)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_output_against_base_dir() {
        assert_eq!(
            OutputResolver::new(Path::new("/tmp/base"))
                .resolve(Some(Path::new("out.md")), Path::new("default.md")),
            PathBuf::from("/tmp/base/out.md")
        );
    }

    #[test]
    fn keeps_absolute_outputs() {
        assert_eq!(
            OutputResolver::new(Path::new("/tmp/base")).resolve(
                Some(Path::new("/tmp/elsewhere/out.md")),
                Path::new("default.md")
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

        let found = FileFinder::new(None).collect(dir.path().to_str().unwrap());
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

        let found = FileFinder::new(Some(&nested)).collect("marker.py");
        assert_eq!(found, vec![dir.path().join("marker.py")]);
    }

    #[test]
    fn expands_globs_in_sorted_order() {
        let found = FileFinder::new(Some(&crate_root())).collect("tests/fixtures/animal.*pp");
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

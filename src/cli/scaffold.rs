//! Create a starter `ini.cfg` in a target directory.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::engine::config::{CONFIG_FILENAME, DEFAULT_CONFIG_TEXT};

/// Writes the default configuration file, optionally overwriting one.
pub struct Scaffolder {
    /// Directory that will contain `ini.cfg`.
    target: PathBuf,
    /// Whether an existing file may be overwritten.
    force: bool,
}

impl Scaffolder {
    /// Create a scaffolder for `target`; `force` allows overwriting.
    pub fn new(target: impl Into<PathBuf>, force: bool) -> Self {
        Self {
            target: target.into(),
            force,
        }
    }

    /// Write the default configuration file and report the written paths.
    pub fn run(&self) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(&self.target)
            .with_context(|| format!("Failed to create {}", self.target.display()))?;

        let path = self.config_path();
        if path.exists() && !self.force {
            bail!("Refusing to overwrite {} (use --force).", path.display());
        }

        std::fs::write(&path, DEFAULT_CONFIG_TEXT)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(vec![path])
    }

    /// Return the target directory this scaffolder was created for.
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// `Path(".") / "ini.cfg"` in Python is just `Path("ini.cfg")`.
    fn config_path(&self) -> PathBuf {
        normalize(&self.target).join(CONFIG_FILENAME)
    }
}

/// Approximate `pathlib.Path`'s normalization: drop `.` components.
fn normalize(path: &Path) -> PathBuf {
    path.components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| component.as_os_str().to_owned())
        .collect()
}

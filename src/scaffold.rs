//! Create a starter `ini.cfg` in a target directory.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::{CONFIG_FILENAME, DEFAULT_CONFIG_TEXT};

pub struct Scaffolder {
    target: PathBuf,
    force: bool,
}

impl Scaffolder {
    pub fn new(target: impl Into<PathBuf>, force: bool) -> Self {
        Self {
            target: target.into(),
            force,
        }
    }

    /// Write the default configuration file.
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

    /// `Path(".") / "ini.cfg"` in Python is just `Path("ini.cfg")`.
    fn config_path(&self) -> PathBuf {
        normalize(&self.target).join(CONFIG_FILENAME)
    }

    pub fn target(&self) -> &Path {
        &self.target
    }
}

/// Approximate `pathlib.Path`'s normalization: drop `.` and empty components.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

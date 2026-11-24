//! INI configuration handling.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::doc::split_lines;

pub const CONFIG_FILENAME: &str = "ini.cfg";

pub const DEFAULT_CONFIG_TEXT: &str = "\
[defaults]
glob     = *
output   = README.md
diagram  = DIAGRAM.md
encoding = utf-8
";

static SECTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\s*$").unwrap());
static KEY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*=\s*(.*)$").unwrap());

/// Settings that control which files are read and where output is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub glob: String,
    pub output: PathBuf,
    pub diagram: PathBuf,
    pub encoding: String,
    pub base_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            glob: "*".to_owned(),
            output: PathBuf::from("README.md"),
            diagram: PathBuf::from("DIAGRAM.md"),
            encoding: "utf-8".to_owned(),
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl Config {
    /// Build the built-in default configuration.
    pub fn defaults() -> Self {
        parse_config(DEFAULT_CONFIG_TEXT, None)
    }

    /// Load configuration from an explicit path or the current directory.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            if !path.is_file() {
                bail!("Config file not found: {}", path.display());
            }
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            return Ok(parse_config(&text, path.parent()));
        }

        let candidate = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(CONFIG_FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)
                .with_context(|| format!("Failed to read {}", candidate.display()))?;
            return Ok(parse_config(&text, candidate.parent()));
        }

        Ok(Self::defaults())
    }
}

/// Parse INI-style configuration text.
pub fn parse_config(text: &str, base_dir: Option<&Path>) -> Config {
    let mut config = Config::default();
    if let Some(base_dir) = base_dir {
        config.base_dir = base_dir.to_path_buf();
    }

    let mut section: Option<String> = None;
    for raw in split_lines(text) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(caps) = SECTION_RE.captures(line) {
            section = Some(caps[1].to_lowercase());
            continue;
        }

        let Some(caps) = KEY_RE.captures(line) else {
            continue;
        };
        if section.as_deref() != Some("defaults") {
            continue;
        }

        let key = caps[1].to_lowercase();
        let value = caps[2].trim();
        match key.as_str() {
            "glob" => config.glob = value.to_owned(),
            "output" => config.output = python_path(value),
            "diagram" => config.diagram = python_path(value),
            "encoding" => {
                config.encoding = if value.is_empty() {
                    "utf-8".to_owned()
                } else {
                    value.to_owned()
                };
            }
            _ => {}
        }
    }

    config
}

/// `Path("")` in Python is `Path(".")`.
pub fn python_path(value: &str) -> PathBuf {
    if value.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let config = Config::defaults();
        assert_eq!(config.glob, "*");
        assert_eq!(config.output, PathBuf::from("README.md"));
        assert_eq!(config.diagram, PathBuf::from("DIAGRAM.md"));
        assert_eq!(config.encoding, "utf-8");
    }

    #[test]
    fn ignores_other_sections_and_comments() {
        let config = parse_config(
            "; comment\n[other]\nglob = no\n[defaults]\n# comment\nGLOB = src/**/*.rs\nencoding =\n",
            Some(Path::new("/tmp")),
        );
        assert_eq!(config.glob, "src/**/*.rs");
        assert_eq!(config.encoding, "utf-8");
        assert_eq!(config.base_dir, PathBuf::from("/tmp"));
    }
}

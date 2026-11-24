//! INI configuration handling.
//!
//! Only the `[defaults]` section is interpreted, matching `dox.py`. Unknown
//! sections, keys, comments, and malformed lines are ignored.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::documentation::split_lines;

/// File name discovered in the current directory when `-c` is absent.
pub const CONFIG_FILENAME: &str = "ini.cfg";

/// Configuration text written by `dozen init`.
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
    /// Input file, directory, or glob pattern.
    pub glob: String,
    /// Default Markdown reference output path.
    pub output: PathBuf,
    /// Default Mermaid diagram output path.
    pub diagram: PathBuf,
    /// Character encoding used for both reading and writing.
    pub encoding: String,
    /// Directory used to resolve relative globs and output paths.
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
        Self::parse(DEFAULT_CONFIG_TEXT, None)
    }

    /// Load configuration from an explicit path or the current directory.
    ///
    /// An explicit path must exist. Without one, `./ini.cfg` is used when
    /// present; otherwise the built-in defaults apply.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            if !path.is_file() {
                bail!("Config file not found: {}", path.display());
            }
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            return Ok(Self::parse(&text, path.parent()));
        }

        let candidate = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(CONFIG_FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)
                .with_context(|| format!("Failed to read {}", candidate.display()))?;
            return Ok(Self::parse(&text, candidate.parent()));
        }

        Ok(Self::defaults())
    }

    /// Parse INI-style configuration text.
    pub fn parse(text: &str, base_dir: Option<&Path>) -> Self {
        let mut builder = ConfigBuilder::new(base_dir);
        split_lines(text)
            .iter()
            .for_each(|line| builder.apply(line));
        builder.finish()
    }
}

/// Mutable state while scanning a configuration file.
struct ConfigBuilder {
    /// Configuration assembled so far.
    config: Config,
    /// Current section name, lowercased.
    section: Option<String>,
}

impl ConfigBuilder {
    /// Create a builder seeded with the built-in defaults.
    fn new(base_dir: Option<&Path>) -> Self {
        let mut config = Config::default();
        if let Some(base_dir) = base_dir {
            config.base_dir = base_dir.to_path_buf();
        }
        Self {
            config,
            section: None,
        }
    }

    /// Apply one raw line: a section header, a key/value pair, or noise.
    fn apply(&mut self, raw: &str) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            return;
        }

        if let Some(captures) = SECTION_RE.captures(line) {
            self.section = Some(captures[1].to_lowercase());
            return;
        }

        let Some(captures) = KEY_RE.captures(line) else {
            return;
        };
        if self.section.as_deref() != Some("defaults") {
            return;
        }

        self.set(&captures[1].to_lowercase(), captures[2].trim());
    }

    /// Assign one recognised key in the active `[defaults]` section.
    fn set(&mut self, key: &str, value: &str) {
        match key {
            "glob" => self.config.glob = value.to_owned(),
            "output" => self.config.output = python_path(value),
            "diagram" => self.config.diagram = python_path(value),
            "encoding" => {
                self.config.encoding = if value.is_empty() {
                    "utf-8".to_owned()
                } else {
                    value.to_owned()
                };
            }
            _ => {}
        }
    }

    /// Finish scanning and return the populated configuration.
    fn finish(self) -> Config {
        self.config
    }
}

/// `Path("")` in Python is `Path(".")`.
fn python_path(value: &str) -> PathBuf {
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
        let config = Config::parse(
            "; comment\n[other]\nglob = no\n[defaults]\n# comment\nGLOB = src/**/*.rs\nencoding =\n",
            Some(Path::new("/tmp")),
        );
        assert_eq!(config.glob, "src/**/*.rs");
        assert_eq!(config.encoding, "utf-8");
        assert_eq!(config.base_dir, PathBuf::from("/tmp"));
    }
}

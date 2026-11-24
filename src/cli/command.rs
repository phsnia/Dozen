//! @brief CLI command interface and subcommand registry.
//! @details Implementations live in [`crate::cli::commands`] and expose only
//! argument metadata and execution through this trait.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::parser::{Field, Parser};
use crate::engine::config::Config;
use crate::renderer::mermaid::KINDS;

use super::{PROG, print_help};

/// @brief Shared state available to every subcommand implementation.
pub struct Context {
    /// Configuration path supplied before the subcommand, if any.
    global_config: Option<PathBuf>,
}

impl Context {
    /// @brief Create a context carrying a global configuration path.
    pub fn new(global_config: Option<PathBuf>) -> Self {
        Self { global_config }
    }

    /// @brief Load configuration selected by subcommand or global options.
    pub fn load_config(&self, parsed: &Parser) -> Result<Config> {
        let path = parsed
            .opt("config")
            .map(PathBuf::from)
            .or_else(|| self.global_config.clone());
        Config::load(path.as_deref())
    }

    /// @brief Return the positional input or the configured glob.
    pub fn input_spec<'a>(&self, parsed: &'a Parser, config: &'a Config) -> &'a str {
        parsed
            .positional
            .first()
            .map(String::as_str)
            .unwrap_or(&config.glob)
    }

    /// @brief Resolve an output path against the configuration directory.
    pub fn output_path(&self, parsed: &Parser, default: &Path, base_dir: &Path) -> PathBuf {
        crate::engine::files::OutputResolver::new(base_dir)
            .resolve(parsed.opt("output").map(Path::new), default)
    }

    /// @brief Create missing output parent directories.
    pub fn prepare_output(&self, output: &Path) -> Result<()> {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        Ok(())
    }
}

/// @brief Interface implemented by every CLI subcommand.
pub trait Command: Sync {
    fn name(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn help(&self) -> &'static str;
    fn fields(&self) -> &'static [Field];
    fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode>;

    fn parse(&self, rest: &[String]) -> Result<Parser> {
        parse_or_exit(rest, self.fields(), self.usage())
    }

    fn error_program(&self) -> String {
        format!("{PROG} {}", self.name())
    }

    fn print_help(&self) {
        print_help(self.help());
    }
}

static CONVERT: crate::cli::commands::ConvertCommand = crate::cli::commands::ConvertCommand;
static DIAGRAM: crate::cli::commands::DiagramCommand = crate::cli::commands::DiagramCommand;
static INIT: crate::cli::commands::InitCommand = crate::cli::commands::InitCommand;

static COMMANDS: &[&dyn Command] = &[&CONVERT, &DIAGRAM, &INIT];

/// @brief Return every registered command in help-text order.
pub fn all() -> &'static [&'static dyn Command] {
    COMMANDS
}

/// @brief Find a registered command by its command-line name.
pub fn find(name: &str) -> Option<&'static dyn Command> {
    COMMANDS
        .iter()
        .copied()
        .find(|command| command.name() == name)
}

/// @brief Render the argparse choice list for `--kind`.
pub(crate) fn kind_choices() -> String {
    KINDS
        .iter()
        .copied()
        .chain(std::iter::once("all"))
        .map(|choice| format!("'{choice}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// @brief Parse arguments or terminate with argparse-style diagnostics.
pub(crate) fn parse_or_exit(rest: &[String], fields: &[Field], usage: &str) -> Result<Parser> {
    match Parser::parse(rest.iter().cloned(), fields) {
        Ok(parsed) => Ok(parsed),
        Err(message) => {
            eprintln!("{usage}");
            eprintln!("{PROG}: error: {message}");
            std::process::exit(2);
        }
    }
}

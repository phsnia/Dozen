//! Command objects for the three CLI subcommands.
//!
//! Each command owns its usage text, option table, and execution strategy, so
//! `cli::mod` only has to locate the subcommand and hand it the remaining
//! arguments.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::parser::{Field, Parser};
use crate::cli::scaffold::Scaffolder;
use crate::engine::config::Config;
use crate::engine::converter::Converter;
use crate::engine::files::FileFinder;
use crate::renderer::mermaid::KINDS;

use super::{PROG, print_help, usage_error};

const CONVERT_USAGE: &str = "usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]";

const CONVERT_HELP: &str = "\
usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]

positional arguments:
  input

options:
  -h, --help           show this help message and exit
  -o, --output OUTPUT
  -c, --config CONFIG";

const DIAGRAM_USAGE: &str = "usage: doxygen2md diagram [-h] [-o OUTPUT]\n                          [-k {class,inheritance,collaboration,call,caller,include,directory,all}]\n                          [-c CONFIG]\n                          [input]";

const DIAGRAM_HELP: &str = "\
usage: doxygen2md diagram [-h] [-o OUTPUT]
                          [-k {class,inheritance,collaboration,call,caller,include,directory,all}]
                          [-c CONFIG]
                          [input]

positional arguments:
  input

options:
  -h, --help            show this help message and exit
  -o, --output OUTPUT
  -k, --kind {class,inheritance,collaboration,call,caller,include,directory,all}
  -c, --config CONFIG";

const INIT_USAGE: &str = "usage: doxygen2md init [-h] [--force] [target]";

const INIT_HELP: &str = "\
usage: doxygen2md init [-h] [--force] [target]

positional arguments:
  target

options:
  -h, --help  show this help message and exit
  --force";

const CONVERT_FIELDS: &[Field] = &[
    Field::option("output", Some('o')),
    Field::option("config", Some('c')),
    Field::positional("input"),
];

const DIAGRAM_FIELDS: &[Field] = &[
    Field::option("output", Some('o')),
    Field::option("kind", Some('k')),
    Field::option("config", Some('c')),
    Field::positional("input"),
];

const INIT_FIELDS: &[Field] = &[Field::flag("force"), Field::positional("target")];

/// Shared state and helpers available to every subcommand.
pub struct Context {
    /// Configuration path supplied before the subcommand, if any.
    global_config: Option<PathBuf>,
}

impl Context {
    /// Create a context carrying the config supplied before the subcommand.
    pub fn new(global_config: Option<PathBuf>) -> Self {
        Self { global_config }
    }

    /// Load the effective configuration for `parsed`.
    pub fn load_config(&self, parsed: &Parser) -> Result<Config> {
        let path = parsed
            .opt("config")
            .map(PathBuf::from)
            .or_else(|| self.global_config.clone());
        Config::load(path.as_deref())
    }

    /// Return the positional input, falling back to the configured glob.
    pub fn input_spec<'a>(&self, parsed: &'a Parser, config: &'a Config) -> &'a str {
        parsed
            .positional
            .first()
            .map(String::as_str)
            .unwrap_or(&config.glob)
    }

    /// Resolve the requested output against the configuration directory.
    pub fn output_path(&self, parsed: &Parser, default: &Path, base_dir: &Path) -> PathBuf {
        let resolver = crate::engine::files::OutputResolver::new(base_dir);
        resolver.resolve(parsed.opt("output").map(Path::new), default)
    }

    /// Create the parent directory of `output` when it is missing.
    pub fn prepare_output(&self, output: &Path) -> Result<()> {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        Ok(())
    }
}

/// A CLI subcommand with its own argument table and execution strategy.
pub trait Command: Sync {
    /// Command name as typed on the command line.
    fn name(&self) -> &'static str;

    /// One-line usage string printed on argument errors.
    fn usage(&self) -> &'static str;

    /// Full help text printed for `-h` and `--help`.
    fn help(&self) -> &'static str;

    /// Argument specification for this command.
    fn fields(&self) -> &'static [Field];

    /// Run the command with the arguments that follow the subcommand name.
    fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode>;

    /// Parse `rest` or exit with argparse-compatible diagnostics.
    fn parse(&self, rest: &[String]) -> Result<Parser> {
        parse_or_exit(rest, self.fields(), self.usage())
    }

    /// Program name used in `argparse` error messages.
    fn error_program(&self) -> String {
        format!("{PROG} {}", self.name())
    }

    /// Print this command's help text.
    fn print_help(&self) {
        print_help(self.help());
    }
}

/// `dozen convert` produces a Markdown API reference.
pub struct ConvertCommand;

impl Command for ConvertCommand {
    /// `convert` is the injected default subcommand.
    fn name(&self) -> &'static str {
        "convert"
    }

    /// Usage line shown when argument parsing fails.
    fn usage(&self) -> &'static str {
        CONVERT_USAGE
    }

    /// Full convert help text.
    fn help(&self) -> &'static str {
        CONVERT_HELP
    }

    /// Convert accepts output, config, and input.
    fn fields(&self) -> &'static [Field] {
        CONVERT_FIELDS
    }

    /// Render the selected files into the resolved Markdown output.
    fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode> {
        let parsed = self.parse(rest)?;
        let config = context.load_config(&parsed)?;
        let spec = context.input_spec(&parsed, &config);
        let files = FileFinder::new(Some(&config.base_dir)).select(spec)?;
        let output = context.output_path(&parsed, &config.output, &config.base_dir);

        context.prepare_output(&output)?;
        Converter::new(config).files(&files, &output)?;
        println!("Generated: {}", output.display());
        Ok(ExitCode::SUCCESS)
    }
}

/// `dozen diagram` produces one or more Mermaid graphs.
pub struct DiagramCommand;

impl Command for DiagramCommand {
    /// `diagram` produces Mermaid graphs.
    fn name(&self) -> &'static str {
        "diagram"
    }

    /// Usage line shown when argument parsing fails.
    fn usage(&self) -> &'static str {
        DIAGRAM_USAGE
    }

    /// Full diagram help text.
    fn help(&self) -> &'static str {
        DIAGRAM_HELP
    }

    /// Diagram accepts output, kind, config, and input.
    fn fields(&self) -> &'static [Field] {
        DIAGRAM_FIELDS
    }

    /// Render the selected files into the resolved diagram output.
    fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode> {
        let parsed = self.parse(rest)?;
        let config = context.load_config(&parsed)?;
        let kind = parsed.opt("kind").unwrap_or("class");

        if kind != "all" && !KINDS.contains(&kind) {
            return Ok(usage_error(
                self.usage(),
                &self.error_program(),
                &format!(
                    "argument -k/--kind: invalid choice: '{kind}' (choose from {})",
                    kind_choices()
                ),
            ));
        }

        let spec = context.input_spec(&parsed, &config);
        let files = FileFinder::new(Some(&config.base_dir)).select(spec)?;
        let output = context.output_path(&parsed, &config.diagram, &config.base_dir);

        context.prepare_output(&output)?;
        Converter::new(config).diagram(&files, &output, kind)?;
        println!("Generated: {}", output.display());
        Ok(ExitCode::SUCCESS)
    }
}

/// `dozen init` writes a starter `ini.cfg`.
pub struct InitCommand;

impl Command for InitCommand {
    /// `init` scaffolds a configuration file.
    fn name(&self) -> &'static str {
        "init"
    }

    /// Usage line shown when argument parsing fails.
    fn usage(&self) -> &'static str {
        INIT_USAGE
    }

    /// Full init help text.
    fn help(&self) -> &'static str {
        INIT_HELP
    }

    /// Init accepts a force flag and a target directory.
    fn fields(&self) -> &'static [Field] {
        INIT_FIELDS
    }

    /// Write the starter configuration and report each written path.
    fn run(&self, rest: &[String], _context: &Context) -> Result<ExitCode> {
        let parsed = self.parse(rest)?;
        let target = parsed
            .positional
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        for path in Scaffolder::new(target, parsed.flag("force")).run()? {
            println!("Wrote: {}", path.display());
        }
        Ok(ExitCode::SUCCESS)
    }
}

static CONVERT: ConvertCommand = ConvertCommand;
static DIAGRAM: DiagramCommand = DiagramCommand;
static INIT: InitCommand = InitCommand;

static COMMANDS: &[&dyn Command] = &[&CONVERT, &DIAGRAM, &INIT];

/// Return every registered command, in help-text order.
pub fn all() -> &'static [&'static dyn Command] {
    COMMANDS
}

/// Find a command by its command-line name.
pub fn find(name: &str) -> Option<&'static dyn Command> {
    COMMANDS
        .iter()
        .copied()
        .find(|command| command.name() == name)
}

/// Render the `argparse` choice list for `-k`.
fn kind_choices() -> String {
    KINDS
        .iter()
        .copied()
        .chain(std::iter::once("all"))
        .map(|choice| format!("'{choice}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse arguments or terminate with an argparse-style usage error.
fn parse_or_exit(rest: &[String], fields: &[Field], usage: &str) -> Result<Parser> {
    match Parser::parse(rest.iter().cloned(), fields) {
        Ok(parsed) => Ok(parsed),
        Err(message) => {
            eprintln!("{usage}");
            eprintln!("{PROG}: error: {message}");
            std::process::exit(2);
        }
    }
}

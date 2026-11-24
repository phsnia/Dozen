//! Command-line interface mirroring `dox.py`.
//!
//! `dox.py` delegates argument handling to `argparse` with the program name
//! `doxygen2md`. The help strings below reproduce that output verbatim so the
//! Rust port stays a drop-in replacement.

pub mod command;
pub mod parser;
pub mod scaffold;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};

const VERSION: &str = "0.5.0";

const PROG: &str = "doxygen2md";

const ROOT_USAGE: &str =
    "usage: doxygen2md [-h] [--version] [-c CONFIG] {convert,diagram,init} ...";

const ROOT_HELP: &str = "\
usage: doxygen2md [-h] [--version] [-c CONFIG] {convert,diagram,init} ...

positional arguments:
  {convert,diagram,init}

options:
  -h, --help            show this help message and exit
  --version             show program's version number and exit
  -c, --config CONFIG";

/// Run the CLI using the process arguments.
pub fn run() -> Result<ExitCode> {
    Cli::from_env().execute()
}

/// Parsed process arguments plus the argparse-compatible dispatch rules.
pub struct Cli {
    /// Process arguments without the executable name.
    args: Vec<String>,
}

impl Cli {
    /// Collect the process arguments, skipping the executable name.
    pub fn from_env() -> Self {
        Self {
            args: std::env::args().skip(1).collect(),
        }
    }

    /// Execute the command line and return the process exit code.
    pub fn execute(&self) -> Result<ExitCode> {
        if self.args.is_empty() {
            print_help(ROOT_HELP);
            return Ok(ExitCode::SUCCESS);
        }

        let Some(index) = self.command_index() else {
            return self.execute_default();
        };

        match self.parse_global(&self.args[..index])? {
            GlobalOutcome::Stop(code) => Ok(code),
            GlobalOutcome::Continue(global_config) => {
                let (command, rest) = dispatch(&self.args[index..])?;
                run_command(command, rest, global_config)
            }
        }
    }

    /// Position of the first recognised subcommand.
    fn command_index(&self) -> Option<usize> {
        self.args
            .iter()
            .position(|arg| command::find(arg).is_some())
    }

    /// Apply argparse's injected default `convert` subcommand.
    fn execute_default(&self) -> Result<ExitCode> {
        if self.args.iter().any(|arg| is_help(arg)) {
            print_help(ROOT_HELP);
            return Ok(ExitCode::SUCCESS);
        }
        if self.args.iter().any(|arg| arg == "--version") {
            print_version();
            return Ok(ExitCode::SUCCESS);
        }

        let rewritten: Vec<String> = std::iter::once("convert".to_owned())
            .chain(self.args.iter().cloned())
            .collect();
        let (command, rest) = dispatch(&rewritten)?;
        run_command(command, rest, None)
    }

    /// Parse the global options that appear before the subcommand.
    fn parse_global(&self, args: &[String]) -> Result<GlobalOutcome> {
        let mut global_config: Option<PathBuf> = None;
        let mut cursor = 0;

        while cursor < args.len() {
            let arg = &args[cursor];
            match arg.as_str() {
                "-h" | "--help" => {
                    print_help(ROOT_HELP);
                    return Ok(GlobalOutcome::Stop(ExitCode::SUCCESS));
                }
                "--version" => {
                    print_version();
                    return Ok(GlobalOutcome::Stop(ExitCode::SUCCESS));
                }
                "-c" | "--config" => {
                    let Some(value) = args.get(cursor + 1) else {
                        return Ok(GlobalOutcome::Stop(usage_error(
                            ROOT_USAGE,
                            PROG,
                            "argument -c/--config: expected one argument",
                        )));
                    };
                    global_config = Some(PathBuf::from(value));
                    cursor += 2;
                }
                _ if arg.starts_with("--config=") => {
                    global_config = Some(PathBuf::from(&arg["--config=".len()..]));
                    cursor += 1;
                }
                _ if arg.starts_with("-c") && arg.len() > 2 => {
                    global_config = Some(PathBuf::from(&arg[2..]));
                    cursor += 1;
                }
                _ => {
                    return Ok(GlobalOutcome::Stop(usage_error(
                        ROOT_USAGE,
                        PROG,
                        &format!("unrecognized arguments: {arg}"),
                    )));
                }
            }
        }

        Ok(GlobalOutcome::Continue(global_config))
    }
}

/// Result of parsing the global option prefix.
enum GlobalOutcome {
    /// Continue with an optional global config path.
    Continue(Option<PathBuf>),
    /// Help, version, or a usage error already printed.
    Stop(ExitCode),
}

/// Resolve the subcommand at the front of `args`.
fn dispatch(args: &[String]) -> Result<(&'static dyn command::Command, &[String])> {
    let Some(first) = args.first() else {
        return Err(anyhow!("missing command"));
    };
    let command = command::find(first).ok_or_else(|| anyhow!("unrecognized command: {first}"))?;
    Ok((command, &args[1..]))
}

/// Print help for a subcommand, or run it with the shared context.
fn run_command(
    command: &dyn command::Command,
    rest: &[String],
    global_config: Option<PathBuf>,
) -> Result<ExitCode> {
    if rest.iter().any(|arg| is_help(arg)) {
        command.print_help();
        return Ok(ExitCode::SUCCESS);
    }

    command.run(rest, &command::Context::new(global_config))
}

/// True for the two spellings of `argparse`'s help flag.
fn is_help(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

/// Print text to stdout, matching `argparse`'s help output stream.
fn print_help(text: &str) {
    println!("{text}");
}

/// Print the `--version` line.
fn print_version() {
    println!("{PROG} {VERSION}");
}

/// Print an argparse-style usage error and return exit code 2.
fn usage_error(usage: &str, program: &str, message: &str) -> ExitCode {
    eprintln!("{usage}");
    eprintln!("{program}: error: {message}");
    ExitCode::from(2)
}

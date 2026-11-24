//! @brief Implementation and argument metadata for `init`.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::command::{Command, Context};
use crate::cli::parser::Field;
use crate::cli::scaffold::Scaffolder;

const USAGE: &str = "usage: doxygen2md init [-h] [--force] [target]";

const HELP: &str = "\
usage: doxygen2md init [-h] [--force] [target]

positional arguments:
  target

options:
  -h, --help  show this help message and exit
  --force";

const FIELDS: &[Field] = &[Field::flag("force"), Field::positional("target")];

/// @brief `dozen init` writes a starter `ini.cfg`.
pub struct InitCommand;

impl Command for InitCommand {
    fn name(&self) -> &'static str {
        "init"
    }

    fn usage(&self) -> &'static str {
        USAGE
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn fields(&self) -> &'static [Field] {
        FIELDS
    }

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

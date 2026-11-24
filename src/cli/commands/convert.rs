//! @brief Implementation and argument metadata for `convert`.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::command::{Command, Context};
use crate::cli::parser::Field;
use crate::engine::converter::Converter;
use crate::engine::files::FileFinder;

const USAGE: &str = "usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]";

const HELP: &str = "\
usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]

positional arguments:
  input

options:
  -h, --help           show this help message and exit
  -o, --output OUTPUT
  -c, --config CONFIG";

const FIELDS: &[Field] = &[
    Field::option("output", Some('o')),
    Field::option("config", Some('c')),
    Field::positional("input"),
];

/// @brief `dozen convert` produces a Markdown API reference.
pub struct ConvertCommand;

impl Command for ConvertCommand {
    fn name(&self) -> &'static str {
        "convert"
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

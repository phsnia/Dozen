//! @brief Implementation and argument metadata for `diagram`.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::command::{Command, Context, kind_choices};
use crate::cli::parser::Field;
use crate::engine::converter::Converter;
use crate::engine::files::FileFinder;
use crate::renderer::mermaid::KINDS;

use super::super::usage_error;

const USAGE: &str = "usage: doxygen2md diagram [-h] [-o OUTPUT]\n                          [-k {class,inheritance,collaboration,call,caller,include,directory,all}]\n                          [-c CONFIG]\n                          [input]";

const HELP: &str = "\
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

const FIELDS: &[Field] = &[
    Field::option("output", Some('o')),
    Field::option("kind", Some('k')),
    Field::option("config", Some('c')),
    Field::positional("input"),
];

/// @brief `dozen diagram` produces one or more Mermaid graphs.
pub struct DiagramCommand;

impl Command for DiagramCommand {
    fn name(&self) -> &'static str {
        "diagram"
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

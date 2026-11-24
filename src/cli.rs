//! Command-line interface mirroring `dox.py`.
//!
//! `dox.py` delegates argument handling to `argparse` with the program name
//! `doxygen2md`. The help strings below reproduce that output verbatim so the
//! Rust port stays a drop-in replacement.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::config::Config;
use crate::converter::Converter;
use crate::files::{resolve_output, select_files};
use crate::parser::{Field, Kind, Parser};
use crate::renderer::mermaid::KINDS;
use crate::scaffold::Scaffolder;

const VERSION: &str = "0.5.0";

const PROG: &str = "doxygen2md";

const ROOT_USAGE: &str =
    "usage: doxygen2md [-h] [--version] [-c CONFIG] {convert,diagram,init} ...";

const CONVERT_USAGE: &str = "usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]";

const DIAGRAM_USAGE: &str = "usage: doxygen2md diagram [-h] [-o OUTPUT]\n                          [-k {class,inheritance,collaboration,call,caller,include,directory,all}]\n                          [-c CONFIG]\n                          [input]";

const INIT_USAGE: &str = "usage: doxygen2md init [-h] [--force] [target]";

const ROOT_HELP: &str = "\
usage: doxygen2md [-h] [--version] [-c CONFIG] {convert,diagram,init} ...

positional arguments:
  {convert,diagram,init}

options:
  -h, --help            show this help message and exit
  --version             show program's version number and exit
  -c, --config CONFIG";

const CONVERT_HELP: &str = "\
usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]

positional arguments:
  input

options:
  -h, --help           show this help message and exit
  -o, --output OUTPUT
  -c, --config CONFIG";

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

const INIT_HELP: &str = "\
usage: doxygen2md init [-h] [--force] [target]

positional arguments:
  target

options:
  -h, --help  show this help message and exit
  --force";

const CONVERT_FIELDS: &[Field] = &[
    Field {
        long: "output",
        short: Some('o'),
        kind: Kind::Opt,
    },
    Field {
        long: "config",
        short: Some('c'),
        kind: Kind::Opt,
    },
    Field {
        long: "input",
        short: None,
        kind: Kind::Pos,
    },
];

const DIAGRAM_FIELDS: &[Field] = &[
    Field {
        long: "output",
        short: Some('o'),
        kind: Kind::Opt,
    },
    Field {
        long: "kind",
        short: Some('k'),
        kind: Kind::Opt,
    },
    Field {
        long: "config",
        short: Some('c'),
        kind: Kind::Opt,
    },
    Field {
        long: "input",
        short: None,
        kind: Kind::Pos,
    },
];

const INIT_FIELDS: &[Field] = &[
    Field {
        long: "force",
        short: None,
        kind: Kind::Flag,
    },
    Field {
        long: "target",
        short: None,
        kind: Kind::Pos,
    },
];

pub fn run() -> Result<ExitCode> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run_args(&args)
}

fn run_args(args: &[String]) -> Result<ExitCode> {
    if args.is_empty() {
        print_help(ROOT_HELP);
        return Ok(ExitCode::SUCCESS);
    }

    let Some(index) = args.iter().position(|arg| is_command(arg)) else {
        if args.iter().any(|arg| arg == "-h" || arg == "--help") {
            print_help(ROOT_HELP);
            return Ok(ExitCode::SUCCESS);
        }
        if args.iter().any(|arg| arg == "--version") {
            print_version();
            return Ok(ExitCode::SUCCESS);
        }
        // argparse prepends the default subcommand when none is named.
        let mut rewritten = Vec::with_capacity(args.len() + 1);
        rewritten.push("convert".to_owned());
        rewritten.extend_from_slice(args);
        let (command, rest) = dispatch(&rewritten)?;
        return run_command(command, rest, None);
    };

    let mut global_config: Option<PathBuf> = None;
    let mut cursor = 0;
    while cursor < index {
        let arg = &args[cursor];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help(ROOT_HELP);
                return Ok(ExitCode::SUCCESS);
            }
            "--version" => {
                print_version();
                return Ok(ExitCode::SUCCESS);
            }
            "-c" | "--config" => {
                let Some(value) = args.get(cursor + 1) else {
                    return Ok(usage_error(
                        ROOT_USAGE,
                        PROG,
                        "argument -c/--config: expected one argument",
                    ));
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
                return Ok(usage_error(
                    ROOT_USAGE,
                    PROG,
                    &format!("unrecognized arguments: {arg}"),
                ));
            }
        }
    }

    dispatch(&args[index..]).and_then(|(command, rest)| run_command(command, rest, global_config))
}

fn dispatch(args: &[String]) -> Result<(&'static str, &[String])> {
    let Some(first) = args.first() else {
        bail!("missing command");
    };
    let command = match first.as_str() {
        "convert" => "convert",
        "diagram" => "diagram",
        "init" => "init",
        _ => bail!("unrecognized command: {first}"),
    };
    Ok((command, &args[1..]))
}

fn run_command(command: &str, rest: &[String], global_config: Option<PathBuf>) -> Result<ExitCode> {
    if rest.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help(match command {
            "convert" => CONVERT_HELP,
            "diagram" => DIAGRAM_HELP,
            _ => INIT_HELP,
        });
        return Ok(ExitCode::SUCCESS);
    }

    match command {
        "convert" => run_convert(rest, global_config),
        "diagram" => run_diagram(rest, global_config),
        "init" => run_init(rest),
        _ => unreachable!(),
    }
}

fn is_command(arg: &str) -> bool {
    matches!(arg, "convert" | "diagram" | "init")
}

fn parse(rest: &[String], fields: &[Field], usage: &str) -> Result<Parser> {
    match Parser::parse(rest.iter().cloned(), fields) {
        Ok(parsed) => Ok(parsed),
        Err(message) => {
            eprintln!("{usage}");
            eprintln!("{PROG}: error: {message}");
            std::process::exit(2);
        }
    }
}

fn selected_config(parsed: &Parser, global: Option<PathBuf>) -> Option<PathBuf> {
    parsed.opt("config").map(PathBuf::from).or(global)
}

fn run_convert(rest: &[String], global_config: Option<PathBuf>) -> Result<ExitCode> {
    let parsed = parse(rest, CONVERT_FIELDS, CONVERT_USAGE)?;
    let config_path = selected_config(&parsed, global_config);
    let config = Config::load(config_path.as_deref())?;

    let spec = parsed
        .positional
        .first()
        .map(String::as_str)
        .unwrap_or(&config.glob);
    let files = select_files(spec, Some(&config.base_dir))?;
    let output = resolve_output(
        parsed.opt("output").map(Path::new),
        &config.output,
        &config.base_dir,
    );
    ensure_parent(&output)?;
    Converter::new(config).files(&files, &output)?;
    println!("Generated: {}", output.display());
    Ok(ExitCode::SUCCESS)
}

fn run_diagram(rest: &[String], global_config: Option<PathBuf>) -> Result<ExitCode> {
    let parsed = parse(rest, DIAGRAM_FIELDS, DIAGRAM_USAGE)?;
    let config_path = selected_config(&parsed, global_config);
    let config = Config::load(config_path.as_deref())?;

    let kind = parsed.opt("kind").unwrap_or("class");
    if kind != "all" && !KINDS.contains(&kind) {
        let choices = KINDS
            .iter()
            .copied()
            .chain(std::iter::once("all"))
            .map(|choice| format!("'{choice}'"))
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(usage_error(
            DIAGRAM_USAGE,
            "doxygen2md diagram",
            &format!("argument -k/--kind: invalid choice: '{kind}' (choose from {choices})"),
        ));
    }

    let spec = parsed
        .positional
        .first()
        .map(String::as_str)
        .unwrap_or(&config.glob);
    let files = select_files(spec, Some(&config.base_dir))?;
    let output = resolve_output(
        parsed.opt("output").map(Path::new),
        &config.diagram,
        &config.base_dir,
    );
    ensure_parent(&output)?;
    Converter::new(config).diagram(&files, &output, kind)?;
    println!("Generated: {}", output.display());
    Ok(ExitCode::SUCCESS)
}

fn run_init(rest: &[String]) -> Result<ExitCode> {
    let parsed = parse(rest, INIT_FIELDS, INIT_USAGE)?;
    let target = parsed
        .positional
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let scaffolder = Scaffolder::new(target, parsed.flag("force"));
    for path in scaffolder.run()? {
        println!("Wrote: {}", path.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn ensure_parent(output: &Path) -> Result<()> {
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    Ok(())
}

fn print_help(text: &str) {
    println!("{text}");
}

fn print_version() {
    println!("{PROG} {VERSION}");
}

fn usage_error(usage: &str, program: &str, message: &str) -> ExitCode {
    eprintln!("{usage}");
    eprintln!("{program}: error: {message}");
    ExitCode::from(2)
}

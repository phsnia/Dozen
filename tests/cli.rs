//! End-to-end checks for the command-line surface `dox.py` exposes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_dozen")
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run dozen")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

fn fixture(name: &str) -> PathBuf {
    root().join("tests/fixtures").join(name)
}

#[test]
fn reports_version() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &["--version"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "doxygen2md 0.5.0\n");
}

#[test]
fn bare_invocation_prints_root_help() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &[]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.starts_with("usage: doxygen2md [-h] [--version] [-c CONFIG]"));
    assert!(text.contains("  {convert,diagram,init}"));
    assert!(text.ends_with("-c, --config CONFIG\n"));
}

#[test]
fn subcommand_help_matches_argparse_layout() {
    let directory = tempfile::tempdir().unwrap();

    let convert = run(directory.path(), &["convert", "-h"]);
    assert!(convert.status.success());
    assert_eq!(
        stdout(&convert),
        "usage: doxygen2md convert [-h] [-o OUTPUT] [-c CONFIG] [input]\n\
         \n\
         positional arguments:\n  input\n\
         \n\
         options:\n  -h, --help           show this help message and exit\n  -o, --output OUTPUT\n  -c, --config CONFIG\n"
    );

    let diagram = run(directory.path(), &["diagram", "--help"]);
    assert!(diagram.status.success());
    assert!(stdout(&diagram).contains(
        "-k, --kind {class,inheritance,collaboration,call,caller,include,directory,all}"
    ));

    let init = run(directory.path(), &["init", "-h"]);
    assert!(init.status.success());
    assert_eq!(
        stdout(&init),
        "usage: doxygen2md init [-h] [--force] [target]\n\
         \n\
         positional arguments:\n  target\n\
         \n\
         options:\n  -h, --help  show this help message and exit\n  --force\n"
    );
}

#[test]
fn unknown_diagram_kind_exits_two() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &["diagram", "-k", "bogus"]);
    assert_eq!(output.status.code(), Some(2));
    let text = stderr(&output);
    assert!(text.starts_with("usage: doxygen2md diagram [-h] [-o OUTPUT]"));
    assert!(text.ends_with(
        "doxygen2md diagram: error: argument -k/--kind: invalid choice: 'bogus' \
         (choose from 'class', 'inheritance', 'collaboration', 'call', 'caller', \
         'include', 'directory', 'all')\n"
    ));
}

#[test]
fn init_scaffolds_and_refuses_to_overwrite() {
    let directory = tempfile::tempdir().unwrap();

    let first = run(directory.path(), &["init"]);
    assert!(first.status.success());
    assert_eq!(stdout(&first), "Wrote: ini.cfg\n");
    let config = std::fs::read_to_string(directory.path().join("ini.cfg")).unwrap();
    assert!(config.contains("glob     = *"));
    assert!(config.contains("output   = README.md"));

    let second = run(directory.path(), &["init"]);
    assert_eq!(second.status.code(), Some(1));
    assert_eq!(
        stderr(&second),
        "Refusing to overwrite ini.cfg (use --force).\n"
    );

    let forced = run(directory.path(), &["init", "--force"]);
    assert!(forced.status.success());
    assert_eq!(stdout(&forced), "Wrote: ini.cfg\n");
}

#[test]
fn init_target_prints_joined_path() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &["init", "docs"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "Wrote: docs/ini.cfg\n");
    assert!(directory.path().join("docs/ini.cfg").is_file());
}

#[test]
fn convert_defaults_to_readme_and_discovered_spec() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(fixture("animal.py"), directory.path().join("animal.py")).unwrap();

    let output = run(directory.path(), &["convert"]);
    assert!(output.status.success());
    let readme = directory.path().join("README.md");
    assert_eq!(
        stdout(&output),
        format!("Generated: {}\n", readme.display())
    );
    let actual = std::fs::read_to_string(&readme).unwrap();
    let golden = std::fs::read_to_string(root().join("tests/golden/animal.py.convert.md")).unwrap();
    assert_eq!(actual, golden);
}

#[test]
fn bare_arguments_default_to_convert() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(fixture("animal.rs"), directory.path().join("animal.rs")).unwrap();

    let output = run(directory.path(), &["-o", "API.md", "animal.rs"]);
    assert!(output.status.success());
    let output_path = directory.path().join("API.md");
    assert_eq!(
        stdout(&output),
        format!("Generated: {}\n", output_path.display())
    );
    let actual = std::fs::read_to_string(&output_path).unwrap();
    let golden = std::fs::read_to_string(root().join("tests/golden/animal.rs.convert.md")).unwrap();
    assert_eq!(actual, golden);
}

#[test]
fn config_discovery_controls_glob_and_diagram_output() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(fixture("animal.py"), directory.path().join("animal.py")).unwrap();
    std::fs::write(
        directory.path().join("ini.cfg"),
        "[defaults]\nglob = animal.py\ndiagram = GRAPH.md\nencoding = utf-8\n",
    )
    .unwrap();

    let output = run(directory.path(), &["diagram", "-k", "class"]);
    assert!(output.status.success());
    let graph = directory.path().join("GRAPH.md");
    assert_eq!(stdout(&output), format!("Generated: {}\n", graph.display()));
    let text = std::fs::read_to_string(&graph).unwrap();
    assert!(text.starts_with("# Contract Graph\n\n## Class Diagram\n\n```mermaid\nclassDiagram\n"));
    assert!(text.contains("  class Animal {"));
    assert!(!text.contains("## Inheritance Diagram"));
}

#[test]
fn global_config_flag_applies_before_and_after_the_subcommand() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(fixture("animal.py"), directory.path().join("animal.py")).unwrap();
    std::fs::write(
        directory.path().join("cfg.ini"),
        "[defaults]\nglob = animal.py\noutput = OUT.md\n",
    )
    .unwrap();

    let before = run(directory.path(), &["-c", "cfg.ini", "convert"]);
    assert!(before.status.success());
    assert_eq!(stdout(&before), "Generated: OUT.md\n");

    std::fs::remove_file(directory.path().join("OUT.md")).unwrap();
    let after = run(directory.path(), &["convert", "-c", "cfg.ini"]);
    assert!(after.status.success());
    assert_eq!(stdout(&after), "Generated: OUT.md\n");
}

#[test]
fn missing_explicit_config_reports_the_path() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &["-c", "nope.cfg", "convert"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "Config file not found: nope.cfg\n");
}

#[test]
fn unknown_global_option_exits_two() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), &["-c", "cfg.ini", "--bogus", "convert"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).ends_with("doxygen2md: error: unrecognized arguments: --bogus\n"));
}

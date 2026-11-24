//! Byte-for-byte comparison against frozen outputs from `docs/dox.py` 0.5.0.

use std::path::{Path, PathBuf};

use dozen::engine::config::Config;
use dozen::engine::converter::Converter;
use dozen::engine::files::FileFinder;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Replace machine-specific repository paths so the tests stay portable.
///
/// Mermaid identifiers sanitize `/` and other punctuation to `_`, so both the
/// raw path and its sanitized form need rewriting.
fn normalize(text: &str, root: &Path) -> String {
    let raw = root.to_string_lossy();
    let sanitized: String = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    text.replace(&sanitized, "_ROOT")
        .replace(raw.as_ref(), "<ROOT>")
}

fn load_config(root: &Path, config: Option<&str>) -> Config {
    match config {
        Some(path) => Config::load(Some(&root.join(path))).expect("load config"),
        None => {
            let mut cfg = Config::defaults();
            cfg.base_dir = root.to_path_buf();
            cfg
        }
    }
}

fn select(cfg: &Config, input: Option<&str>) -> Vec<PathBuf> {
    let spec = input.unwrap_or(&cfg.glob);
    FileFinder::new(Some(&cfg.base_dir))
        .select(spec)
        .expect("select files")
}

fn convert(root: &Path, input: Option<&str>, config: Option<&str>) -> String {
    let cfg = load_config(root, config);
    let files = select(&cfg, input);
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("out.md");
    Converter::new(cfg)
        .files(&files, &output)
        .expect("convert files");
    std::fs::read_to_string(output).expect("read output")
}

fn diagram(root: &Path, input: Option<&str>, config: Option<&str>) -> String {
    let cfg = load_config(root, config);
    let files = select(&cfg, input);
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("out.md");
    Converter::new(cfg)
        .diagram(&files, &output, "all")
        .expect("diagram files");
    std::fs::read_to_string(output).expect("read output")
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(repo_root().join("tests/golden").join(name)).expect("read golden")
}

fn assert_matches(name: &str, actual: &str) {
    let root = repo_root();
    let expected = normalize(&golden(name), &root);
    let actual = normalize(actual, &root);
    assert_eq!(expected, actual, "mismatch against golden {name}");
}

#[test]
fn python_fixture_convert_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.py.convert.md",
        &convert(&root, Some("tests/fixtures/animal.py"), None),
    );
}

#[test]
fn rust_fixture_convert_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.rs.convert.md",
        &convert(&root, Some("tests/fixtures/animal.rs"), None),
    );
}

#[test]
fn python_fixture_diagram_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.py.graph.md",
        &diagram(&root, Some("tests/fixtures/animal.py"), None),
    );
}

#[test]
fn rust_fixture_diagram_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.rs.graph.md",
        &diagram(&root, Some("tests/fixtures/animal.rs"), None),
    );
}

#[test]
fn self_hosted_contract_matches_golden() {
    let root = repo_root();
    assert_matches(
        "dox.contract.md",
        &convert(&root, None, Some("docs/dox-contract.cfg")),
    );
}

#[test]
fn self_hosted_diagram_matches_golden() {
    let root = repo_root();
    assert_matches(
        "dox.graph.md",
        &diagram(&root, None, Some("docs/dox-contract.cfg")),
    );
}

#[test]
fn cpp_fixture_convert_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.cpp.convert.md",
        &convert(&root, Some("tests/fixtures/animal.*pp"), None),
    );
}

#[test]
fn cpp_fixture_diagram_matches_golden() {
    let root = repo_root();
    assert_matches(
        "animal.cpp.graph.md",
        &diagram(&root, Some("tests/fixtures/animal.*pp"), None),
    );
}

#[test]
fn python_bank_fixture_matches_golden() {
    let root = repo_root();
    assert_matches(
        "bank.py.convert.md",
        &convert(&root, Some("tests/fixtures/bank.py"), None),
    );
    assert_matches(
        "bank.py.graph.md",
        &diagram(&root, Some("tests/fixtures/bank.py"), None),
    );
}

#[test]
fn rust_shapes_fixture_matches_golden() {
    let root = repo_root();
    assert_matches(
        "shapes.rs.convert.md",
        &convert(&root, Some("tests/fixtures/shapes.rs"), None),
    );
    assert_matches(
        "shapes.rs.graph.md",
        &diagram(&root, Some("tests/fixtures/shapes.rs"), None),
    );
}

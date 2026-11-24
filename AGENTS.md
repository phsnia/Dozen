# AGENTS.md

Guidance for AI coding agents working in the **Dozen** repository.

## Project summary

`dozen` is a pure-Rust port of `docs/dox.py` 0.5.0. The compiled binary is
named `dozen`, but the user-facing program name is `doxygen2md`.

It turns Doxygen-style source comments into:

- uniform Markdown API references (`convert`),
- Mermaid diagrams (`diagram`),
- a starter configuration file (`init`).

The Python reference `docs/dox.py` is the behavior oracle. Runtime code must
never invoke Python; Python is only used to regenerate golden files.

**Parity is the core requirement.** For every supported input, `dozen` output
must match `docs/dox.py` byte-for-byte, including path rendering, ordering,
whitespace, and error messages.

## Build and run

```bash
cargo build                 # target/debug/dozen
cargo run -- --version      # doxygen2md 0.5.0
```

`cargo run` needs `--` before CLI arguments:

```bash
cargo run -- convert src/main.rs -o API.md
cargo run -- diagram src/main.rs -k all -o GRAPH.md
```

## Using the tool

### `convert` — Markdown reference

```bash
dozen convert [input] [-o OUTPUT] [-c CONFIG]
```

```bash
# One file
dozen convert src/main.rs -o API.md

# Glob (quote it so the tool expands it, not the shell)
dozen convert 'src/**/*.rs' -o API.md

# Use the glob/output from the discovered ini.cfg
dozen convert

# Explicit config
dozen -c docs/dox-contract.cfg convert
dozen convert -c docs/dox-contract.cfg
```

- `input` defaults to the config `glob`; with no config it is `*`.
- `output` defaults to the config `output`; built-in default is `README.md`.
- Relative outputs resolve against the config file's directory (or the current
  directory when no config is found).
- Missing parent directories are created automatically.
- Prints `Generated: <path>` on success.

### `diagram` — Mermaid graphs

```bash
dozen diagram [input] [-o OUTPUT] [-k KIND] [-c CONFIG]
```

Kinds: `class`, `inheritance`, `collaboration`, `call`, `caller`, `include`,
`directory`, and `all`. The default is `class`.

```bash
dozen diagram src/lib.rs -k class
dozen diagram 'src/**/*.rs' -k all -o GRAPH.md
dozen -c docs/dox-contract.cfg diagram -k all -o DIAGRAM.md
```

- `output` defaults to the config `diagram`; built-in default is `DIAGRAM.md`.
- An invalid `-k` value exits with code `2` and an argparse-style message.
- Prints `Generated: <path>` on success.

### `init` — scaffold `ini.cfg`

```bash
dozen init [target] [--force]
```

```bash
dozen init            # writes ./ini.cfg
dozen init docs       # writes docs/ini.cfg
dozen init --force    # overwrite an existing file
```

- Refuses to overwrite without `--force` (exit `1`).
- Prints `Wrote: <path>`.

### Configuration

`convert` and `diagram` discover `./ini.cfg` when `-c` is not given. An explicit
`-c` may appear before or after the subcommand; the subcommand value wins.

```ini
[defaults]
glob     = *
output   = README.md
diagram  = DIAGRAM.md
encoding = utf-8
```

- Only the `[defaults]` section is read; unknown keys are ignored.
- `encoding` is handled by `encoding_rs` for both reading sources and writing
  output.
- Empty `output`/`diagram` values become `.` to mirror Python `Path("")`.

### Exit codes and streams

| Code | Meaning |
|------|---------|
| `0` | success |
| `1` | config not found, no source files, refuse overwrite, other runtime error |
| `2` | invalid CLI usage (unknown option, bad `-k`, missing option value) |

Successful output (`Generated:` / `Wrote:`) goes to stdout. Errors go to
stderr.

## Supported inputs

| Language | Extensions |
|----------|------------|
| Python | `.py` |
| Rust | `.rs` |
| C++ | `.cpp`, `.hpp`, `.h`, `.cc`, `.cxx`, `.hh` |
| Bash | `.sh`, `.bash` |

Extensions are matched case-insensitively. Directories are walked recursively;
unsupported files are skipped.

## Supported doc tags

`@brief`, `@details`, `@param[in]`, `@return` / `@returns`,
`@throws` / `@exception` / `@raises`, `@see`, `@note`, `@warning`, `@example`.

Tags accept `@` or `\` and are case-insensitive. Untagged leading text becomes
`brief`, then `details`.

## Architecture

```text
src/
  main.rs                 thin entry point -> cli::run
  lib.rs                  public modules for integration tests
  cli/
    mod.rs                convert / diagram / init orchestration + help text
    parser.rs             hand-rolled argv parser
    scaffold.rs           init / ini.cfg writing
  engine/
    mod.rs                pipeline module declarations
    config.rs             Config, defaults, ini parsing, discovery
    converter.rs          parse + stamp entities + write output
    files.rs              glob expansion, base candidates, output resolution
  documentation/
    mod.rs                DocParser, split_lines, cleandoc
  model/
    mod.rs                EntityKind, Param, Variant, Field, Doc, Entity
  lang/
    mod.rs                Language trait + shared source helpers
    registry.rs           extension -> language mapping
    rust.rs               Rust parser (Tree-sitter + Rust-native lowering)
    cpp.rs                C++ parser (Tree-sitter + Rust-native lowering)
    bash.rs               Bash parser (Tree-sitter + Rust-native lowering)
    python.rs             Python parser (Tree-sitter + Rust-native AST lowering)
  renderer/
    mod.rs
    reference.rs          Markdown API reference renderer
    mermaid.rs            all seven Mermaid diagram kinds
tests/
  golden.rs               byte-for-byte parity tests
  cli.rs                  binary integration tests
  fixtures/               animal.py, animal.rs, animal.cpp/.hpp, bank.py, shapes.rs
  golden/                 frozen Python-generated expected outputs
```

Python parsing uses pinned `tree-sitter` and `tree-sitter-python` crates.
Tree-sitter syntax nodes are lowered through a Rust-native Python AST model;
a small custom expression renderer matches the CPython annotation formatting
used by the golden suite.

Rust, C++, and Bash parsing use pinned Tree-sitter grammar crates and
Rust-native lowering behind the same `Language` plugin interface.

## Development workflow

Run all quality gates before finishing any change:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Current suites: unit tests in `src/`, CLI tests in `tests/cli.rs`, and golden
parity tests in `tests/golden.rs`.

## Testing and golden parity

- `tests/golden.rs` compares generated output against frozen files in
  `tests/golden/`. It normalizes the repository root to `<ROOT>` and its
  Mermaid-sanitized form to `_ROOT` so goldens stay portable.
- `tests/cli.rs` drives the real binary through `CARGO_BIN_EXE_dozen` and
  `tempfile`.
- Fixtures live in `tests/fixtures/`.

Only regenerate goldens when intentionally syncing with `docs/dox.py`:

```bash
python3 docs/dox.py convert tests/fixtures/animal.py -o tests/golden/animal.py.convert.md
python3 docs/dox.py diagram tests/fixtures/animal.py -k all -o tests/golden/animal.py.graph.md

# Repeat for animal.rs, 'animal.*pp', bank.py, and shapes.rs.

python3 docs/dox.py -c docs/dox-contract.cfg convert -o tests/golden/dox.contract.md
python3 docs/dox.py -c docs/dox-contract.cfg diagram -k all -o tests/golden/dox.graph.md
```

When changing behavior, verify parity directly against the Python oracle on
real files (for example Python stdlib modules and Rust sources) before
updating a golden.

## Invariants and conventions

- **Do not add `clap`.** Extend `src/cli/parser.rs`.
- **Do not "improve" output formatting.** Whitespace, separators, ordering,
  empty cells, and path strings are part of the contract.
- **No `python3` subprocess at runtime.** Python is only for regenerating
  goldens.
- Preserve Python quirks deliberately implemented in the port: `DocParser`
  blank-line handling, `inspect.cleandoc`, `str.splitlines`, glob base
  candidates, `pathlib`-style path normalization, and `ast.unparse` formatting.
- Keep CLI help text, usage lines, error strings, and exit codes identical to
  argparse.
- Add tests for new language constructs, tags, or diagram behavior, and keep
  the golden suite green.

## Common tasks

**Add a language**
1. Implement the `Language` trait in `src/lang/<name>.rs`.
2. Declare the module in `src/lang/mod.rs`.
3. Register extensions in `src/lang/registry.rs` (`EXTENSIONS` and
   `language_for`).
4. Add fixtures and Python-generated goldens, then a test in
   `tests/golden.rs`.

**Add or change a doc tag**
1. Update `src/documentation/mod.rs`.
2. Update the renderer if the tag gets its own section.
3. Add a unit test and regenerate/inspect affected goldens.

**Add a diagram kind**
1. Implement it in `src/renderer/mermaid.rs`.
2. Add it to `KINDS` and the dispatch in `render_all`.
3. Add a CLI test and a golden fixture.

## Notes

- `target/` is the only ignored path; all source and docs are untracked until
  the first commit.
- `src/ast/`, `src/graph/`, `src/parser/`, and `src/semantic/` contain empty,
  unreferenced skeleton files from an earlier design. They are not compiled
  and should not be wired in without an intentional refactor.
- `docs/dozen-rust-port.md` is the implementation plan and parity checklist.

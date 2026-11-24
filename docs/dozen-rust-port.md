# Dozen: Rust port of `docs/dox.py`

## Goal

Replace the partial Rust scaffolding in `src/` with a pure-Rust implementation of
`docs/dox.py` 0.5.0 that is behaviorally and byte-for-byte compatible with the
Python reference for:

- `convert` — emit uniform Markdown API references.
- `diagram` — emit Mermaid graphs (`class`, `inheritance`, `collaboration`,
  `call`, `caller`, `include`, `directory`).
- `init` — scaffold `ini.cfg`.

No `python3` subprocess is used at runtime.

### Decisions

| Topic | Decision |
|-------|----------|
| CLI surface | Mirror `dox.py`: `convert`, `diagram`, `init`; `ini.cfg` discovery; identical flags/defaults. Remove the placeholder `run`/`build`/`test`/`check`/`languages`/`files` surface. |
| Python parsing | Pure Rust. Use the Ruff parser crates. Fallback if Ruff APIs/unparse do not match: a hand-written Python scanner or small custom unparser. |
| Fidelity | Byte-for-byte output against frozen Python-generated golden files. |
| CLI parser | Extend the existing hand-rolled `src/parser.rs` instead of adding `clap`. |
| Version/name | Package version `0.5.0`; displayed program name `doxygen2md`; binary name stays `dozen`. |

### Non-goals

- No new diagram kinds or tags beyond `dox.py`.
- No runtime Python dependency.
- No automatic editing of `docs/dox.py`; it remains the reference oracle.

---

## Current state

- `Cargo.toml`: package `dozen` 0.5.0, edition 2024; runtime dependencies
  `anyhow`, `regex`, `glob`, `encoding_rs`, and pinned Ruff crates
  (`ruff_python_parser`, `ruff_python_ast`, `ruff_python_codegen`, plus
  `ruff_source_file` for line endings).
- `src/main.rs`: thin entry point over `dozen::cli::run`.
- `src/lib.rs`: public modules for integration tests.
- `src/cli/`: command objects with argparse-compatible help, errors, and exit
  codes (`mod.rs`, `command.rs`), the hand-rolled argv parser (`parser.rs`),
  and `ini.cfg` scaffolding (`scaffold.rs`).
- `src/engine/`: `Config`, glob/file selection, and the conversion pipeline.
- `src/documentation/mod.rs`: `DocParser`, `Tag`/`ListKind`, and `DocBuilder`.
- `src/model/mod.rs`: the language-agnostic entity/documentation model.
- `src/lang/`: language plugins (`rust.rs`, `cpp.rs`, `python.rs`), the shared
  `Language` trait, and the extension registry (`registry.rs`).
- `src/renderer/`: Markdown reference and Mermaid diagram renderers.
- `docs/dox.py`: complete reference implementation (2,291 lines).
- `docs/dox-contract.md`, `docs/dox-contract.cfg`,
  `docs/dox-contract-graph.md`: expected self-hosted output.
- `tests/golden.rs`: byte-for-byte parity tests against frozen Python output.
- `tests/cli.rs`: binary integration tests.
- `tests/fixtures/`: `animal.py`, `animal.rs`, `animal.cpp`/`animal.hpp`,
  `bank.py`, `shapes.rs`.
- `tests/golden/`: frozen outputs for every fixture, `-k all` diagrams, and the
  self-hosted contract.

`cargo test` passes (47 tests across unit, golden, and CLI suites).

---

## Target layout

```text
Cargo.toml
src/
  main.rs              # argv -> cli::run -> ExitCode
  lib.rs               # public modules for integration tests
  cli/
    mod.rs             # Cli struct + root argparse surface
    command.rs         # Command trait, Convert/Diagram/Init objects
    parser.rs          # hand-rolled argv parser
    scaffold.rs        # init / ini.cfg writing
  engine/
    mod.rs             # pipeline module declarations
    config.rs          # Config, DEFAULT_CONFIG_TEXT, Config::parse, load
    converter.rs       # TextCodec + Converter pipeline
    files.rs           # FileFinder + OutputResolver
  documentation/
    mod.rs             # DocParser, Tag/ListKind, DocBuilder, split_lines
  model/
    mod.rs             # EntityKind, Param, Variant, Field, Doc, Entity
  lang/
    mod.rs             # Language trait, consume_block, read_block_comment
    registry.rs        # LanguageRegistry (extension -> language)
    rust.rs            # heuristic port of RustLanguage
    cpp.rs             # heuristic port of CppLanguage
    python.rs          # Ruff-AST port of PythonLanguage
  renderer/
    mod.rs
    reference.rs       # ReferenceRenderer
    mermaid.rs         # MermaidRenderer + 7 diagram kinds
tests/
  golden.rs            # byte-for-byte parity tests
  cli.rs               # binary integration tests
  fixtures/            # animal.*, bank.py, shapes.rs, edge cases
  golden/              # frozen expected .md outputs
```

### Dependency plan

```toml
[dependencies]
anyhow = "1"
regex = "1"
glob = "0.3"
encoding_rs = "0.8"
ruff_python_parser = "=0.0.23"
ruff_python_ast = "=0.0.23"
ruff_python_codegen = "=0.0.23"

[dev-dependencies]
tempfile = "3"
```

- Pin Ruff crates exactly; they are internal component crates.
- Configure `glob` with `require_literal_separator = true` and
  `require_literal_leading_dot = true` to approximate Python `glob`.
- `encoding_rs` honors the `encoding` config key; keep a UTF-8 fast path.
- No `clap`; extend the existing parser.

---

## Milestone 0 — Freeze the oracle and add the harness

### Tasks

1. Generate golden outputs with the Python reference.

   ```bash
   mkdir -p tests/golden

   python3 docs/dox.py convert tests/fixtures/animal.py \
     -o tests/golden/animal.py.convert.md
   python3 docs/dox.py convert tests/fixtures/animal.rs \
     -o tests/golden/animal.rs.convert.md
   python3 docs/dox.py diagram tests/fixtures/animal.rs -k all \
     -o tests/golden/animal.rs.graph.md
   python3 docs/dox.py diagram tests/fixtures/animal.py -k all \
     -o tests/golden/animal.py.graph.md

   python3 docs/dox.py -c docs/dox-contract.cfg convert \
     -o tests/golden/dox.contract.md
   python3 docs/dox.py -c docs/dox-contract.cfg diagram -k all \
     -o tests/golden/dox.graph.md
   ```

2. Copy `tests/fixtures/animal.*` into `tests/fixtures/`; add a C++ fixture and
   edge-case Rust/Python fixtures.
3. Normalize the repository root in golden diagram files to `<ROOT>`
   (`/home/phng/Code/Wheels/Dozen` and its canonical form).
4. Add `tests/golden.rs` with:
   - a `normalize_paths` helper that replaces the actual manifest directory and
     canonical form with `<ROOT>`;
   - helpers that call the library and compare exact bytes for convert outputs;
   - path-normalized comparison for diagrams with absolute paths.
5. Add a test regenerating `docs/dox-contract.md` and comparing to the checked-in
   file.
6. Convert `src/main.rs` into a thin binary over a new `src/lib.rs`.

### Acceptance

- `cargo test` compiles the new harness; parity tests are ignored/pending until
  implementation exists.
- Golden files are checked in and reproducible.

---

## Milestone 1 — Model, DocParser, Config

### `src/model/mod.rs`

Port exactly:

- `EntityKind`: `MODULE`, `FUNCTION`, `METHOD`, `CLASS`, `STRUCT`, `ENUM`,
  `TRAIT`, `INTERFACE`, with lowercase string values.
- `Param { name, type, direction = "in", description }`
- `Variant { name, description }`
- `Field { name, type, visibility = "public", static, description }`
- `Doc { brief, details, params, returns, throws, see, notes, warnings,
  examples }` (`throws: Vec<(String, String)>`)
- `Entity { kind, name, signature, return_type, doc, abstract, members,
  variants, fields, bases, implements, module, path, line, calls, includes }`
- `Default` implementations matching Python dataclass defaults.

### `src/documentation/mod.rs`

Port `DocParser` line-by-line.

- `TAG_RE = ^\s*[@\\](\w+)(?:\[([^\]]+)\])?(?:\s+(.*))?$`
- First-tag split; leading paragraphs become `brief`, then `details`.
- Alias map: `return`/`returns` -> returns; `exception`/`raises` -> throws.
- List tags: `see` -> `see`, `note` -> `notes`, `warning` -> `warnings`,
  `example` -> `examples`.
- Preserve quirks:
  - Blank lines append `"\n"` only to `details` and the active example.
  - `list_name` is not reset by a new non-list tag.
  - Unknown tags set the active tag to `None`; continuation text falls back to
    `brief`.
  - `details` is trimmed only of newlines; `brief` is fully trimmed.
  - `param`/`throws` partition at the first space; descriptions are stripped.
- Add `split_lines` matching `str.splitlines()` for `\n`, `\r\n`, `\r`, `\v`,
  `\f`, `\x1c`-`\x1e`, `\x85`, `\u2028`, `\u2029`.
- Add `cleandoc` matching `inspect.cleandoc`:
  - expand tabs with tab size 8,
  - remove the minimum indentation of non-blank lines after the first,
  - strip leading/trailing blank lines,
  - left-strip the first line.

### `src/engine/config.rs`

- `CONFIG_FILENAME = "ini.cfg"`.
- Exact `DEFAULT_CONFIG_TEXT`.
- `Config` with `glob`, `output`, `diagram`, `encoding`, `base_dir`.
- `Config::defaults`, `Config::load`, `Config::parse` (delegating to the private
  `ConfigBuilder` state object):
  - trim lines; skip empty and `#`/`;` comments;
  - `[section]` matching, lowercase section;
  - `key = value` only in `[defaults]`;
  - lowercase keys; empty encoding becomes `utf-8`;
  - unknown/duplicate keys ignored/last-wins like Python.
- Missing explicit config: error message `Config file not found: <path>` and
  exit 1.

### Tests

- Each tag, alias, direction, continuation rule, blank-line rule, unknown tag.
- Paragraph splitting and `cleandoc`.
- Config defaults, comments, sections, case-insensitivity, empty encoding,
  explicit vs discovered config.

---

## Milestone 2 — Language trait and shared helpers

### `src/lang/mod.rs`

```rust
pub trait Language {
    fn extensions(&self) -> &'static [&'static str];
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity>;
}
```

Port exactly:

- `consume_block(lines, start)`:
  - per-character `{`/`}` depth;
  - `opened` flag;
  - return when opened and depth <= 0;
  - without braces, return on `;`;
  - unmatched opens return `lines.len() - start`.
- `read_block_comment(lines, start)`:
  - only starts when the line matches `^\s*/\*\*`;
  - strip `^\s*\*\s?`;
  - append only non-blank stripped lines;
  - return `(lines, consumed)`; `([], 0)` when not a block start.
- Safe body-slice helper that clamps like Python slicing.

### `src/lang/registry.rs`

- Extension -> language mapping for:
  - `.py`
  - `.rs`
  - `.cpp`, `.hpp`, `.h`, `.cc`, `.cxx`, `.hh`
- Lowercase matching.
- Return a boxed language or use static dispatch.

---

## Milestone 3 — Rust language

### `src/lang/rust.rs`

Port `RustLanguage` exactly, including regexes:

```text
_VIS       (?:pub(?:\([^)]*\))?\s+)?
_DOC_LINE  ^\s*///\s?(.*)$
_ATTR      ^\s*#\[.*\]\s*$
_FN        ^\s* VIS (?:(?:const|async|unsafe|extern(?:\s+"[^"]*")?)\s+)*
           fn\s+(\w+)\s*(?:<[^>]*>)?\s*\((.*?)\)
           (?:\s*->\s*([^{;]+?))?\s*(?:\{|;|where|$)
_STRUCT    ^\s* VIS struct\s+(\w+)
_ENUM      ^\s* VIS enum\s+(\w+)
_TRAIT     ^\s* VIS (?:unsafe\s+)?trait\s+(\w+)(?:\s*:\s*([^{;]+?))?\s*(?:\{|;|$)
_IMPL      ^\s*impl(?:\s*<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)
_VARIANT   ^\s*(?:#\[.*\]\s*)?(\w+)\s*(?:[({,]|$)
_FIELD     ^(pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*(.+?),?\s*$
_USE       ^\s*(?:pub\s+)?use\s+([^;]+);
_MOD       ^\s*(?:pub\s+)?mod\s+(\w+)\s*;
_EXTERN    ^\s*extern\s+crate\s+(\w+)
_CALL      \b([A-Za-z_]\w*)\s*\(
```

Port behavior:

- Iterate lines; collect `///` doc lines and `/** ... */` blocks.
- Skip blank lines, `//` comments, attributes.
- Try `fn`, `struct`, `enum`, `trait` in that order.
- Merge `impl` methods into an earlier entity by target name; append trait to
  `implements`.
- Reset pending docs on unmatched lines and on `impl` lines.
- Struct fields: visibility, type, doc text.
- Enum variants: alpha-first check, doc text.
- Impl/trait methods: abstract when the consumed block contains no `{`.
- `use`/`mod`/`extern crate` includes.
- Calls: strip `//` comments, regex find, dedupe, stop-word set exactly.
- Argument splitter: top-level commas, depth over `<([` / `>)]`.
- Preserve ordering and quirks, including dropped docs before `impl`.

### Tests

- `animal.rs` convert + all diagrams match golden.
- Additional fixture: generics, `pub(crate)`, trait bounds, block comments,
  one-line functions, calls, nested `impl`, methods on unknown target.

---

## Milestone 4 — C++ language

### `src/lang/cpp.rs`

Port `CppLanguage` exactly:

- `_DOC_LINE`, `_ATTR` (`[[...]]`), `_FN`, `_CLASS`, `_ENUM`, `_ABSTRACT`,
  `_FIELD`, `_INCLUDE`, `_CALL`.
- Top-level precedence: function -> class/struct -> enum.
- `Class::method` handling and merging into existing class.
- Access sections; default visibility `public` for struct, `private` for class.
- Skip `using`, `typedef`, `friend`, `template` lines and lines containing
  `(`, `{`, `}` when looking for fields.
- Static field detection; ignore types `return`/`delete`.
- Base-clause cleanup: remove access keywords, generic suffix, `::` qualifier.
- Enum variants: split by comma, keep identifiers.
- Includes, calls with stop-word set.
- Parameter type inference and `_merge_types`.

### Tests

- New `tests/fixtures/animal.hpp` exercising inheritance, visibility, static
  fields, abstract methods, and includes.
- Golden convert + diagram outputs generated from Python.

---

## Milestone 5 — Python language (risk spike first)

### Spike

1. Add Ruff crates and confirm AST traversal APIs.
2. Compare Ruff `Generator` output with Python `ast.unparse` for:
   - `list["Entity"]` -> `list['Entity']`
   - `Path | None`
   - `tuple[str, str]`
   - `dict[str, int]`
   - forward-ref strings
3. If output differs, implement a targeted unparser for annotations
   (Name, Attribute, Subscript, Tuple, BinOp `|`, Constant, Call).

### `src/lang/python.rs`

Port `PythonLanguage` behavior:

- Parse module; on syntax error return `[]`.
- Module entity:
  - name = filename, line 1;
  - doc from cleaned module docstring;
  - includes from imports over the whole tree (BFS `ast.walk` order).
- Top-level body only: `FunctionDef`, `AsyncFunctionDef`, `ClassDef`.
- Function:
  - docstring via `cleandoc`;
  - annotation map from `args.args` only (not posonly/kwonly/vararg);
  - fill missing doc param types;
  - return type unparse;
  - signature excludes `self`/`cls`, includes annotations, uses `async def`;
  - calls via BFS with nested def/class/lambda pruning; name reduction exactly:
    - Name -> id;
    - Attribute -> owner (Name id, or recursively call-name of Call) + `.attr`;
    - else empty; dedupe.
- Class:
  - abstract via `ABC`/`ABCMeta` bases or abstract methods;
  - enum via `Enum`, `IntEnum`, `StrEnum`, `Flag`, `IntFlag` bases;
  - enum variants from top-level `Assign` targets not starting `_`;
  - bases cleaned through `_node_name` (Name/Attribute/Subscript), excluding
    abstract/enum bases and `object`;
  - fields:
    - top-level `AnnAssign` name targets (public default);
    - `__init__` walk: `self.attr` annotations and assignments;
    - inferred type when assignment value is a `Name` matching an annotated
      init arg;
    - visibility by name (`__x` private, `_x` protected, else public);
    - skip dunder names;
    - first type wins when enriching;
    - descriptions from `__init__` doc params.
  - members: top-level functions, public unless `_`-prefixed and not
    `__init__`/`__del__`; kind METHOD; abstract decorators
    `abstractmethod`/`abstractproperty`; line numbers.
- Imports:
  - `import a.b` -> `a.b`;
  - `from ..pkg import x` -> `..pkg`;
  - dedupe preserving BFS order; ignore imported names/wildcards.
- Replicate BFS ordering with a queue and identical child order.
- Preserve quirks: only `args.args`, only FunctionDef for abstract checks,
  Subscript excluded from enum/abstract base detection, etc.

### Tests

- `animal.py` convert + diagrams match golden.
- Self-hosted `docs/dox.py` contract matches `tests/golden/dox.contract.md`
  byte-for-byte.
- Edge fixture: async functions, decorators, nested defs/classes, forward refs,
  unions, dataclasses, relative imports, enum subclasses.

---

## Milestone 6 — Renderers

### `src/renderer/reference.rs`

Port `ReferenceRenderer`:

- `render`: filter empty entity renderings; join with `\n\n---\n\n`.
- Dispatch by kind: enum -> enum renderer; class/struct/trait/interface ->
  class renderer; module -> brief/details/examples; else function.
- Function:
  - `## \`name()\``
  - fenced signature
  - brief, details, parameter table, returns, exception table
  - notes/warnings as `> ` blocks joined by blank lines
  - example section, see-also list
- Class:
  - title `` `Name` Kind Reference `` plus ` *abstract*`
  - brief/details, examples
  - `**Inherits:**` / `**Implements:**` relation lines
  - fields table, member summary table
  - method contracts only when a member has more than a brief
- Enum: heading, brief/details, enumerator table.
- Table cell escaping: `|` -> `\|`.
- Exact empty-cell behavior, including `` ` ` `` for empty return types.
- Example section: skip empty examples; use existing fences when content
  contains ```; otherwise wrap in ```; omit heading when no examples.

### `src/renderer/mermaid.rs`

Port all seven kinds in order:
`class`, `inheritance`, `collaboration`, `call`, `caller`, `include`,
`directory`.

Implement:

- `render_all` exact document layout: `# Contract Graph\n`, level-2 headings,
  fenced `mermaid` blocks.
- Class diagram:
  - collect block-like entities recursively (members only);
  - identifiers sanitized to `[A-Za-z0-9_]`, `entity` fallback, `_2` suffixes;
  - `by_name` first occurrence; `by_file` `(path, name)` first occurrence;
  - block indentation 2/4 spaces; stereotypes; field markers; enum variants;
  - `_display_type` transformations;
  - inheritance `<|--`, implementation `<|..`, dependency `..>`;
  - type-token extraction and built-in exclusion;
  - lookup rules (strip generics/`::`, dotted names, by-file then by-name).
- Inheritance diagram: no details, no dependencies, no enums.
- Collaboration diagram: `flowchart LR`, node labels, `-->|uses|` edges from
  fields, member return types, and param doc types.
- Call/caller:
  - flatten functions/methods with owner;
  - `fn_` identifiers from qualified names;
  - labels `module.owner.name`;
  - duplicate counts; by-name only when unique; by-file only when unique;
  - `self.`/`cls.` -> owner resolution;
  - inverse edge direction and `called by` label;
  - dedupe.
- Include graph:
  - module entities only;
  - `file_` identifiers from sanitized POSIX path;
  - labels from basename or module name;
  - external `ext_N` nodes for unresolved includes;
  - candidate resolution via raw path, basename, stem, normalized `::`/`/`
    splits;
  - dedupe, skip self edges.
- Directory graph:
  - module parent directories, sorted;
  - `dir_N` identifiers;
  - labels directory or `"."`;
  - resolved cross-directory edges only.
- Label escaping: `"` -> `'`, newline -> space.

### IDs

Use a flattened `Vec<&Entity>` and index-based `NodeId` maps. This replaces
Python `id(entity)` while preserving creation order and duplicate-name suffix
behavior.

### Tests

- Golden comparison for all seven kinds on `animal.*`, C++ fixture, and `dox.py`.
- Duplicate-name/ID behavior.
- Edge dedupe and empty-diagram cases.

---

## Milestone 7 — Files, converter, scaffold, CLI

### `src/engine/files.rs`

- `FileFinder::new(base_dir)` / `FileFinder::collect(root)`:
  - `candidate_bases` -> resolved base dir, its parent, current dir;
  - dedupe preserving order;
  - absolute pattern -> single search base;
  - relative -> candidate bases;
  - first base with matches wins;
  - Python-like glob options;
  - file candidates accepted only with a registered extension;
  - directory candidates recursively include matching files;
  - dedupe, then sort by path string (Python `Path` order).
- `FileFinder::select(root)`:
  - no matches -> `No supported source files found for '<spec>'`, exit 1.
- `OutputResolver::new(base_dir)` / `resolve(value, default)`:
  - absolute stays absolute;
  - relative joins `base_dir`.

### `src/engine/converter.rs`

- `TextCodec` owns the encoding boundary:
  - `read`/`decode` reject unknown labels and invalid sequences;
  - `encode`/`write` reject unrepresentable output.
- `Converter::entities(path)`:
  - look up language by lowercase suffix;
  - read using the codec;
  - parse;
  - stamp `path` and `module = path.stem` recursively via `Entity::stamp`.
- `Converter::file(path)`.
- `Converter::project(paths)` (iterator-collected).
- `Converter::files(paths, output)`:
  - skip documents whose trimmed rendering is empty;
  - prepend `# <filename>\n\n`;
  - join with `\n\n`; append final `\n`;
  - write using the codec.
- `Converter::diagram(paths, output, kind)`:
  - `all` -> every kind; otherwise one kind;
  - write `render_all`.

### `src/cli/scaffold.rs`

- Create target directory recursively.
- Refuse overwrite unless `--force`:
  `Refusing to overwrite <path> (use --force).`, exit 1.
- Write exact `DEFAULT_CONFIG_TEXT`.

### `src/cli/parser.rs`

Extend the existing parser:

- Preserve all current behavior and tests.
- Add subcommand-aware specs.
- Support:
  - `-c`/`--config` global and per-subcommand;
  - `-o`/`--output`, attached and separate values;
  - `-k`/`--kind`, `--kind=all`;
  - `--force`;
  - `--version`;
  - `-h`/`--help`;
  - `--`.
- Subcommand `-c` overrides global config.

### `src/cli/mod.rs`

- `Cli` owns process arguments and argparse-compatible dispatch.
- `convert [input] [-o OUTPUT] [-c CONFIG]`
- `diagram [input] [-o OUTPUT] [-k KIND] [-c CONFIG]`
- `init [target] [--force]`
- No args -> print help, exit 0.
- No subcommand but other args -> inject `convert` unless help/version present.
- Print `Generated: <path>` / `Wrote: <path>`.
- Map known failures to stderr + exit code 1:
  - missing config;
  - no source files;
  - refusing overwrite.

### `src/cli/command.rs`

- `Command` trait: `name`, `usage`, `help`, `fields`, `run`.
- `ConvertCommand`, `DiagramCommand`, `InitCommand` own their metadata and
  execution; `Context` supplies config loading and output resolution.
- `COMMANDS` statically registers every command for lookup and help order.

### `src/main.rs` / `src/lib.rs`

- `lib.rs` exposes modules for tests.
- `main.rs` calls `cli::run` and maps errors to `ExitCode`.

### Tests

- `collect_files`: explicit file, directory, glob, absolute pattern, multiple
  base candidates, unsupported extensions, hidden files.
- Config discovery and output resolution.
- CLI integration using `env!("CARGO_BIN_EXE_dozen")` + `tempfile`:
  - convert;
  - diagram single kind and `-k all`;
  - init and `--force`;
  - injected `convert`;
  - global vs subcommand `-c`;
  - missing config;
  - no matching files.

---

## Milestone 8 — Parity cleanup

1. Run all golden tests and fix diffs.
2. Verify self-hosted generation:

   ```bash
   cargo run -- -c docs/dox-contract.cfg convert
   cargo run -- -c docs/dox-contract.cfg diagram -k all
   ```

   Compare `docs/dox-contract.md` and `docs/dox-contract-graph.md`.
3. Run quality gates:

   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. Decide whether to keep `docs/dox.py` in place as the reference oracle. Recommended:
   keep it until the port has shipped; it is the source of truth for future
   behavior changes.

---

## Behavioral parity checklist

### DocParser

- [x] `@brief`, `@details`, `@param[in]`, `@return`, `@throws`, `@see`,
      `@note`, `@warning`, `@example`
- [x] `@` and `\` prefixes; case-insensitive tags
- [x] aliases: return/returns, exception/raises -> throws
- [x] leading paragraphs -> brief/details
- [x] continuation lines and blank-line rules
- [x] unknown tags fall back to brief
- [x] direction defaults to `in`
- [x] list state quirks preserved

### Rust

- [x] `///` and `/** */`
- [x] fn/struct/enum/trait/impl
- [x] impl merging and implements list
- [x] fields, variants, methods
- [x] abstract methods (no `{` in block)
- [x] calls and includes
- [x] argument splitting and generic signatures

### C++

- [x] functions, `Class::method`, class/struct/enum
- [x] visibility sections and default visibility
- [x] static fields, types, descriptions
- [x] abstract methods, bases
- [x] includes, calls, parameter types
- [x] enum variants

### Python

- [x] module/class/function docstrings with `cleandoc`
- [x] imports and relative imports
- [x] annotations and return types
- [x] signatures excluding `self`/`cls`
- [x] class bases, enum detection, abstract detection
- [x] fields from annotations and `__init__`
- [x] inferred field types and visibility
- [x] method filtering, abstract decorators
- [x] calls with BFS/pruning and name reduction
- [x] syntax-error fallback to empty entities
- [x] unparse parity with `ast.unparse`

### Reference renderer

- [x] entity separators
- [x] function/class/enum/module layouts
- [x] tables and `|` escaping
- [x] method contracts
- [x] examples, notes, warnings, see-also
- [x] empty-output filtering

### Mermaid

- [x] all seven kinds and `render_all`
- [x] IDs, labels, duplicate suffixes
- [x] class blocks, stereotypes, fields, methods, variants
- [x] inheritance/realization/dependency edges
- [x] collaboration edges
- [x] call/caller edges and inverse direction
- [x] include resolution and external nodes
- [x] directory graph
- [x] label escaping

### Config / files / CLI

- [x] defaults and `ini.cfg` discovery
- [x] explicit config and `base_dir`
- [x] section/key parsing quirks
- [x] glob/base-candidate behavior
- [x] directory recursion
- [x] sort order
- [x] output resolution and parent creation
- [x] convert/diagram/init
- [x] injected `convert`
- [x] global vs subcommand `-c`
- [x] error messages and exit codes
- [x] `--version` prints `doxygen2md 0.5.0`

---

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Ruff codegen differs from `ast.unparse` | Spike in milestone 5; targeted unparser fallback; self-hosted `dox.py` golden test. |
| Python BFS ordering differences | Implement queue-based traversal with identical child order; field/import ordering tests. |
| `inspect.cleandoc` differences | Port the algorithm directly; docstring-heavy fixture. |
| Glob/path ordering differences | Explicit glob options; string sort; discovery tests. |
| Regex dialect differences | Patterns operate on single lines; test every pattern against fixtures. |
| `id(entity)` semantics | Stable node indices from flattening order. |
| Absolute paths in golden diagrams | Normalize repo root to `<ROOT>` in test helper. |
| Ruff crate API churn | Pin exact versions; isolate Python parsing behind `lang/python.rs`. |
| Non-UTF8 source files | `encoding_rs` with strict error handling; UTF-8 fast path. |

---

## Definition of done

- `cargo test` passes, including byte-for-byte golden tests for:
  - `animal.py`, `animal.rs`, C++ fixture;
  - `dox.py` itself across `convert` and all seven diagram kinds.
- `dozen -c docs/dox-contract.cfg convert` reproduces `docs/dox-contract.md`.
- `dozen -c docs/dox-contract.cfg diagram -k all` reproduces
  `docs/dox-contract-graph.md` (path-normalized if needed).
- No Python runtime is required for the Rust binary or its tests.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.

# dox.py

Turn Doxygen-style source comments into a uniform Markdown API contract.

Pipeline of replaceable parts:

| Stage | Type | Responsibility |
|-------|------|----------------|
| Model | `Entity`, `Field`, `Doc`, `Param`, `Variant` | Language-agnostic documentation tree. |
| Parse | `DocParser` | Read `@brief`, `@param[in]`, `@return`, and related tags. |
| Language | `Language` subclasses | Discover declarations, fields, and relations. |
| Registry | `LANGUAGES` | Map file extensions to language plugins. |
| Render | `ReferenceRenderer`, `MermaidRenderer` | Emit Markdown references or Mermaid diagrams. |
| Drive | `Converter`, `Scaffolder`, `main` | Read files, write output, and scaffold config. |

Adding a language: subclass `Language`, set `extensions`, implement `parse()`.
Adding a tag: add a branch in `DocParser.parse`.

### Example

```
python dox.py -c ini.cfg convert
python dox.py -c ini.cfg diagram -k all -o DIAGRAM.md
python dox.py -c ini.cfg init docs
```

---

## `Config` Class Reference

Settings that control which files are read and where output is written.

The `[defaults]` section of `ini.cfg` recognizes four keys:
`glob`, `output`, `diagram`, and `encoding`. `base_dir` is derived from the
loaded config file and resolves relative output and diagram paths.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `glob` |  |
| public | `Path` | `output` |  |
| public | `Path` | `diagram` |  |
| public | `str` | `encoding` |  |
| public | `Path` | `base_dir` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `'Config'` | `def defaults() -> 'Config'`<br>Build the built-in default configuration. |
| `'Config'` | `def load(path: Path \| None) -> 'Config'`<br>Load configuration from an explicit path or the current directory. |

### Method Contracts

#### `defaults()`

**Returns:** A `Config` equivalent to parsing `DEFAULT_CONFIG_TEXT`.

#### `load()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `path` | `Path \| None` | Explicit configuration file, or `None` to discover `ini.cfg` in the current directory. |

**Returns:** The resolved `Config`.

**Exceptions**

| Exception | Description |
|-----------|-------------|
| `SystemExit` | When an explicit `path` is not a file. |

---

## `parse_config()`

```
def parse_config(text: str, base_dir: Path | None) -> Config
```

Parse INI-style configuration text.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `text` | `str` | Configuration file contents. |
| [in] | `base_dir` | `Path \| None` | Directory used to resolve relative paths. |

### Returns

A populated `Config`.

---

## `EntityKind` Enum Reference

Kinds of declaration the documentation model can represent.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `MODULE` |  |
| `FUNCTION` |  |
| `METHOD` |  |
| `CLASS` |  |
| `STRUCT` |  |
| `ENUM` |  |
| `TRAIT` |  |
| `INTERFACE` |  |

---

## `Param` Class Reference

A documented parameter or field.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `name` |  |
| public | `str` | `type` |  |
| public | `str` | `direction` |  |
| public | `str` | `description` |  |

---

## `Variant` Class Reference

One enumerator or enum case.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `name` |  |
| public | `str` | `description` |  |

---

## `Field` Class Reference

One data member of a class, struct, or interface.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `name` |  |
| public | `str` | `type` |  |
| public | `str` | `visibility` |  |
| public | `bool` | `static` |  |
| public | `str` | `description` |  |

---

## `Doc` Class Reference

Structured documentation extracted from a comment block.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `brief` |  |
| public | `str` | `details` |  |
| public | `list[Param]` | `params` |  |
| public | `str` | `returns` |  |
| public | `list[tuple[str, str]]` | `throws` |  |
| public | `list[str]` | `see` |  |
| public | `list[str]` | `notes` |  |
| public | `list[str]` | `warnings` |  |
| public | `list[str]` | `examples` |  |

---

## `Entity` Class Reference

One documented declaration and its optional children.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `EntityKind` | `kind` |  |
| public | `str` | `name` |  |
| public | `str` | `signature` |  |
| public | `str` | `return_type` |  |
| public | `Doc` | `doc` |  |
| public | `bool` | `abstract` |  |
| public | `list['Entity']` | `members` |  |
| public | `list[Variant]` | `variants` |  |
| public | `list[Field]` | `fields` |  |
| public | `list[str]` | `bases` |  |
| public | `list[str]` | `implements` |  |
| public | `str` | `module` |  |
| public | `str` | `path` |  |
| public | `int` | `line` |  |
| public | `list[str]` | `calls` |  |
| public | `list[str]` | `includes` |  |

---

## `DocParser` Class Reference

Convert a comment block into a structured `Doc` model.

Leading untagged text is split into paragraphs: first is `brief`,
the rest is `details`. Tags are case-insensitive and may use `@` or `\`.
Continuation lines extend the active tag; blank lines are preserved inside
`details` and `examples`.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Doc` | `def parse(text: str) -> Doc`<br>Parse Doxygen-style tags into a `Doc` model. |

### Method Contracts

#### `parse()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `text` | `str` | Raw docstring or comment text. |

**Returns:** A `Doc` with brief, details, parameters, returns, exceptions, see-also references, notes, warnings, and examples populated.

---

## `Language` Class Reference *abstract*

Base class for language plugins.

Subclasses set `extensions` and implement `parse()`. Extensions
are registered in `LANGUAGES` through `__init_subclass__`.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `tuple[str, ...]` | `extensions` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `list[Entity]` | abstract `def parse(source: str, filename: str) -> list[Entity]`<br>Discover documented declarations in a source file. |
| `int` | `def consume_block(lines: list[str], start: int) -> int`<br>Count the lines occupied by the block starting at `start`. |

### Method Contracts

#### `consume_block()`

Braces are tracked per character so one-line blocks
(`fn foo() {}`) are handled. Without braces, a `;` ends the block.

---

## `PythonLanguage` Class Reference

Parse Python modules, classes, and functions through `ast`.

**Inherits:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `list[Entity]` | `def parse(source: str, filename: str) -> list[Entity]`<br>Parse Python source text into entities. |

---

## `RustLanguage` Class Reference

Parse Rust items from `///` and `/** ... */` doc comments.

**Inherits:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `list[Entity]` | `def parse(source: str, filename: str) -> list[Entity]`<br>Parse Rust source text into entities. |

---

## `CppLanguage` Class Reference

Parse C++ declarations from `///` and `/** ... */` comments.

**Inherits:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `list[Entity]` | `def parse(source: str, filename: str) -> list[Entity]`<br>Parse C++ source text into entities. |

---

## `ReferenceRenderer` Class Reference

Render a parsed `Entity` tree as Markdown.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `str` | `def render(entities: list[Entity]) -> str`<br>Render top-level entities as one Markdown document. |

---

## `MermaidRenderer` Class Reference

Render parsed entities as Mermaid diagrams.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `str` | `def render(entities: list[Entity], kind: str) -> str`<br>Render entities using the selected diagram kind. |
| `str` | `def render_all(entities: list[Entity], kinds: Iterable[str] \| None) -> str`<br>Render several diagrams into one Markdown document. |
| `str` | `def class_diagram(entities: list[Entity]) -> str`<br>Render the full class diagram. |
| `str` | `def inheritance_diagram(entities: list[Entity]) -> str`<br>Render only the class inheritance tree. |
| `str` | `def collaboration_diagram(entities: list[Entity]) -> str`<br>Render which classes use which other classes. |
| `str` | `def call_graph(entities: list[Entity]) -> str`<br>Render caller to callee edges. |
| `str` | `def caller_graph(entities: list[Entity]) -> str`<br>Render callee to caller edges. |
| `str` | `def include_graph(entities: list[Entity]) -> str`<br>Render imports and includes between files. |
| `str` | `def directory_graph(entities: list[Entity]) -> str`<br>Render dependencies between directories. |

### Method Contracts

#### `render_all()`

Each diagram becomes a level-2 heading followed by a fenced
` ```mermaid ` block. This is the single-file layout produced by
`diagram -k all`.

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `entities` | `list[Entity]` | Top-level entities in source order. |
| [in] | `kinds` | `Iterable[str] \| None` | Diagram kinds to include, or `None` for every kind in `KINDS`. |

**Returns:** A complete Markdown document.

#### Example

```
md = MermaidRenderer().render_all(entities)
Path("GRAPH.md").write_text(md)
```

---

## `Converter` Class Reference

Orchestrate file discovery, language parsing, and rendering.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `Config` | `cfg` |  |
| public | `` | `renderer` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `None` | `def __init__(cfg: Config) -> None` |
| `list[Entity]` | `def entities(path: Path) -> list[Entity]`<br>Parse one source file into entities. |
| `str` | `def file(path: Path) -> str`<br>Convert one source file to Markdown. |
| `list[Entity]` | `def project(paths: Iterable[Path]) -> list[Entity]`<br>Parse several source files into one entity list. |
| `None` | `def diagram(paths: Iterable[Path], output: Path, kind: str) -> None`<br>Convert several source files into one Markdown diagram file. |
| `None` | `def files(paths: Iterable[Path], output: Path) -> None`<br>Convert several source files into one Markdown file. |

### Method Contracts

#### `diagram()`

The output contains one fenced ` ```mermaid ` block per
selected kind. `kind="all"` selects every kind in
`MermaidRenderer.KINDS`.

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `paths` | `Iterable[Path]` | Source files to convert. |
| [in] | `output` | `Path` | Destination Markdown file. |
| [in] | `kind` | `str` | Diagram kind, or `"all"`. |

**Returns:** Nothing; output is written using `self.cfg.encoding`.

---

## `Scaffolder` Class Reference

Create a starter `ini.cfg` in a target directory.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `Path` | `target` |  |
| public | `bool` | `force` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `None` | `def __init__(target: Path, force: bool) -> None` |
| `list[Path]` | `def run() -> list[Path]`<br>Write the default configuration file. |

---

## `build_parser()`

```
def build_parser() -> argparse.ArgumentParser
```

Build the command-line argument parser.

---

## `_candidate_bases()`

```
def _candidate_bases(base_dir: Path | None) -> list[Path]
```

Build ordered directories for resolving relative glob patterns.

---

## `collect_files()`

```
def collect_files(root: Path | str, base_dir: Path | None) -> list[Path]
```

Expand a file, directory, or glob pattern into source files.

---

## `_resolve_output()`

```
def _resolve_output(value: Path | None, default: Path, base_dir: Path) -> Path
```

Choose an output path and resolve it against the config directory.

---

## `_select_files()`

```
def _select_files(args: argparse.Namespace, cfg: Config) -> list[Path]
```

Resolve the input or config glob into source files.

---

## `cmd_convert()`

```
def cmd_convert(args: argparse.Namespace) -> None
```

Execute the `convert` subcommand.

---

## `cmd_diagram()`

```
def cmd_diagram(args: argparse.Namespace) -> None
```

Execute the `diagram` subcommand.

### Details

`--kind all` writes every supported diagram into one Markdown
file as separate ` ```mermaid ` code blocks. A single kind writes just
that diagram, also wrapped in a fenced block. The output path comes from
`-o`, or from the config `diagram` key.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `args` | `argparse.Namespace` | Parsed arguments containing `input`, `config`, `output`, and `kind`. |

### Returns

Nothing; the Markdown file is written as a side effect.

### Exceptions

| Exception | Description |
|-----------|-------------|
| `SystemExit` | When no supported source files are found. |

---

## `cmd_init()`

```
def cmd_init(args: argparse.Namespace) -> None
```

Execute the `init` subcommand.

---

## `main()`

```
def main() -> None
```

Run the command-line interface.

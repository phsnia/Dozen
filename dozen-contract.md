# attribute.rs

## `Visibility` Enum Reference

! Declaration attributes shared by the model and the renderers. Effective member visibility.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Public` | Visible to every caller. |
| `Protected` | Visible to subclasses. |
| `Private` | Visible only inside the declaring type. |
| `Package` | Visible inside the declaring package. |

---

## `Modifiers` Struct Reference

Non-type declaration modifiers carried by a member.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `bool` | `r#static` | The member is declared `static`. |
| public | `bool` | `r#abstract` | The member has no concrete implementation. |

# node.rs

## `NodeId` Struct Reference

! Node identity shared by semantic tables and graphs. Stable index-based identity for a declaration inside one analysis pass.

Python's renderer used `id(entity)`; the port uses the insertion order of a node list so identifiers stay deterministic across runs.

**Implements:** `From<usize>`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(index: usize) -> Self`<br>Create an identifier for the node at `index`. |
| `usize` | `fn index(self) -> usize`<br>Return the zero-based insertion index. |
| `Self` | `fn from(index: usize) -> Self` |

# types.rs

## `TypeRef` Struct Reference

One parsed source-level type expression.

The type system only needs three views of a type: the original spelling, the unqualified base name used for symbol lookup, and the identifier tokens that may refer to user-declared types.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `String` | `raw` | Original type text as written in the source. |
| private | `String` | `base` | Base name with generics and `::` qualification removed. |
| private | `Vec<String>` | `tokens` | Non-builtin identifier tokens referenced by the type. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn parse(raw: &str) -> Self`<br>Parse a raw type expression into its lookup and display views. |
| `&str` | `fn raw(&self) -> &str`<br>Return the type exactly as written in the source. |
| `&str` | `fn base(&self) -> &str`<br>Return the unqualified base name used for symbol lookup. |
| `&[String]` | `fn tokens(&self) -> &[String]`<br>Return the user-declared identifier tokens referenced by this type. |
| `String` | `fn display(&self) -> String`<br>Render the type for a Mermaid class-block field. |

# command.rs

## `Context` Struct Reference

Shared state available to every subcommand implementation.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Option<PathBuf>` | `global_config` | Configuration path supplied before the subcommand, if any. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(global_config: Option<PathBuf>) -> Self`<br>Create a context carrying a global configuration path. |
| `Result<Config>` | `fn load_config(&self, parsed: &Parser) -> Result<Config>`<br>Load configuration selected by subcommand or global options. |
| `&'a str` | `fn input_spec(&self, parsed: &'a Parser, config: &'a Config) -> &'a str`<br>Return the positional input or the configured glob. |
| `PathBuf` | `fn output_path(&self, parsed: &Parser, default: &Path, base_dir: &Path) -> PathBuf`<br>Resolve an output path against the configuration directory. |
| `Result<()>` | `fn prepare_output(&self, output: &Path) -> Result<()>`<br>Create missing output parent directories. |

---

## `Command` Trait Reference

Interface implemented by every CLI subcommand.

**Inherits:** `Sync`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static str` | abstract `fn name(&self) -> &'static str` |
| `&'static str` | abstract `fn usage(&self) -> &'static str` |
| `&'static str` | abstract `fn help(&self) -> &'static str` |
| `&'static [Field]` | abstract `fn fields(&self) -> &'static [Field]` |
| `Result<ExitCode>` | abstract `fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode>` |
| `Result<Parser>` | `fn parse(&self, rest: &[String]) -> Result<Parser>` |
| `String` | `fn error_program(&self) -> String` |
| `` | `fn print_help(&self)` |

---

## `all()`

```
fn all() -> &'static [&'static dyn Command]
```

Return every registered command in help-text order.

---

## `find()`

```
fn find(name: &str) -> Option<&'static dyn Command>
```

Find a registered command by its command-line name.

---

## `kind_choices()`

```
fn kind_choices() -> String
```

Render the argparse choice list for `--kind`.

---

## `parse_or_exit()`

```
fn parse_or_exit(rest: &[String], fields: &[Field], usage: &str) -> Result<Parser>
```

Parse arguments or terminate with argparse-style diagnostics.

# convert.rs

## `ConvertCommand` Struct Reference

`dozen convert` produces a Markdown API reference.

**Implements:** `Command`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static str` | `fn name(&self) -> &'static str` |
| `&'static str` | `fn usage(&self) -> &'static str` |
| `&'static str` | `fn help(&self) -> &'static str` |
| `&'static [Field]` | `fn fields(&self) -> &'static [Field]` |
| `Result<ExitCode>` | `fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode>` |

# diagram.rs

## `DiagramCommand` Struct Reference

`dozen diagram` produces one or more Mermaid graphs.

**Implements:** `Command`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static str` | `fn name(&self) -> &'static str` |
| `&'static str` | `fn usage(&self) -> &'static str` |
| `&'static str` | `fn help(&self) -> &'static str` |
| `&'static [Field]` | `fn fields(&self) -> &'static [Field]` |
| `Result<ExitCode>` | `fn run(&self, rest: &[String], context: &Context) -> Result<ExitCode>` |

# init.rs

## `InitCommand` Struct Reference

`dozen init` writes a starter `ini.cfg`.

**Implements:** `Command`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static str` | `fn name(&self) -> &'static str` |
| `&'static str` | `fn usage(&self) -> &'static str` |
| `&'static str` | `fn help(&self) -> &'static str` |
| `&'static [Field]` | `fn fields(&self) -> &'static [Field]` |
| `Result<ExitCode>` | `fn run(&self, rest: &[String], _context: &Context) -> Result<ExitCode>` |

# mod.rs

## `run()`

```
fn run() -> Result<ExitCode>
```

Run the CLI using the process arguments.

---

## `Cli` Struct Reference

Parsed process arguments plus the argparse-compatible dispatch rules.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Vec<String>` | `args` | Process arguments without the executable name. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn from_env() -> Self`<br>Collect the process arguments, skipping the executable name. |
| `Result<ExitCode>` | `fn execute(&self) -> Result<ExitCode>`<br>Execute the command line and return the process exit code. |
| `Option<usize>` | `fn command_index(&self) -> Option<usize>`<br>Position of the first recognised subcommand. |
| `Result<ExitCode>` | `fn execute_default(&self) -> Result<ExitCode>`<br>Apply argparse's injected default `convert` subcommand. |
| `Result<GlobalOutcome>` | `fn parse_global(&self, args: &[String]) -> Result<GlobalOutcome>`<br>Parse the global options that appear before the subcommand. |

---

## `GlobalOutcome` Enum Reference

Result of parsing the global option prefix.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Continue` | Continue with an optional global config path. |
| `Stop` | Help, version, or a usage error already printed. |

---

## `dispatch()`

```
fn dispatch(args: &[String]) -> Result<(&'static dyn command::Command, &[String])>
```

Resolve the subcommand at the front of `args`.

---

## `run_command()`

```
fn run_command(command: &dyn command::Command, rest: &[String], global_config: Option<PathBuf>) -> Result<ExitCode>
```

Print help for a subcommand, or run it with the shared context.

---

## `is_help()`

```
fn is_help(arg: &str) -> bool
```

True for the two spellings of `argparse`'s help flag.

---

## `print_help()`

```
fn print_help(text: &str)
```

Print text to stdout, matching `argparse`'s help output stream.

---

## `print_version()`

```
fn print_version()
```

Print the `--version` line.

---

## `usage_error()`

```
fn usage_error(usage: &str, program: &str, message: &str) -> ExitCode
```

Print an argparse-style usage error and return exit code 2.

# parser.rs

## `Kind` Enum Reference

The kind of argument a [`Field`] describes.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Flag` | A boolean flag that takes no value. |
| `Opt` | An option that consumes one value. |
| `Pos` | A positional value. |

---

## `Field` Struct Reference

One accepted argument, declared statically by a command.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `&'static str` | `long` | Long name without the leading dashes. |
| public | `Option<char>` | `short` | Optional single-character short name. |
| public | `Kind` | `kind` | Argument kind. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn option(long: &'static str, short: Option<char>) -> Self`<br>Declare an option with an optional short name. |
| `Self` | `fn flag(long: &'static str) -> Self`<br>Declare a boolean flag. |
| `Self` | `fn short_flag(long: &'static str, short: char) -> Self`<br>Declare a boolean flag with a short name. |
| `Self` | `fn positional(long: &'static str) -> Self`<br>Declare a positional argument. |
| `bool` | `fn accepts_long(&self, name: &str) -> bool`<br>True when this field accepts the long option name `name`. |
| `bool` | `fn accepts_short(&self, name: char) -> bool`<br>True when this field accepts the short option name `name`. |

---

## `Parser` Struct Reference

Parsed arguments grouped by how they were declared.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `Vec<String>` | `flags` | Long names of flags that were present. |
| public | `HashMap<String, String>` | `opts` | Long name to value for options that were present. |
| public | `Vec<String>` | `positional` | Positional values in source order. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `bool` | `fn flag(&self, name: &str) -> bool`<br>Returns true when flag `name` was provided. |
| `Option<&str>` | `fn opt(&self, name: &str) -> Option<&str>`<br>Returns the value of option `name`, if it was provided. |
| `Option<&str>` | `fn str(&self) -> Option<&str>`<br>Returns the first positional argument, if any. |
| `Result<Self, String>` | `fn parse(args: impl Iterator<Item = String>, spec: &[Field]) -> Result<Self, String>`<br>Parse `args` according to `spec`. |
| `Result<(), String>` | `fn parse_long(&mut self, long: &str, args: &mut impl Iterator<Item = String>, spec: &[Field]) -> Result<(), String>`<br>Parse one `--name[=value]` argument. |
| `Result<(), String>` | `fn parse_short(&mut self, short: &str, args: &mut impl Iterator<Item = String>, spec: &[Field]) -> Result<(), String>`<br>Parse one cluster of short options such as `-va` or `-oout.md`. |
| `` | `fn push_flag(&mut self, long: &str)`<br>Record a flag once, even when it is repeated. |

# scaffold.rs

## `Scaffolder` Struct Reference

Writes the default configuration file, optionally overwriting one.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `PathBuf` | `target` | Directory that will contain `ini.cfg`. |
| private | `bool` | `force` | Whether an existing file may be overwritten. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(target: impl Into<PathBuf>, force: bool) -> Self`<br>Create a scaffolder for `target`; `force` allows overwriting. |
| `Result<Vec<PathBuf>>` | `fn run(&self) -> Result<Vec<PathBuf>>`<br>Write the default configuration file and report the written paths. |
| `&Path` | `fn target(&self) -> &Path`<br>Return the target directory this scaffolder was created for. |
| `PathBuf` | `fn config_path(&self) -> PathBuf`<br>`Path(".") / "ini.cfg"` in Python is just `Path("ini.cfg")`. |

---

## `normalize()`

```
fn normalize(path: &Path) -> PathBuf
```

Approximate `pathlib.Path`'s normalization: drop `.` components.

# mod.rs

## `Tag` Enum Reference

A recognised documentation tag.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Brief` | `@brief` — short summary. |
| `Details` | `@details` — long description. |
| `Param` | `@param[in]` — parameter documentation. |
| `Returns` | `@return` / `@returns` — return value documentation. |
| `Throws` | `@throws` / `@exception` / `@raises` — exception documentation. |
| `See` | `@see` — cross-reference list entry. |
| `Note` | `@note` — note list entry. |
| `Warning` | `@warning` — warning list entry. |
| `Example` | `@example` — example block. |
| `Unknown` | Any other tag; ignored without changing the active field. |

---

## `ListKind` Enum Reference

A repeated documentation list carried by [`Doc`].

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `See` | `@see` references. |
| `Note` | `@note` notes. |
| `Warning` | `@warning` warnings. |
| `Example` | `@example` examples. |

---

## `DocBuilder` Struct Reference

Builder state for one tagged documentation block.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Doc` | `doc` | Document assembled so far. |
| private | `Option<Tag>` | `active` | Tag whose continuation lines are currently being consumed. |
| private | `Option<usize>` | `param` | Index of the parameter receiving continuation lines. |
| private | `Option<usize>` | `throw` | Index of the exception receiving continuation lines. |
| private | `Option<(ListKind, usize)>` | `list` | Current list and the index receiving continuation lines. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(leading: &[&str]) -> Self`<br>Seed the builder with the untagged leading paragraphs. |
| `` | `fn consume(&mut self, raw: &str)`<br>Consume one tagged line. |
| `` | `fn blank_line(&mut self)`<br>Handle a blank line inside a details block or example. |
| `` | `fn begin_tag(&mut self, captures: &regex::Captures<'_>)`<br>Start a new tag, seeding the selected field with its inline value. |
| `` | `fn continuation(&mut self, raw: &str, line: &str)`<br>Append a plain continuation line to the active field. |
| `` | `fn append_unstructured(&mut self, raw: &str, line: &str)`<br>Handle text that either extends the active list or the brief summary. |
| `usize` | `fn push_param(&mut self, value: &str, direction: &str) -> usize`<br>Push a parameter and return its index. |
| `usize` | `fn push_throw(&mut self, value: &str) -> usize`<br>Push an exception and return its index. |
| `usize` | `fn push_list(&mut self, kind: ListKind, value: &str) -> usize`<br>Push a list entry and return its index. |
| `&mut Vec<String>` | `fn list_mut(&mut self, kind: ListKind) -> &mut Vec<String>`<br>Borrow one of the repeated lists by kind. |
| `Doc` | `fn finish(mut self) -> Doc`<br>Finish the document by trimming the fields `dox.py` trims. |
| `Vec<String>` | `fn paragraphs(lines: &[&str]) -> Vec<String>`<br>Split untagged lines into whitespace-collapsed paragraphs. |

---

## `split_lines()`

```
fn split_lines(text: &str) -> Vec<&str>
```

Split text the way Python's `str.splitlines` does.

---

## `expand_tabs()`

```
fn expand_tabs(text: &str) -> String
```

Expand tabs using Python's default tab size and tab stops.

---

## `cleandoc()`

```
fn cleandoc(text: &str) -> String
```

Equivalent of Python's `inspect.cleandoc`.

---

## `DocParser` Struct Reference

Convert a comment block into a structured [`Doc`] model.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Doc` | `fn parse(&self, text: &str) -> Doc`<br>Parse one comment block into its brief, details, and tagged fields. |

---

## `partition_space()`

```
fn partition_space(value: &str) -> (&str, &str, &str)
```

Split `value` on the first ASCII space, mirroring Python's `partition(" ")`.

# parser.rs

## `DocParser` Struct Reference

Convert a comment block into a structured [`Doc`] model.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Doc` | `fn parse(&self, text: &str) -> Doc`<br>Parse one comment block into its brief, details, and tagged fields. |

# tags.rs

## `Tag` Enum Reference

A recognised documentation tag.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Brief` | `@brief` — short summary. |
| `Details` | `@details` — long description. |
| `Param` | `@param[in]` — parameter documentation. |
| `Returns` | `@return` / `@returns` — return value documentation. |
| `Throws` | `@throws` / `@exception` / `@raises` — exception documentation. |
| `See` | `@see` — cross-reference list entry. |
| `Note` | `@note` — note list entry. |
| `Warning` | `@warning` — warning list entry. |
| `Example` | `@example` — example block. |
| `Unknown` | Any other tag; ignored without changing the active field. |

---

## `ListKind` Enum Reference

A repeated documentation list carried by [`Doc`].

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `See` | `@see` references. |
| `Note` | `@note` notes. |
| `Warning` | `@warning` warnings. |
| `Example` | `@example` examples. |

---

## `TagLine` Struct Reference

One parsed `@tag`, including its optional direction and inline value.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `Tag` | `tag` | Resolved tag. |
| public | `&'a str` | `direction` | Direction capture, defaulting to `in`. |
| public | `&'a str` | `value` | Inline text after the tag, already trimmed. |

---

## `DocBuilder` Struct Reference

Builder state for one tagged documentation block.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Doc` | `doc` | Document assembled so far. |
| private | `Option<Tag>` | `active` | Tag whose continuation lines are currently being consumed. |
| private | `Option<usize>` | `param` | Index of the parameter receiving continuation lines. |
| private | `Option<usize>` | `throw` | Index of the exception receiving continuation lines. |
| private | `Option<(ListKind, usize)>` | `list` | Current list and the index receiving continuation lines. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(leading: &[&str]) -> Self`<br>Seed the builder with the untagged leading paragraphs. |
| `` | `fn consume(&mut self, raw: &str)`<br>Consume one tagged line. |
| `` | `fn blank_line(&mut self)`<br>Handle a blank line inside a details block or example. |
| `` | `fn begin_tag(&mut self, tag_line: &TagLine<'_>)`<br>Start a new tag, seeding the selected field with its inline value. |
| `` | `fn continuation(&mut self, raw: &str, line: &str)`<br>Append a plain continuation line to the active field. |
| `` | `fn append_unstructured(&mut self, raw: &str, line: &str)`<br>Handle text that either extends the active list or the brief summary. |
| `usize` | `fn push_param(&mut self, value: &str, direction: &str) -> usize`<br>Push a parameter and return its index. |
| `usize` | `fn push_throw(&mut self, value: &str) -> usize`<br>Push an exception and return its index. |
| `usize` | `fn push_list(&mut self, kind: ListKind, value: &str) -> usize`<br>Push a list entry and return its index. |
| `&mut Vec<String>` | `fn list_mut(&mut self, kind: ListKind) -> &mut Vec<String>`<br>Borrow one of the repeated lists by kind. |
| `Doc` | `fn finish(mut self) -> Doc`<br>Finish the document by trimming the fields `dox.py` trims. |
| `Vec<String>` | `fn paragraphs(lines: &[&str]) -> Vec<String>`<br>Split untagged lines into whitespace-collapsed paragraphs. |
| `(&str, &str, &str)` | `fn partition_space(value: &str) -> (&str, &str, &str)`<br>Split `value` on the first ASCII space, mirroring Python's `partition`. |

# config.rs

## `Config` Struct Reference

Settings that control which files are read and where output is written.

**Implements:** `Default`

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `glob` | Input file, directory, or glob pattern. |
| public | `PathBuf` | `output` | Default Markdown reference output path. |
| public | `PathBuf` | `diagram` | Default Mermaid diagram output path. |
| public | `String` | `encoding` | Character encoding used for both reading and writing. |
| public | `PathBuf` | `base_dir` | Directory used to resolve relative globs and output paths. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn default() -> Self` |
| `Self` | `fn defaults() -> Self`<br>Build the built-in default configuration. |
| `Result<Self>` | `fn load(path: Option<&Path>) -> Result<Self>`<br>Load configuration from an explicit path or the current directory. |
| `Self` | `fn parse(text: &str, base_dir: Option<&Path>) -> Self`<br>Parse INI-style configuration text. |

### Method Contracts

#### `load()`

An explicit path must exist. Without one, `./ini.cfg` is used when present; otherwise the built-in defaults apply.

---

## `ConfigBuilder` Struct Reference

Mutable state while scanning a configuration file.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Config` | `config` | Configuration assembled so far. |
| private | `Option<String>` | `section` | Current section name, lowercased. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(base_dir: Option<&Path>) -> Self`<br>Create a builder seeded with the built-in defaults. |
| `` | `fn apply(&mut self, raw: &str)`<br>Apply one raw line: a section header, a key/value pair, or noise. |
| `` | `fn set(&mut self, key: &str, value: &str)`<br>Assign one recognised key in the active `[defaults]` section. |
| `Config` | `fn finish(self) -> Config`<br>Finish scanning and return the populated configuration. |

---

## `python_path()`

```
fn python_path(value: &str) -> PathBuf
```

`Path("")` in Python is `Path(".")`.

# converter.rs

## `TextCodec` Struct Reference

Character encoding boundary for all files the tool reads or writes.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `String` | `encoding` | Encoding label used for both directions. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(encoding: impl Into<String>) -> Self`<br>Create a codec for `encoding`, a label such as `utf-8` or `latin-1`. |
| `Result<String>` | `fn read(&self, path: &Path) -> Result<String>`<br>Read and decode a UTF-8 path according to the configured encoding. |
| `Result<String>` | `fn decode(&self, bytes: &[u8]) -> Result<String>`<br>Decode bytes, rejecting unknown labels and invalid sequences. |
| `Result<Vec<u8>>` | `fn encode(&self, text: &str) -> Result<Vec<u8>>`<br>Encode text for output, rejecting unknown labels and unrepresentable text. |
| `Result<()>` | `fn write(&self, path: &Path, text: &str) -> Result<()>`<br>Encode and write `text`, creating no directories on its own. |

---

## `encoding_for()`

```
fn encoding_for(label: &str) -> Result<&'static Encoding>
```

Resolve an encoding label with Python-compatible error reporting.

---

## `Converter` Struct Reference

The conversion pipeline: source files in, Markdown or Mermaid out.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `MarkdownRenderer` | `markdown` | Markdown reference renderer. |
| private | `MermaidRenderer` | `mermaid` | Mermaid diagram renderer. |
| private | `TextCodec` | `codec` | Encoding boundary for source and output files. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(cfg: Config) -> Self`<br>Create a converter using the encoding in `cfg`. |
| `Result<Vec<Entity>>` | `fn entities(&self, path: &Path) -> Result<Vec<Entity>>`<br>Parse one source file into entities. |
| `Result<String>` | `fn file(&self, path: &Path) -> Result<String>`<br>Convert one source file to Markdown. |
| `Result<Vec<Entity>>` | `fn project(&self, paths: &[PathBuf]) -> Result<Vec<Entity>>`<br>Parse several source files into one entity list. |
| `Result<()>` | `fn diagram(&self, paths: &[PathBuf], output: &Path, kind: &str) -> Result<()>`<br>Convert several source files into one Markdown diagram file. |
| `Result<()>` | `fn files(&self, paths: &[PathBuf], output: &Path) -> Result<()>`<br>Convert several source files into one Markdown file. |
| `Result<Option<String>>` | `fn document(&self, path: &Path) -> Result<Option<String>>`<br>Render one file as a titled document, or `None` when it has no content. |

---

## `display_name()`

```
fn display_name(path: &Path) -> String
```

Use the file name when present, otherwise the full path.

# files.rs

## `match_options()`

```
fn match_options() -> MatchOptions
```

Match semantics used by `dox.py`: `*` never crosses `/`, and dot files are only matched explicitly.

### Returns

Match options matching Python-style glob behaviour.

---

## `FileFinder` Struct Reference

Expand a file, directory, or glob pattern into supported source files.

Relative patterns are tried against configured and working
directories in contract order. Absolute patterns are searched directly.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `Option<&'a Path>` | `base_dir` | Directory used to resolve relative patterns.  @note `None` disables configured-base candidate lookup. |

---

## `OutputResolver` Struct Reference

Resolve an output path against the configuration directory.

Absolute paths remain unchanged; relative paths are joined to the
configured base directory.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `&'a Path` | `base_dir` | Directory that relative outputs are joined to.  @note This is normally the directory containing `ini.cfg`. |

# call_graph.rs

## `CallEdge` Struct Reference

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `caller` |  |
| public | `String` | `callee` |  |

---

## `edges()`

```
fn edges(entities: &[Entity]) -> Vec<CallEdge>
```

# dependency.rs

## `Dependency` Struct Reference

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `owner` |  |
| public | `String` | `dependency` |  |

---

## `dependencies()`

```
fn dependencies(entities: &[Entity]) -> Vec<Dependency>
```

# inheritance.rs

## `Relation` Struct Reference

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `child` |  |
| public | `String` | `parent` |  |
| public | `bool` | `implementation` |  |

---

## `relations()`

```
fn relations(entities: &[Entity]) -> Vec<Relation>
```

# mod.rs

## `Edge` Struct Reference

A directed graph edge with an optional label.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `T` | `from` |  |
| public | `T` | `to` |  |
| public | `Option<String>` | `label` |  |

# bash.rs

## `BashLanguage` Struct Reference

Parse Bash functions and module metadata through Tree-sitter.

**Implements:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static [&'static str]` | `fn extensions(&self) -> &'static [&'static str]`<br>Return Bash source extensions. |
| `Vec<Entity>` | `fn parse(&self, source: &str, filename: &str) -> Vec<Entity>`<br>Parse one Bash source file. |

### Method Contracts

#### `extensions()`

**Returns:** `.sh` and `.bash` extensions.

#### `parse()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Bash source text. |
| [in] | `filename` | `` | Source filename used as the module name. |

**Returns:** Contract entities, or an empty list for invalid syntax.

---

## `text_at()`

```
fn text_at(source: &str, node: &Node<'_>) -> String
```

---

## `module_doc()`

```
fn module_doc(source: &str, _root: &Node<'_>) -> String
```

---

## `preceding_doc()`

```
fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String>
```

---

## `walk_functions()`

```
fn walk_functions(source: &str, node: &Node<'_>, entities: &mut Vec<Entity>)
```

---

## `includes()`

```
fn includes(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `collect_includes()`

```
fn collect_includes(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

---

## `calls()`

```
fn calls(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `walk_calls()`

```
fn walk_calls(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

# cpp.rs

## `CppLanguage` Struct Reference

Parse C++ declarations through Tree-sitter.

**Implements:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static [&'static str]` | `fn extensions(&self) -> &'static [&'static str]`<br>Return C++ source extensions. |
| `Vec<Entity>` | `fn parse(&self, source: &str, filename: &str) -> Vec<Entity>`<br>Parse one C++ source file. |

### Method Contracts

#### `extensions()`

**Returns:** Supported C++ header and implementation extensions.

#### `parse()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | C++ source text. |
| [in] | `filename` | `` | Source filename used as the module name. |

**Returns:** Contract entities, or an empty list for invalid syntax.

---

## `text_at()`

```
fn text_at(source: &str, node: &Node<'_>) -> String
```

---

## `clean_comment()`

```
fn clean_comment(text: &str) -> Vec<String>
```

---

## `preceding_doc()`

```
fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String>
```

---

## `doc_for()`

```
fn doc_for(source: &str, node: &Node<'_>) -> crate::model::Doc
```

---

## `walk_top_level()`

```
fn walk_top_level(source: &str, node: &Node<'_>, entities: &mut Vec<Entity>, by_name: &mut HashMap<String, usize>)
```

---

## `enum_entity()`

```
fn enum_entity(source: &str, node: &Node<'_>) -> Entity
```

---

## `class_entity()`

```
fn class_entity(source: &str, node: &Node<'_>) -> Entity
```

---

## `field_entity()`

```
fn field_entity(source: &str, node: &Node<'_>, visibility: Visibility) -> Option<Field>
```

---

## `node_name()`

```
fn node_name(source: &str, node: &Node<'_>) -> String
```

---

## `method_declaration()`

```
fn method_declaration(source: &str, node: &Node<'_>) -> Option<Entity>
```

---

## `declarator_parameter_text()`

```
fn declarator_parameter_text(source: &str, declarator: &Node<'_>) -> String
```

---

## `find_parameters()`

```
fn find_parameters(node: &Node<'a>) -> Option<Node<'a>>
```

---

## `parameter_types()`

```
fn parameter_types(source: &str, parameters: &Node<'_>) -> HashMap<String, String>
```

---

## `merge_parameter_types()`

```
fn merge_parameter_types(doc: &mut crate::model::Doc, types: &HashMap<String, String>)
```

---

## `function_entity()`

```
fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity
```

---

## `qualified_name()`

```
fn qualified_name(source: &str, declarator: &Node<'_>) -> String
```

---

## `merge_impl_methods()`

```
fn merge_impl_methods(source: &str, root: &Node<'_>, entities: &mut [Entity], by_name: &HashMap<String, usize>)
```

---

## `merge_methods_in_namespace()`

```
fn merge_methods_in_namespace(source: &str, root: &Node<'_>, entities: &mut [Entity], by_name: &HashMap<String, usize>)
```

---

## `base_names()`

```
fn base_names(raw: &str) -> Vec<String>
```

---

## `includes()`

```
fn includes(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `collect_includes()`

```
fn collect_includes(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

---

## `calls()`

```
fn calls(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `walk_calls()`

```
fn walk_calls(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

# mod.rs

## `Language` Trait Reference

A source-language plugin that lowers one file into the shared model.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static [&'static str]` | abstract `fn extensions(&self) -> &'static [&'static str]`<br>File extensions handled by this plugin, including the leading dot. |
| `Vec<Entity>` | abstract `fn parse(&self, source: &str, filename: &str) -> Vec<Entity>`<br>Parse `source`, using `filename` as the module entity's name. |

---

## `consume_block()`

```
fn consume_block(lines: &[&str], start: usize) -> usize
```

Count the lines occupied by the block starting at `start`.

### Details

Braces are tracked per character so one-line blocks (`fn foo() {}`) are handled. Without braces, a `;` ends the block.

---

## `read_block_comment()`

```
fn read_block_comment(lines: &[&str], start: usize) -> (Vec<String>, usize)
```

Read a `/** ... */` block starting at `start`.

### Details

Returns `([], 0)` when the line does not start a block comment.

---

## `body_slice()`

```
fn body_slice(lines: &'a [&'a str], start: usize, consumed: usize) -> &'a [&'a str]
```

Slice `lines[start + 1 .. start + consumed - 1]` safely.

# python.rs

## `PyExpr` Enum Reference

Rust-native expression subset used for Python annotations and call names.

This small AST avoids exposing Tree-sitter grammar nodes to renderers.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Raw` |  |
| `Name` |  |
| `Attribute` |  |
| `Subscript` |  |
| `Tuple` |  |
| `List` |  |
| `BinOp` |  |
| `String` |  |
| `Call` |  |

---

## `PythonLanguage` Struct Reference

Parse Python modules, classes, and functions through Tree-sitter.

**Implements:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static [&'static str]` | `fn extensions(&self) -> &'static [&'static str]`<br>Return Python source extensions. |
| `Vec<Entity>` | `fn parse(&self, source: &str, filename: &str) -> Vec<Entity>`<br>Parse one Python source file. |

### Method Contracts

#### `extensions()`

**Returns:** The `.py` extension.

#### `parse()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Python source text. |
| [in] | `filename` | `` | Source filename used as the module name. |

**Returns:** Contract entities, or an empty list for invalid syntax.

---

## `unwrap_decorated()`

```
fn unwrap_decorated(node: &Node<'a>) -> Node<'a>
```

Unwrap one decorated class or function definition.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `node` | `` | Tree-sitter decorated-definition node. |

### Returns

The underlying function or class node.

---

## `text_at()`

```
fn text_at(source: &str, node: &Node<'_>) -> String
```

Read the exact source text covered by a syntax node.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Complete source text. |
| [in] | `node` | `` | Tree-sitter syntax node. |

### Returns

Source text for the node.

---

## `module_docstring()`

```
fn module_docstring(source: &str, root: &Node<'_>) -> Option<String>
```

Extract and clean a module docstring.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Complete source text. |
| [in] | `root` | `` | Python module syntax root. |

### Returns

Cleaned module docstring, if present.

---

## `docstring()`

```
fn docstring(source: &str, body: &Node<'_>) -> Option<String>
```

Extract and clean the leading docstring of a declaration body.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Complete source text. |
| [in] | `body` | `` | Declaration body node. |

### Returns

Cleaned docstring, if present.

---

## `clean_string()`

```
fn clean_string(raw: &str) -> String
```

Remove Python string prefixes and quote delimiters.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `raw` | `` | Source spelling of a string literal. |

### Returns

String content suitable for `inspect.cleandoc`-style cleanup.

---

## `line_of()`

```
fn line_of(node: &Node<'_>) -> usize
```

Convert a Tree-sitter row to a one-based source line.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `node` | `` | Tree-sitter syntax node. |

### Returns

One-based source line.

---

## `function_entity()`

```
fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity
```

Lower a Python function or method into an entity.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Complete source text. |
| [in] | `node` | `` | Function-definition syntax node. |
| [in] | `kind` | `` | Function or method entity kind. |

### Returns

The lowered contract entity.

---

## `parameter_annotations()`

```
fn parameter_annotations(source: &str, parameters: Option<&Node<'_>>) -> HashMap<String, String>
```

---

## `parameter_name()`

```
fn parameter_name(source: &str, parameter: &Node<'_>) -> String
```

---

## `parameter_annotation()`

```
fn parameter_annotation(source: &str, parameter: &Node<'_>) -> Option<String>
```

---

## `unparse_annotation_text()`

```
fn unparse_annotation_text(text: &str) -> String
```

---

## `class_entity()`

```
fn class_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity
```

---

## `find_method()`

```
fn find_method(source: &str, class: &Node<'a>, wanted: &str) -> Option<Node<'a>>
```

---

## `is_enum_class()`

```
fn is_enum_class(source: &str, node: &Node<'_>) -> bool
```

---

## `is_abstract_class()`

```
fn is_abstract_class(source: &str, node: &Node<'_>) -> bool
```

---

## `is_abstract_method()`

```
fn is_abstract_method(source: &str, original: &Node<'_>) -> bool
```

---

## `class_bases()`

```
fn class_bases(source: &str, node: &Node<'_>) -> Vec<String>
```

---

## `enum_variants()`

```
fn enum_variants(source: &str, node: &Node<'_>) -> Vec<Variant>
```

---

## `collect_fields()`

```
fn collect_fields(source: &str, class: &Node<'_>, init: Option<&Node<'_>>) -> Vec<Field>
```

---

## `walk_fields()`

```
fn walk_fields(source: &str, node: &Node<'_>, hints: &HashMap<String, String>, found: &mut Vec<Field>, index_by_name: &mut HashMap<String, usize>)
```

---

## `self_attribute()`

```
fn self_attribute(source: &str, node: &Node<'_>) -> Option<String>
```

---

## `add_field()`

```
fn add_field(found: &mut Vec<Field>, index_by_name: &mut HashMap<String, usize>, name: &str, type_: &str, visibility: &str)
```

---

## `visibility()`

```
fn visibility(name: &str) -> &'static str
```

---

## `collect_imports()`

```
fn collect_imports(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `collect_import_nodes()`

```
fn collect_import_nodes(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

---

## `import_name()`

```
fn import_name(source: &str, node: &Node<'_>) -> String
```

---

## `collect_calls()`

```
fn collect_calls(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `walk_calls()`

```
fn walk_calls(source: &str, node: &Node<'_>, depth: usize, out: &mut Vec<String>)
```

---

## `call_name()`

```
fn call_name(source: &str, node: &Node<'_>) -> String
```

---

## `node_name()`

```
fn node_name(source: &str, node: &Node<'_>) -> String
```

---

## `unparse_text()`

```
fn unparse_text(source: &str, node: &Node<'_>) -> String
```

---

## `parse_expr()`

```
fn parse_expr(source: &str, node: &Node<'_>) -> PyExpr
```

---

## `render_expr()`

```
fn render_expr(expr: &PyExpr) -> String
```

---

## `unparse_string()`

```
fn unparse_string(raw: &str) -> String
```

# registry.rs

## `LanguageRegistry` Struct Reference

Maps file paths to the language plugin that can parse them.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new() -> Self`<br>Create a stateless registry. |
| `String` | `fn suffix(&self, path: &Path) -> String`<br>Return the lowercase extension including the leading dot. |
| `bool` | `fn is_supported(&self, path: &Path) -> bool`<br>True when the tool has a parser for `path`. |
| `Option<Box<dyn Language>>` | `fn language_for(&self, path: &Path) -> Option<Box<dyn Language>>`<br>Return the parser for `path`, if the extension is supported. |

# rust.rs

## `RustLanguage` Struct Reference

Parse Rust items from documentation comments and Tree-sitter syntax nodes.

**Implements:** `Language`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'static [&'static str]` | `fn extensions(&self) -> &'static [&'static str]`<br>Return Rust source extensions. |
| `Vec<Entity>` | `fn parse(&self, source: &str, filename: &str) -> Vec<Entity>`<br>Parse one Rust source file. |

### Method Contracts

#### `extensions()`

**Returns:** The `.rs` extension.

#### `parse()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Rust source text. |
| [in] | `filename` | `` | Source filename used as the module name. |

**Returns:** Contract entities, or an empty list for invalid syntax.

---

## `text_at()`

```
fn text_at(source: &str, node: &Node<'_>) -> String
```

---

## `node_name()`

```
fn node_name(source: &str, node: &Node<'_>) -> String
```

---

## `preceding_doc()`

```
fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String>
```

---

## `clean_line_comment()`

```
fn clean_line_comment(text: &str) -> String
```

---

## `clean_block_comment()`

```
fn clean_block_comment(text: &str) -> Vec<String>
```

---

## `doc_for()`

```
fn doc_for(source: &str, node: &Node<'_>) -> crate::model::Doc
```

---

## `visibility()`

```
fn visibility(node: &Node<'_>) -> Visibility
```

---

## `struct_entity()`

```
fn struct_entity(source: &str, node: &Node<'_>) -> Entity
```

---

## `enum_entity()`

```
fn enum_entity(source: &str, node: &Node<'_>) -> Entity
```

---

## `trait_entity()`

```
fn trait_entity(source: &str, node: &Node<'_>) -> Entity
```

---

## `function_entity()`

```
fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity
```

---

## `parameter_text()`

```
fn parameter_text(source: &str, parameters: &Node<'_>) -> String
```

---

## `merge_impl()`

```
fn merge_impl(source: &str, node: &Node<'_>, entities: &mut [Entity], by_name: &HashMap<String, usize>)
```

---

## `includes()`

```
fn includes(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `calls()`

```
fn calls(source: &str, root: &Node<'_>) -> Vec<String>
```

---

## `walk_calls()`

```
fn walk_calls(source: &str, node: &Node<'_>, out: &mut Vec<String>)
```

---

## `call_name()`

```
fn call_name(source: &str, node: &Node<'_>) -> String
```

---

## `split_args()`

```
fn split_args(raw: &str) -> Vec<String>
```

Split a parameter list on top-level commas.

# main.rs

## `main()`

```
fn main() -> ExitCode
```

Run the CLI, printing any top-level error to stderr.

# mod.rs

## `EntityKind` Enum Reference

Kinds of declaration the documentation model can represent.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Module` | A source file or module. |
| `Function` | A free function. |
| `Method` | A function that belongs to a class or struct. |
| `Class` | A class declaration. |
| `Struct` | A struct declaration. |
| `Enum` | An enum declaration. |
| `Trait` | A Rust trait or equivalent abstract contract. |
| `Interface` | A C++ pure-virtual interface or similar abstract type. |

---

## `Param` Struct Reference

A documented parameter or field.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `name` | Parameter name as written in the source. |
| public | `String` | `r#type` | Declared type, when the parser can infer one. |
| public | `String` | `direction` | Doxygen direction (`in`, `out`, or `in,out`). |
| public | `String` | `description` | Human-readable description collected from the doc block. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(name: impl Into<String>) -> Self`<br>Create an input parameter with no type or description yet. |
| `` | `fn extend_description(&mut self, line: &str)`<br>Append a continuation line to the description, collapsing whitespace. |

---

## `Variant` Struct Reference

One enumerator or enum case.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `name` | Enumerator name. |
| public | `String` | `description` | Documentation attached to the enumerator. |

---

## `Field` Struct Reference

One data member of a class, struct, or interface.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `name` | Member name. |
| public | `String` | `r#type` | Declared member type. |
| public | `Visibility` | `visibility` | Effective member visibility. |
| public | `bool` | `r#static` | Whether the member is declared `static`. |
| public | `String` | `description` | Documentation attached to the member. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(name: impl Into<String>) -> Self`<br>Create a public instance field with no type or description yet. |

---

## `Doc` Struct Reference

Structured documentation extracted from a comment block.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `String` | `brief` | Short summary shown first in every rendered section. |
| public | `String` | `details` | Long-form description. |
| public | `Vec<Param>` | `params` | Documented parameters, in source order. |
| public | `String` | `returns` | Documented return value. |
| public | `Vec<(String, String)>` | `throws` | Exceptions and their descriptions. |
| public | `Vec<String>` | `see` | Cross references collected from `@see`. |
| public | `Vec<String>` | `notes` | Notes collected from `@note`. |
| public | `Vec<String>` | `warnings` | Warnings collected from `@warning`. |
| public | `Vec<String>` | `examples` | Examples collected from `@example`. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `bool` | `fn has_contract(&self) -> bool`<br>True when the documentation carries structured contract content. |

### Method Contracts

#### `has_contract()`

The brief is intentionally excluded: a brief alone is already rendered in the member table and does not deserve a separate contract section.

---

## `Entity` Struct Reference

One documented declaration and its optional children.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `EntityKind` | `kind` | Declared kind. |
| public | `String` | `name` | Declared name. |
| public | `String` | `signature` | Signature exactly as rendered in reference tables. |
| public | `String` | `return_type` | Declared return type. |
| public | `Doc` | `doc` | Structured documentation. |
| public | `bool` | `r#abstract` | Whether the declaration is abstract. |
| public | `Vec<Entity>` | `members` | Nested methods, functions, or type members. |
| public | `Vec<Variant>` | `variants` | Enum cases, for [`EntityKind::Enum`]. |
| public | `Vec<Field>` | `fields` | Data members, for class-like declarations. |
| public | `Vec<String>` | `bases` | Base classes or inherited interfaces. |
| public | `Vec<String>` | `implements` | Implemented interfaces or traits. |
| public | `String` | `module` | Owning module stem assigned during conversion. |
| public | `String` | `path` | Source path assigned during conversion. |
| public | `usize` | `line` | One-based source line, when known. |
| public | `Vec<String>` | `calls` | Function calls observed in the body. |
| public | `Vec<String>` | `includes` | Included headers or imported modules. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(kind: EntityKind, name: impl Into<String>) -> Self`<br>Create an empty entity of `kind` with the given `name`. |
| `` | `fn stamp(&mut self, path: &std::path::Path)`<br>Attach the source path and module stem to this entity and its members. |

---

## `UniquePush` Trait Reference

Extension trait for building ordered, de-duplicated name lists.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `` | abstract `fn push_unique(&mut self, value: impl Into<String>)`<br>Push `value` unless it is empty or already present. |

---

## `dedup_push()`

```
fn dedup_push(values: &mut Vec<String>, value: impl Into<String>)
```

Append a non-empty, unique string while preserving insertion order.

### Details

This is the legacy-compatible free-function form used by language parsers.

# tree_sitter.rs

## `python_parser()`

```
fn python_parser() -> anyhow::Result<Parser>
```

Create a parser for Python source.

### Returns

A configured Python parser.

---

## `parse_python()`

```
fn parse_python(source: &str) -> Option<Tree>
```

Parse Python source and reject trees containing syntax errors.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Python source text. |

### Returns

Parsed tree, or `None` for invalid syntax.

---

## `rust_parser()`

```
fn rust_parser() -> anyhow::Result<Parser>
```

Create a parser for Rust source.

### Returns

A configured Rust parser.

---

## `parse_rust()`

```
fn parse_rust(source: &str) -> Option<Tree>
```

Parse Rust source and reject trees containing syntax errors.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Rust source text. |

### Returns

Parsed tree, or `None` for invalid syntax.

---

## `cpp_parser()`

```
fn cpp_parser() -> anyhow::Result<Parser>
```

Create a parser for C++ source.

### Returns

A configured C++ parser.

---

## `parse_cpp()`

```
fn parse_cpp(source: &str) -> Option<Tree>
```

Parse C++ source and reject syntax-error trees.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | C++ source text. |

### Returns

Parsed tree, or `None` for invalid syntax.

---

## `bash_parser()`

```
fn bash_parser() -> anyhow::Result<Parser>
```

Create a parser for Bash source.

### Returns

A configured Bash parser.

---

## `parse_bash()`

```
fn parse_bash(source: &str) -> Option<Tree>
```

Parse Bash source and reject syntax-error trees.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `` | Bash source text. |

### Returns

Parsed tree, or `None` for invalid syntax.

---

## `has_missing()`

```
fn has_missing(node: &tree_sitter::Node<'_>) -> bool
```

---

## `parse()`

```
fn parse(language: &dyn Language, source: &str, filename: &str) -> Vec<Entity>
```

Parse source with an existing language plugin.

### Details

This is a compatibility facade; language lowering remains implemented by the existing plugin registry and is not duplicated here.

# markdown.rs

## `MarkdownRenderer` Struct Reference

Stateful entry point for Markdown reference rendering.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `ReferenceRenderer` | `reference` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new() -> Self`<br>Create a Markdown renderer. |
| `String` | `fn render(&self, entities: &[Entity]) -> String`<br>Render one or more entities as a complete Markdown document. |

### Method Contracts

#### `new()`

**Returns:** A renderer ready to emit API reference Markdown.

#### `render()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `entities` | `` | Entity tree to render. |

**Returns:** Rendered Markdown, or an empty string for an empty input.

---

## `escape_table_cell()`

```
fn escape_table_cell(value: &str) -> String
```

Escape a pipe so it cannot end a Markdown table cell.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `value` | `` | Raw table-cell text. |

### Returns

Text with Markdown table separators escaped.

---

## `cell()`

```
fn cell(value: &str) -> String
```

Escape a pipe for a table cell.

### Notes

> Retained as a concise helper name for renderer call sites.

---

## `table()`

```
fn table(header: &str, rule: &str, rows: impl IntoIterator<Item = String>) -> String
```

Join a header, separator, and rows into a Markdown table.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `header` | `` | Markdown table header row. |
| [in] | `rule` | `` | Markdown table separator row. |
| [in] | `rows` | `` | Table body rows. |

### Returns

A complete Markdown table.

---

## `code_list()`

```
fn code_list(items: &[String]) -> String
```

Render items as inline code separated by commas.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `items` | `` | Names or values to format. |

### Returns

A comma-separated Markdown inline-code list.

---

## `bullets()`

```
fn bullets(items: &[String]) -> String
```

Render items as Markdown bullet entries.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `items` | `` | Names or values to format. |

### Returns

Markdown bullet lines.

---

## `heading()`

```
fn heading(level: usize, text: &str) -> String
```

Render a Markdown heading.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `level` | `` | Heading level from one through six. |
| [in] | `text` | `` | Heading text. |

### Returns

A heading line, clamped to the Markdown heading range.

---

## `fenced_code()`

```
fn fenced_code(content: &str, language: Option<&str>) -> String
```

Wrap content in a fenced Markdown code block.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `content` | `` | Code or documentation example text. |
| [in] | `language` | `` | Optional fence language. |

### Returns

A fenced code block.

# mermaid.rs

## `MermaidRenderer` Struct Reference

Render parsed entities as Mermaid diagrams.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `String` | `fn title(kind: &str) -> String`<br>Return the human-readable title used for a diagram section. |
| `anyhow::Result<String>` | `fn render(&self, entities: &[Entity], kind: &str) -> anyhow::Result<String>`<br>Render one diagram kind, or fail with the list of valid kinds. |
| `String` | `fn render_all(&self, entities: &[Entity], kinds: Option<&[String]>) -> String`<br>Render a titled Mermaid document containing the selected kinds. |
| `String` | `fn class_diagram(&self, entities: &[Entity]) -> String`<br>Render the detailed class diagram. |
| `String` | `fn inheritance_diagram(&self, entities: &[Entity]) -> String`<br>Render the inheritance-only diagram. |
| `String` | `fn class_diagram_impl(&self, entities: &[Entity], detailed: bool, dependencies: bool, include_enums: bool) -> String`<br>Render a class diagram with configurable detail and edge kinds. |
| `String` | `fn collaboration_diagram(&self, entities: &[Entity]) -> String`<br>Render a flowchart of types used by fields, returns, and parameters. |
| `String` | `fn call_graph(&self, entities: &[Entity]) -> String`<br>Render the callee-oriented call graph. |
| `String` | `fn caller_graph(&self, entities: &[Entity]) -> String`<br>Render the caller-oriented call graph. |
| `String` | `fn call_diagram(&self, entities: &[Entity], inverse: bool) -> String`<br>Render a call graph, optionally inverted into a caller graph. |
| `String` | `fn include_graph(&self, entities: &[Entity]) -> String`<br>Render the module include graph. |
| `String` | `fn directory_graph(&self, entities: &[Entity]) -> String`<br>Render the directory-to-directory include graph. |

---

## `Identifiers` Struct Reference

Identifiers assigned to class-like nodes and lookup tables for edge targets.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `HashMap<usize, String>` | `ids` | Identifier for each collected node index. |
| private | `HashMap<String, String>` | `by_name` | First identifier registered for each entity name. |
| private | `HashMap<(String, String), String>` | `by_file` | First identifier registered for each `(path, name)` pair. |

---

## `collect()`

```
fn collect(entities: &[Entity], include_enums: bool) -> Vec<&Entity>
```

Collect class-like declarations, optionally including enums.

---

## `identifiers()`

```
fn identifiers(nodes: &[&Entity]) -> Identifiers
```

Assign unique Mermaid identifiers to each collected node.

---

## `block()`

```
fn block(entity: &Entity, identifier: &str, detailed: bool) -> Vec<String>
```

Render one class block, including fields, methods, and enum cases.

---

## `stereotype()`

```
fn stereotype(entity: &Entity) -> &'static str
```

Return the Mermaid stereotype for an entity, or an empty string.

---

## `display_type()`

```
fn display_type(raw: &str) -> String
```

Convert a source type into Mermaid-safe display text.

---

## `class_edges()`

```
fn class_edges(nodes: &[&Entity], ids: &Identifiers, dependencies: bool) -> Vec<String>
```

Build inheritance, implementation, and optional dependency edges.

---

## `lookup()`

```
fn lookup(name: &str, by_name: &HashMap<String, String>, by_file: &HashMap<(String, String), String>, path: &str) -> Option<String>
```

Resolve a type name against same-file nodes first, then by name.

---

## `type_names()`

```
fn type_names(raw: &str) -> Vec<String>
```

Extract referenced type tokens, ignoring built-in scalar types.

---

## `used_types()`

```
fn used_types(entity: &Entity) -> Vec<String>
```

Collect unique type names referenced by fields, returns, and parameters.

---

## `functions()`

```
fn functions(entities: &'a [Entity], owner: &str) -> Vec<(&'a Entity, String)>
```

Flatten functions and methods, tagging each with its owning type name.

---

## `FunctionNodes` Struct Reference

Identifiers and labels assigned to function and method nodes.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `HashMap<usize, String>` | `ids` | Identifier for each callable node index. |
| private | `HashMap<usize, String>` | `labels` | Display label for each callable node index. |
| private | `HashMap<String, String>` | `by_name` | First identifier registered for each name. |
| private | `HashMap<(String, String), String>` | `by_file` | First identifier registered for each `(path, name)` pair. |

---

## `function_nodes()`

```
fn function_nodes(functions: &[(&Entity, String)]) -> FunctionNodes
```

Assign stable identifiers and qualified labels to callable nodes.

---

## `call_target()`

```
fn call_target(call: &str, owner: &str) -> String
```

Qualify `self.method` and `cls.method` calls with the owning type.

---

## `qualified()`

```
fn qualified(entity: &Entity, owner: &str) -> String
```

Build a dotted label from module, owner, and entity names.

---

## `modules()`

```
fn modules(entities: &[Entity]) -> Vec<&Entity>
```

Return the module entities from a flat entity list.

---

## `module_index()`

```
fn module_index(modules: &[&Entity]) -> (HashMap<usize, String>, HashMap<String, String>)
```

Assign module identifiers and index them by path, stem, and name.

---

## `resolve_module()`

```
fn resolve_module(target: &str, by_key: &HashMap<String, String>) -> Option<String>
```

Resolve an include target to a known module identifier.

---

## `include_candidates()`

```
fn include_candidates(target: &str) -> Vec<String>
```

Expand one include string into the names `dox.py` tries to match.

---

## `sanitize()`

```
fn sanitize(value: &str) -> String
```

Replace every character Mermaid cannot use in an identifier.

---

## `label()`

```
fn label(text: &str) -> String
```

Escape text for a double-quoted Mermaid label.

---

## `as_posix()`

```
fn as_posix(path: &str) -> String
```

Normalize a path to POSIX separators for display.

---

## `path_file_name()`

```
fn path_file_name(path: &str) -> Option<String>
```

Return the final path component, when there is one.

---

## `path_file_stem()`

```
fn path_file_stem(path: &str) -> Option<String>
```

Return the final path component without its extension.

---

## `parent_posix()`

```
fn parent_posix(path: &str) -> String
```

Return the parent directory in POSIX form, using `.` for none.

# reference.rs

## `ReferenceRenderer` Struct Reference

Render a parsed entity tree as Markdown.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `String` | `fn render(&self, entities: &[Entity]) -> String`<br>Render every entity, separating top-level sections with horizontal rules. |
| `String` | `fn entity(&self, entity: &Entity) -> String`<br>Dispatch one entity to the renderer for its kind. |
| `String` | `fn module_reference(&self, entity: &Entity) -> String`<br>Render a module as its brief, details, and examples. |
| `String` | `fn function(&self, entity: &Entity) -> String`<br>Render a function as a full API reference section. |
| `String` | `fn class_reference(&self, entity: &Entity) -> String`<br>Render a class-like declaration as a reference section with tables. |
| `String` | `fn enum_reference(&self, entity: &Entity) -> String`<br>Render an enum as a reference section with an enumerator table. |
| `Option<String>` | `fn relations(&self, entity: &Entity) -> Option<String>`<br>Render inheritance and interface relations, when present. |
| `Option<String>` | `fn member_contracts(&self, members: &[Entity]) -> Option<String>`<br>Render the contracts of members that have structured documentation. |
| `String` | `fn member_contract(&self, member: &Entity) -> String`<br>Render one method contract entry. |
| `String` | `fn example_section(&self, entity: &Entity, heading: &str) -> String`<br>Render an example section, leaving already-fenced code untouched. |
| `String` | `fn members_table(&self, members: &[Entity]) -> String`<br>Render the public member function table. |
| `String` | `fn fields_table(&self, fields: &[Field]) -> String`<br>Render the data member table. |
| `String` | `fn params_table(&self, params: &[Param]) -> String`<br>Render the parameter table. |
| `String` | `fn throws_table(&self, throws: &[(String, String)]) -> String`<br>Render the exception table. |
| `String` | `fn quote_blocks(&self, items: &[String]) -> String`<br>Render list entries as block quotes. |

# analyzer.rs

## `Analysis` Struct Reference

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `SymbolTable` | `symbols` |  |

---

## `analyze()`

```
fn analyze(entities: &[Entity]) -> Analysis
```

# symbol_table.rs

## `SymbolTable` Struct Reference

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `HashMap<String, Vec<NodeId>>` | `by_name` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn from_entities(entities: &[Entity]) -> Self` |
| `` | `fn add(&mut self, entity: &Entity, index: &mut usize)` |
| `&[NodeId]` | `fn get(&self, name: &str) -> &[NodeId]` |
| `usize` | `fn len(&self) -> usize` |
| `bool` | `fn is_empty(&self) -> bool` |

# type_resolver.rs

## `TypeResolver` Struct Reference

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `&'a [NodeId]` | `fn resolve(&self, table: &'a SymbolTable, raw: &str) -> &'a [NodeId]` |

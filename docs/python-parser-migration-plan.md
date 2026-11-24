# Python Parser Migration Plan

## Status

Implementation complete for Python, Rust, C++, and Bash. Ruff dependencies
have been removed. The remaining work is parity expansion and broader fixture
coverage, not parser migration.

## Goal

Replace the four pinned Ruff dependencies in `Cargo.toml` with a shared
Tree-sitter parsing architecture. Tree-sitter is used for syntax parsing across
supported languages, while Rust-native semantic AST lowering maps every language
into the existing `Entity` model:

- remove `ruff_python_ast`
- remove `ruff_python_codegen`
- remove `ruff_python_parser`
- remove `ruff_source_file`
- keep `PythonLanguage`, `RustLanguage`, `CppLanguage`, and `BashLanguage` as public language-plugin
  entry points
- add Tree-sitter grammars through a common parser infrastructure
- replace heuristic parsing for Rust and C++ rather than maintaining
  a separate parser architecture per language
- preserve byte-for-byte output and existing CLI behavior

This plan changes implementation dependencies only. It does not change
`Entity`, `Doc`, the language trait, diagrams, CLI arguments, or the current
renderers.

## Terminology

"Rust-native AST" means a project-owned Rust syntax model, not the AST
of the Rust compiler. Rust `syn` is therefore not suitable for Python source.
Tree-sitter supplies parsing and syntax nodes; the project converts those nodes
into a small normalized Rust-native Python AST before semantic lowering.
The same principle applies to Rust and C++: Tree-sitter supplies syntax, and
Rust-owned lowering types and logic preserve the language-neutral `Entity`
model.

The proposed split is:

```text
Python source
  -> tree-sitter-python parser
  -> Rust PythonSyntax model
  -> Rust semantic lowering
  -> existing Entity model
  -> existing Markdown/Mermaid renderers
```

## Proposed Dependencies

Add:

- `tree-sitter`
- `tree-sitter-python`
- `tree-sitter-rust`
- `tree-sitter-cpp`

Keep:

- `regex`
- `encoding_rs`
- `anyhow`

Do not add a second general-purpose parser per language abstraction. Use a
shared Tree-sitter runtime and normalized Rust-native lowering types, with
language-specific mapping kept behind the existing `Language` trait.

Suggested layout:

```text
src/parser/
  mod.rs
  tree_sitter.rs
src/lang/
  python.rs
  rust.rs
  cpp.rs
  registry.rs
```

Tree-sitter grammars should be isolated behind parser wrappers. Language
plugins must depend on normalized syntax or lowering APIs, not directly on
grammar-specific node APIs.

## Internal AST

Create Rust-owned types for only the constructs the contract currently needs:

- `PyModule`
- `PyStatement`
- `PyFunction`
- `PyClass`
- `PyAssignment`
- `PyAnnotatedAssignment`
- `PyParameter`
- `PyExpression`
- `PyImport`
- `PyImportFrom`

Minimum expression nodes:

- name
- attribute
- subscript
- tuple
- list
- bitwise union
- string
- slice
- call
- lambda
- other expressions, retained as source text or as an opaque fallback

The AST must retain:

- source line and byte range
- top-level statement order
- function and class body order
- async status
- decorators
- function parameters and annotations
- base expressions
- assignment targets and values
- import aliases and relative-import level

Avoid mirroring every Tree-sitter node in public API types. The internal AST
should contain contract-relevant semantics, not parser implementation detail.

## Lowering Requirements

Keep the existing behavior in [src/lang/python.rs](/home/phng/Code/Wheels/Dozen/src/lang/python.rs):

- module entity for every successful parse
- cleaned module docstring
- top-level function, async function, and class only
- source line numbers
- imports in breadth-first source order
- call extraction with nested function, class, and lambda pruning
- function and method signatures
- parameter annotation merging
- return annotations
- enum detection
- abstract base and decorator detection
- class fields and `__init__` inference
- enum variants
- public, protected, and private visibility
- existing doc-tag parsing through `DOC_PARSER`

The lowering phase must continue to return `Vec<Entity>` and must not expose
Tree-sitter or AST types to the renderer.

## Syntax and Error Semantics

Tree-sitter is error-tolerant. The parser must detect syntax errors and behave
like the Ruff implementation:

- return no entities on invalid syntax
- do not partially emit a module
- preserve the existing syntax-error fallback behavior
- perform syntax validation with Tree-sitter before AST lowering

The implementation should distinguish:

- parser node kinds marked as `ERROR`
- `MISSING` nodes introduced for malformed input
- incomplete constructs at end of file

Unknown or unsupported syntax inside a function body should not necessarily
invalidate an otherwise valid module. Invalid structure at module,
declaration, parameter, annotation, or assignment boundaries should invalidate
the relevant parse according to parity tests.

## CPython-Style Annotation Rendering

Replace Ruff codegen and the custom AST unparser with a dedicated Rust
expression renderer that matches Python `ast.unparse` formatting.

Required first-pass cases:

- names and attributes
- forward-reference strings
- subscript and tuple slices
- lists
- tuples, including one-element tuples
- bitwise-or unions
- calls
- slice expressions
- literals used by annotations

The renderer must preserve the checked examples:

- `list["Entity"]` -> `list['Entity']`
- `Path | None`
- `tuple[str, str]`
- `dict[str, int]`
- `X[a:b, c]` -> `X[a:b, c]`

Treat Python 3.15 or CPython host-dependent formatting differences as
contract parity issues, not as a reason to invoke Python at runtime.

## Implementation Phases

### Shared parser layer

1. Introduce a common Tree-sitter parser factory.
2. Register Python, Rust, and C++ grammars.
3. Centralize UTF-8 offsets, source ranges, and syntax-error detection.
4. Provide stable node inspection helpers for the language lowering modules.
5. Add one malformed-source test per grammar.

Exit gate:

- all supported grammars compile and parse fixture source
- byte ranges and one-based line numbers are consistent
- `ERROR` and `MISSING` nodes are detectable before lowering

### Phase 1: Dependency and parser spike

1. Add `tree-sitter` and `tree-sitter-python`.
2. Remove no implementation until a parser spike passes.
3. Confirm the parser accepts the current Python fixtures.
4. Record node kinds for module, function, class, parameter, annotation,
   assignment, import, call, subscript, and slice nodes.
5. Confirm syntax-error nodes and byte/range semantics.
6. Add unit tests for invalid syntax and source line calculation.

Exit gate:

- every Python fixture produces a non-empty Tree-sitter module tree
- invalid syntax is detected before lowering
- byte ranges map to the same one-based source lines

### Phase 2: Internal AST

1. Introduce Rust-owned Python syntax types.
2. Convert top-level module nodes into the internal AST.
3. Parse function signatures and class bodies.
4. Do not yet replace Ruff entry points used in production.

Exit gate:

- small AST fixtures cover functions, async functions, classes, enums,
  imports, fields, decorators, annotations, and calls
- no Ruff AST types leak into the internal AST

### Phase 3: Expression lowering

1. Implement the Rust expression node and CPython-style renderer.
2. Add targeted tests for subscript, tuple, list, union, call, attribute,
  and slice expressions.
3. Compare against frozen annotation examples.

Exit gate:

- all existing `unparse` tests pass
- no Ruff codegen or source-file dependency remains in the expression path

### Phase 4: Semantic parity

1. Move module/function/class/import/call/field lowering to the internal AST.
2. Preserve breadth-first and source-order behavior.
3. Switch `PythonLanguage::parse` to Tree-sitter.
4. Run all Python fixtures and self-hosted golden tests.

Exit gate:

- `animal.py` convert and all diagram kinds match frozen goldens
- `bank.py` matches its golden
- Python contract output remains byte-for-byte compatible

### Phase 5: Remove Ruff

1. Remove all Ruff imports from `src/lang/python.rs`.
2. Remove all four Ruff dependencies and lockfile entries.
3. Remove Ruff-specific comments and documentation.
4. Search the repository for stale Ruff references.
5. Run all quality gates.

Exit gate:

- `cargo build` succeeds without Ruff crates
- all tests pass
- no Ruff crate remains in `Cargo.toml` or `Cargo.lock`
- no Python subprocess is used at runtime

## Test Strategy

Add tests in `src/lang/python.rs` for:

- module docstrings
- top-level function and async function
- class bases
- enum classes
- abstract classes and methods
- public/protected/private fields
- annotated fields
- `__init__` assignments
- inferred field types
- imports and relative imports
- nested definition and lambda call pruning
- breadth-first call and import order
- malformed input
- incomplete declarations
- forward references and unions
- subscript slices

Keep the existing golden parity suite as the primary acceptance gate:

- `tests/fixtures/animal.py`
- `tests/fixtures/bank.py`
- self-hosted Doxygen contract conversion
- self-hosted all-kind diagram generation

## Risks

| Risk | Mitigation |
|------|------------|
| Tree-sitter has error-recovery nodes | Validate `ERROR` and `MISSING` nodes before lowering |
| Node-kind differences across grammar versions | Pin exact dependency versions and add fixture-level tests |
| Annotation formatting differs from CPython | Add a dedicated AST unparser with frozen parity examples |
| BFS ordering differs | Implement explicit source-order queues and level collection |
| Byte ranges differ from character offsets | Convert parser byte offsets using UTF-8-safe line counting |
| Python syntax is too broad for a hand-written AST | Lower only contract-relevant constructs and retain opaque expressions |
| Migration changes output | Run golden parity after every lowering phase; do not update goldens first |

## Recommendation

Proceed with Tree-sitter plus a Rust-owned normalized AST. It removes Ruff
runtime dependencies and reduces coupling, but it is a parser migration rather
than a dependency-only swap.

Implementation is now driven in the phases above. Ruff must remain out of the
runtime path and has been removed from `Cargo.toml` after the Python fixture
parity gates passed.

#!/usr/bin/env python3
"""dox — multi-language Doxygen-reference generator.

@brief Turn Doxygen-style source comments into a uniform Markdown API contract.

@details Pipeline of replaceable parts:

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

@example
python dox.py -c ini.cfg convert
python dox.py -c ini.cfg diagram -k all -o DIAGRAM.md
python dox.py -c ini.cfg init docs
"""

from __future__ import annotations

import argparse
import ast
import glob as globlib
import re
import sys
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Iterable

__version__ = "0.5.0"


# ============================================================
# CONFIG
# ============================================================

CONFIG_FILENAME = "ini.cfg"

DEFAULT_CONFIG_TEXT = """\
[defaults]
glob     = *
output   = README.md
diagram  = DIAGRAM.md
encoding = utf-8
"""


@dataclass
class Config:
    """@brief Settings that control which files are read and where output is written.

    @details The `[defaults]` section of `ini.cfg` recognizes four keys:
    `glob`, `output`, `diagram`, and `encoding`. `base_dir` is derived from the
    loaded config file and resolves relative output and diagram paths.
    """

    glob: str = "*"
    output: Path = Path("README.md")
    diagram: Path = Path("DIAGRAM.md")
    encoding: str = "utf-8"
    base_dir: Path = field(default_factory=Path.cwd, repr=False, compare=False)

    @classmethod
    def defaults(cls) -> "Config":
        """@brief Build the built-in default configuration.

        @return A `Config` equivalent to parsing `DEFAULT_CONFIG_TEXT`.
        """
        return parse_config(DEFAULT_CONFIG_TEXT)

    @classmethod
    def load(cls, path: Path | None) -> "Config":
        """@brief Load configuration from an explicit path or the current directory.

        @param[in] path Explicit configuration file, or `None` to discover
        `ini.cfg` in the current directory.
        @return The resolved `Config`.
        @throws SystemExit When an explicit `path` is not a file.
        """
        if path is not None:
            if not path.is_file():
                raise SystemExit(f"Config file not found: {path}")
            return parse_config(
                path.read_text(encoding="utf-8"), base_dir=path.parent
            )
        candidate = Path.cwd() / CONFIG_FILENAME
        if candidate.is_file():
            return parse_config(
                candidate.read_text(encoding="utf-8"), base_dir=candidate.parent
            )
        return cls.defaults()


def parse_config(text: str, base_dir: Path | None = None) -> Config:
    """@brief Parse INI-style configuration text.

    @param[in] text Configuration file contents.
    @param[in] base_dir Directory used to resolve relative paths.
    @return A populated `Config`.
    """
    cfg = Config()
    if base_dir is not None:
        cfg.base_dir = base_dir
    section = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith(("#", ";")):
            continue
        m = re.match(r"^\[([^\]]+)\]\s*$", line)
        if m:
            section = m.group(1).lower()
            continue
        m = re.match(r"^(\w+)\s*=\s*(.*)$", line)
        if not m or section != "defaults":
            continue
        key, value = m.group(1).lower(), m.group(2).strip()
        if key == "glob":
            cfg.glob = value
        elif key == "output":
            cfg.output = Path(value)
        elif key == "diagram":
            cfg.diagram = Path(value)
        elif key == "encoding":
            cfg.encoding = value or "utf-8"
    return cfg


# ============================================================
# CORE MODEL — language-agnostic
# ============================================================


class EntityKind(Enum):
    """@brief Kinds of declaration the documentation model can represent."""

    MODULE = "module"
    FUNCTION = "function"
    METHOD = "method"
    CLASS = "class"
    STRUCT = "struct"
    ENUM = "enum"
    TRAIT = "trait"
    INTERFACE = "interface"


@dataclass
class Param:
    """@brief A documented parameter or field."""

    name: str
    type: str = ""
    direction: str = "in"
    description: str = ""


@dataclass
class Variant:
    """@brief One enumerator or enum case."""

    name: str
    description: str = ""


@dataclass
class Field:
    """@brief One data member of a class, struct, or interface."""

    name: str
    type: str = ""
    visibility: str = "public"
    static: bool = False
    description: str = ""


@dataclass
class Doc:
    """@brief Structured documentation extracted from a comment block."""

    brief: str = ""
    details: str = ""
    params: list[Param] = field(default_factory=list)
    returns: str = ""
    throws: list[tuple[str, str]] = field(default_factory=list)
    see: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    examples: list[str] = field(default_factory=list)


@dataclass
class Entity:
    """@brief One documented declaration and its optional children."""

    kind: EntityKind
    name: str
    signature: str = ""
    return_type: str = ""
    doc: Doc = field(default_factory=Doc)
    abstract: bool = False
    members: list["Entity"] = field(default_factory=list)
    variants: list[Variant] = field(default_factory=list)
    fields: list[Field] = field(default_factory=list)
    bases: list[str] = field(default_factory=list)
    implements: list[str] = field(default_factory=list)
    module: str = ""
    path: str = ""
    line: int = 0
    calls: list[str] = field(default_factory=list)
    includes: list[str] = field(default_factory=list)


# ============================================================
# DOC PARSER — universal @tag syntax
# ============================================================

TAG_RE = re.compile(r"^\s*[@\\](\w+)(?:\[([^\]]+)\])?(?:\s+(.*))?$")


class DocParser:
    """@brief Convert a comment block into a structured `Doc` model.

    @details Leading untagged text is split into paragraphs: first is `brief`,
    the rest is `details`. Tags are case-insensitive and may use `@` or `\\`.
    Continuation lines extend the active tag; blank lines are preserved inside
    `details` and `examples`.
    """

    _TAG_ALIASES = {
        "return": "returns",
        "returns": "returns",
        "exception": "throws",
        "raises": "throws",
    }

    _LIST_TAGS = {
        "see": "see",
        "note": "notes",
        "warning": "warnings",
        "example": "examples",
    }

    def parse(self, text: str) -> Doc:
        """@brief Parse Doxygen-style tags into a `Doc` model.

        @param[in] text Raw docstring or comment text.
        @return A `Doc` with brief, details, parameters, returns, exceptions,
        see-also references, notes, warnings, and examples populated.
        """
        doc = Doc()
        if not text or not text.strip():
            return doc

        lines = text.splitlines()

        first_tag = next(
            (i for i, ln in enumerate(lines) if TAG_RE.match(ln.strip())),
            None,
        )
        leading = lines[:first_tag] if first_tag is not None else lines
        tag_lines = lines[first_tag:] if first_tag is not None else []

        paragraphs = self._paragraphs(leading)
        if paragraphs:
            doc.brief = paragraphs[0]
            if len(paragraphs) > 1:
                doc.details = "\n\n".join(paragraphs[1:])

        tag: str | None = None
        last_param: Param | None = None
        throw_index = -1
        list_name: str | None = None
        list_index = -1

        for raw in tag_lines:
            line = raw.strip()
            if not line:
                if tag == "details":
                    doc.details += "\n"
                elif list_name == "examples" and list_index >= 0:
                    doc.examples[list_index] += "\n"
                continue

            m = TAG_RE.match(line)
            if m:
                tag = self._TAG_ALIASES.get(m.group(1).lower(), m.group(1).lower())
                direction = m.group(2) or "in"
                rest = (m.group(3) or "").strip()

                if tag == "brief":
                    doc.brief = rest
                elif tag == "details":
                    doc.details = rest
                elif tag == "param":
                    name, _, desc = rest.partition(" ")
                    last_param = Param(name, direction=direction, description=desc.strip())
                    doc.params.append(last_param)
                elif tag == "returns":
                    doc.returns = rest
                elif tag == "throws":
                    exc, _, desc = rest.partition(" ")
                    doc.throws.append((exc, desc.strip()))
                    throw_index = len(doc.throws) - 1
                elif tag in self._LIST_TAGS:
                    list_name = self._LIST_TAGS[tag]
                    lst = getattr(doc, list_name)
                    lst.append(rest)
                    list_index = len(lst) - 1
                else:
                    tag = None
                continue

            if tag == "brief":
                doc.brief += " " + line
            elif tag == "details":
                doc.details += "\n" + raw.rstrip()
            elif tag == "param" and last_param:
                last_param.description = (last_param.description + " " + line).strip()
            elif tag == "returns":
                doc.returns += " " + line
            elif tag == "throws" and throw_index >= 0:
                exc, desc = doc.throws[throw_index]
                doc.throws[throw_index] = (exc, (desc + " " + line).strip())
            elif list_name and list_index >= 0:
                lst = getattr(doc, list_name)
                if list_name == "examples":
                    lst[list_index] += "\n" + raw.rstrip()
                else:
                    lst[list_index] += " " + line
            else:
                doc.brief = (doc.brief + " " + line).strip()

        doc.brief = doc.brief.strip()
        doc.details = doc.details.strip("\n")
        return doc

    @staticmethod
    def _paragraphs(lines: list[str]) -> list[str]:
        """@brief Split lines into paragraphs separated by blank lines.

        @param[in] lines Lines to group.
        @return One string per paragraph with normalized internal whitespace.
        """
        out, buf = [], []
        for raw in lines:
            line = raw.strip()
            if not line:
                if buf:
                    out.append(" ".join(buf))
                    buf = []
            else:
                buf.append(line)
        if buf:
            out.append(" ".join(buf))
        return out


DOC_PARSER = DocParser()


# ============================================================
# LANGUAGE PLUGIN — abstract base
# ============================================================

LANGUAGES: dict[str, type["Language"]] = {}


class Language(ABC):
    """@brief Base class for language plugins.

    @details Subclasses set `extensions` and implement `parse()`. Extensions
    are registered in `LANGUAGES` through `__init_subclass__`.
    """

    extensions: tuple[str, ...] = ()

    def __init_subclass__(cls, **kwargs):
        """@brief Register a language subclass by its declared extensions."""
        super().__init_subclass__(**kwargs)
        for ext in cls.extensions:
            LANGUAGES[ext.lower()] = cls

    @abstractmethod
    def parse(self, source: str, filename: str) -> list[Entity]:
        """@brief Discover documented declarations in a source file."""
        ...

    @staticmethod
    def consume_block(lines: list[str], start: int) -> int:
        """@brief Count the lines occupied by the block starting at `start`.

        @details Braces are tracked per character so one-line blocks
        (`fn foo() {}`) are handled. Without braces, a `;` ends the block.
        """
        depth = 0
        opened = False
        for i in range(start, len(lines)):
            for ch in lines[i]:
                if ch == "{":
                    depth += 1
                    opened = True
                elif ch == "}":
                    depth -= 1
            if opened and depth <= 0:
                return i - start + 1
            if not opened and ";" in lines[i]:
                return i - start + 1
        return len(lines) - start

    @staticmethod
    def _read_block_comment(lines: list[str], start: int) -> tuple[list[str], int]:
        """@brief Read a `/** ... */` block starting at `start`.

        @details Returns `([], 0)` when the line does not start a block comment.
        """
        raw = lines[start]
        m = re.match(r"^\s*/\*\*", raw)
        if not m:
            return [], 0
        out: list[str] = []
        i = start
        while i < len(lines):
            text = lines[i] if i > start else raw[m.end():]
            close = text.find("*/")
            if close >= 0:
                line = re.sub(r"^\s*\*\s?", "", text[:close]).rstrip()
                if line.strip():
                    out.append(line)
                return out, i - start + 1
            line = re.sub(r"^\s*\*\s?", "", text).rstrip()
            if line.strip():
                out.append(line)
            i += 1
        return out, len(lines) - start


# ============================================================
# PYTHON LANGUAGE — ast-based
# ============================================================


class PythonLanguage(Language):
    """@brief Parse Python modules, classes, and functions through `ast`."""

    extensions = (".py",)

    _ABSTRACT_DECOS = {"abstractmethod", "abstractproperty"}
    _ENUM_BASES = {"Enum", "IntEnum", "StrEnum", "Flag", "IntFlag"}
    _ABSTRACT_BASES = {"ABC", "ABCMeta"}

    def parse(self, source: str, filename: str) -> list[Entity]:
        """@brief Parse Python source text into entities."""
        try:
            tree = ast.parse(source)
        except SyntaxError:
            return []

        out: list[Entity] = []
        module = Entity(
            EntityKind.MODULE,
            filename,
            doc=DOC_PARSER.parse(ast.get_docstring(tree) or ""),
        )
        module.line = 1
        module.includes = self._imports(tree)
        out.append(module)

        for node in tree.body:
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
                entity = self._function(node)
                entity.line = node.lineno
                out.append(entity)
            elif isinstance(node, ast.ClassDef):
                kind = EntityKind.ENUM if self._is_enum(node) else EntityKind.CLASS
                entity = self._class(node, kind)
                entity.line = node.lineno
                out.append(entity)
        return out

    def _function(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> Entity:
        """@brief Build a function entity from an AST node."""
        doc = DOC_PARSER.parse(ast.get_docstring(node) or "")
        ann = {
            a.arg: self._unparse(a.annotation) for a in node.args.args if a.annotation
        }
        for p in doc.params:
            p.type = p.type or ann.get(p.name, "")
        ret = self._unparse(node.returns) if node.returns else ""
        entity = Entity(
            kind=EntityKind.FUNCTION,
            name=node.name,
            signature=self._signature(node),
            return_type=ret,
            doc=doc,
        )
        entity.calls = self._calls(node)
        return entity

    def _signature(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
        """@brief Render a normalized Python function signature."""
        args = []
        for a in node.args.args:
            if a.arg in ("self", "cls"):
                continue
            s = a.arg
            if a.annotation:
                s += f": {self._unparse(a.annotation)}"
            args.append(s)
        ret = f" -> {self._unparse(node.returns)}" if node.returns else ""
        prefix = "async def" if isinstance(node, ast.AsyncFunctionDef) else "def"
        return f"{prefix} {node.name}({', '.join(args)}){ret}"

    def _class(self, node: ast.ClassDef, kind: EntityKind) -> Entity:
        """@brief Build a class or enum entity from an AST node."""
        doc = DOC_PARSER.parse(ast.get_docstring(node) or "")
        abstract = self._is_abstract(node)

        entity = Entity(kind=kind, name=node.name, doc=doc, abstract=abstract)

        if kind == EntityKind.ENUM:
            entity.variants = [
                Variant(name=t.id, description="")
                for item in node.body
                if isinstance(item, ast.Assign)
                for t in item.targets
                if isinstance(t, ast.Name) and not t.id.startswith("_")
            ]
            return entity

        entity.bases = self._base_names(node)
        init = next(
            (
                item
                for item in node.body
                if isinstance(item, ast.FunctionDef) and item.name == "__init__"
            ),
            None,
        )
        entity.fields = self._fields(node, init)

        for item in node.body:
            if not isinstance(item, ast.FunctionDef | ast.AsyncFunctionDef):
                continue
            is_public = not item.name.startswith("_") or item.name in (
                "__init__",
                "__del__",
            )
            if not is_public:
                continue
            fn = self._function(item)
            fn.kind = EntityKind.METHOD
            fn.abstract = self._is_abstract_method(item)
            fn.line = item.lineno
            entity.members.append(fn)
        return entity

    def _base_names(self, node: ast.ClassDef) -> list[str]:
        """@brief Collect relationship-describing base class names."""
        excluded = self._ABSTRACT_BASES | self._ENUM_BASES | {"object"}
        names: list[str] = []
        for base in node.bases:
            name = self._node_name(base)
            if name and name not in excluded and name not in names:
                names.append(name)
        return names

    def _fields(
        self, node: ast.ClassDef, init: ast.FunctionDef | None
    ) -> list[Field]:
        """@brief Collect data members declared by a Python class."""
        found: dict[str, Field] = {}
        hints = {
            arg.arg: self._unparse(arg.annotation)
            for arg in (init.args.args if init else [])
            if arg.annotation
        }
        for item in node.body:
            if isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
                self._add_field(found, item.target.id, self._unparse(item.annotation))
        if init is not None:
            for item in ast.walk(init):
                if isinstance(item, ast.AnnAssign) and self._is_self_attr(item.target):
                    self._add_field(
                        found,
                        item.target.attr,
                        self._unparse(item.annotation),
                        self._visibility(item.target.attr),
                    )
                elif isinstance(item, ast.Assign):
                    for target in item.targets:
                        if not self._is_self_attr(target):
                            continue
                        inferred = ""
                        if isinstance(item.value, ast.Name):
                            inferred = hints.get(item.value.id, "")
                        self._add_field(
                            found,
                            target.attr,
                            inferred,
                            self._visibility(target.attr),
                        )
        params = (
            DOC_PARSER.parse(ast.get_docstring(init) or "").params if init else []
        )
        descriptions = {p.name: p.description for p in params}
        for field_ in found.values():
            field_.description = descriptions.get(field_.name, "")
        return list(found.values())

    @staticmethod
    def _add_field(
        found: dict[str, Field],
        name: str,
        type_: str,
        visibility: str = "public",
    ) -> None:
        """@brief Add or enrich one field in a name-keyed collection."""
        if name.startswith("__") and name.endswith("__"):
            return
        field_ = found.get(name)
        if field_ is None:
            found[name] = Field(name=name, type=type_, visibility=visibility)
        elif type_ and not field_.type:
            field_.type = type_

    @staticmethod
    def _is_self_attr(node) -> bool:
        """@brief Check whether an AST target is `self.<name>`."""
        return (
            isinstance(node, ast.Attribute)
            and isinstance(node.value, ast.Name)
            and node.value.id == "self"
        )

    @staticmethod
    def _visibility(name: str) -> str:
        """@brief Infer Python attribute visibility from its name."""
        if name.startswith("__"):
            return "private"
        if name.startswith("_"):
            return "protected"
        return "public"

    @staticmethod
    def _node_name(node) -> str:
        """@brief Reduce a base-class expression to a simple name."""
        if isinstance(node, ast.Name):
            return node.id
        if isinstance(node, ast.Attribute):
            return node.attr
        if isinstance(node, ast.Subscript):
            return PythonLanguage._node_name(node.value)
        return ""

    @staticmethod
    def _imports(tree: ast.Module) -> list[str]:
        """@brief Collect imported module names from a Python module."""
        out: list[str] = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    if alias.name not in out:
                        out.append(alias.name)
            elif isinstance(node, ast.ImportFrom):
                module = "." * (node.level or 0) + (node.module or "")
                if module and module not in out:
                    out.append(module)
        return out

    def _calls(self, node) -> list[str]:
        """@brief Collect names called from a function body."""
        out: list[str] = []
        stack = list(ast.iter_child_nodes(node))
        i = 0
        while i < len(stack):
            child = stack[i]
            i += 1
            if isinstance(
                child,
                ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef | ast.Lambda,
            ):
                continue
            if isinstance(child, ast.Call):
                name = self._call_name(child.func)
                if name and name not in out:
                    out.append(name)
            stack.extend(ast.iter_child_nodes(child))
        return out

    @staticmethod
    def _call_name(node) -> str:
        """@brief Reduce a call target to its final or owner-qualified name."""
        if isinstance(node, ast.Name):
            return node.id
        if isinstance(node, ast.Attribute):
            owner = ""
            if isinstance(node.value, ast.Name):
                owner = node.value.id
            elif isinstance(node.value, ast.Call):
                owner = PythonLanguage._call_name(node.value.func)
            return f"{owner}.{node.attr}" if owner else node.attr
        return ""

    def _is_enum(self, node: ast.ClassDef) -> bool:
        """@brief Check whether a class derives from a known enum base."""
        return any(
            (isinstance(b, ast.Name) and b.id in self._ENUM_BASES)
            or (isinstance(b, ast.Attribute) and b.attr in self._ENUM_BASES)
            for b in node.bases
        )

    def _is_abstract(self, node: ast.ClassDef) -> bool:
        """@brief Check whether a class is abstract."""
        for b in node.bases:
            name = (
                b.id
                if isinstance(b, ast.Name)
                else (b.attr if isinstance(b, ast.Attribute) else None)
            )
            if name in self._ABSTRACT_BASES:
                return True
        return any(
            isinstance(item, ast.FunctionDef) and self._is_abstract_method(item)
            for item in node.body
        )

    def _is_abstract_method(self, node: ast.FunctionDef) -> bool:
        """@brief Check whether a function carries an abstract decorator."""
        for deco in node.decorator_list:
            name = (
                deco.id
                if isinstance(deco, ast.Name)
                else (deco.attr if isinstance(deco, ast.Attribute) else None)
            )
            if name in self._ABSTRACT_DECOS:
                return True
        return False

    @staticmethod
    def _unparse(node) -> str:
        """@brief Convert an AST expression back to source text."""
        try:
            return ast.unparse(node)
        except Exception:
            return ""


# ============================================================
# RUST LANGUAGE — doc-comment + declaration heuristic
# ============================================================


class RustLanguage(Language):
    """@brief Parse Rust items from `///` and `/** ... */` doc comments."""

    extensions = (".rs",)

    _VIS = r"(?:pub(?:\([^)]*\))?\s+)?"
    _DOC_LINE = re.compile(r"^\s*///\s?(.*)$")
    _ATTR = re.compile(r"^\s*#\[.*\]\s*$")

    _FN = re.compile(
        r"^\s*" + _VIS + r"(?:(?:const|async|unsafe|extern(?:\s+\"[^\"]*\")?)\s+)*"
        r"fn\s+(\w+)\s*(?:<[^>]*>)?\s*\((.*?)\)"
        r"(?:\s*->\s*([^{;]+?))?\s*(?:\{|;|where|$)"
    )
    _STRUCT = re.compile(r"^\s*" + _VIS + r"struct\s+(\w+)")
    _ENUM = re.compile(r"^\s*" + _VIS + r"enum\s+(\w+)")
    _TRAIT = re.compile(
        r"^\s*" + _VIS + r"(?:unsafe\s+)?trait\s+(\w+)"
        r"(?:\s*:\s*([^{;]+?))?\s*(?:\{|;|$)"
    )
    _IMPL = re.compile(r"^\s*impl(?:\s*<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)")
    _VARIANT = re.compile(r"^\s*(?:#\[.*\]\s*)?(\w+)\s*(?:[({,]|$)")
    _FIELD = re.compile(r"^(pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*(.+?),?\s*$")
    _USE = re.compile(r"^\s*(?:pub\s+)?use\s+([^;]+);")
    _MOD = re.compile(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;")
    _EXTERN = re.compile(r"^\s*extern\s+crate\s+(\w+)")
    _CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
    _CALL_STOP = {
        "if", "for", "while", "match", "loop", "return", "let", "fn", "impl",
        "struct", "enum", "trait", "use", "mod", "as", "in", "where", "move",
        "async", "await", "dyn", "Some", "None", "Ok", "Err", "Self", "self",
        "super", "crate",
    }

    def parse(self, source: str, filename: str) -> list[Entity]:
        """@brief Parse Rust source text into entities."""
        lines = source.splitlines()
        module = Entity(EntityKind.MODULE, filename)
        module.line = 1
        module.includes = self._includes(lines)
        entities: list[Entity] = [module]
        by_name: dict[str, Entity] = {}
        pending: list[str] = []
        i = 0

        while i < len(lines):
            line = lines[i]

            m = self._DOC_LINE.match(line)
            if m:
                pending.append(m.group(1))
                i += 1
                continue

            if "/**" in line:
                block, consumed = self._read_block_comment(lines, i)
                if consumed:
                    pending.extend(block)
                    i += consumed
                    continue

            stripped = line.strip()
            if not stripped or stripped.startswith("//") or self._ATTR.match(line):
                i += 1
                continue

            matched = False
            for pattern, handler in (
                (self._FN, self._parse_fn),
                (self._STRUCT, self._parse_struct),
                (self._ENUM, self._parse_enum),
                (self._TRAIT, self._parse_trait),
            ):
                m = pattern.match(line)
                if not m:
                    continue
                entity, consumed = handler(m, lines, i, pending)
                entities.append(entity)
                by_name[entity.name] = entity
                i += consumed
                pending = []
                matched = True
                break
            if matched:
                continue

            m = self._IMPL.match(line)
            if m:
                consumed = self.consume_block(lines, i)
                inner = lines[i + 1 : i + consumed - 1]
                trait, target = m.group(1), m.group(2)
                methods = self._parse_impl_body(inner)
                if target in by_name:
                    by_name[target].members.extend(methods)
                    if trait and trait not in by_name[target].implements:
                        by_name[target].implements.append(trait)
                i += consumed
                pending = []
                continue

            pending = []
            i += 1

        return entities

    def _parse_fn(self, m, lines, start, doc_lines):
        """@brief Build a function entity from a matched `fn` declaration."""
        name = m.group(1)
        args = self._split_args(m.group(2) or "")
        ret = (m.group(3) or "").strip()
        sig = f"fn {name}({', '.join(args)})" + (f" -> {ret}" if ret else "")
        doc = DOC_PARSER.parse("\n".join(doc_lines))
        entity = Entity(EntityKind.FUNCTION, name, sig, ret, doc)
        consumed = self.consume_block(lines, start)
        entity.calls = self._calls_from_lines(lines[start + 1 : start + consumed - 1])
        entity.line = start + 1
        return entity, consumed

    def _parse_struct(self, m, lines, start, doc_lines):
        """@brief Build a struct entity and parse its fields."""
        name = m.group(1)
        doc = DOC_PARSER.parse("\n".join(doc_lines))
        entity = Entity(EntityKind.STRUCT, name, doc=doc)
        consumed = self.consume_block(lines, start)
        body = lines[start + 1 : start + consumed - 1]
        entity.fields = self._parse_rust_fields(body)
        entity.line = start + 1
        return entity, consumed

    def _parse_enum(self, m, lines, start, doc_lines):
        """@brief Build an enum entity and parse its variants."""
        name = m.group(1)
        doc = DOC_PARSER.parse("\n".join(doc_lines))
        entity = Entity(EntityKind.ENUM, name, doc=doc)
        consumed = self.consume_block(lines, start)
        body = lines[start + 1 : start + consumed - 1]
        entity.variants = self._parse_variants(body)
        entity.line = start + 1
        return entity, consumed

    def _parse_trait(self, m, lines, start, doc_lines):
        """@brief Build a trait entity and parse its method signatures."""
        name = m.group(1)
        doc = DOC_PARSER.parse("\n".join(doc_lines))
        entity = Entity(EntityKind.TRAIT, name, doc=doc)
        bounds = (m.group(2) or "").strip()
        entity.bases = [b.strip() for b in bounds.split("+") if b.strip()]
        consumed = self.consume_block(lines, start)
        body = lines[start + 1 : start + consumed - 1]
        entity.members = self._parse_impl_body(body)
        entity.line = start + 1
        return entity, consumed

    def _parse_rust_fields(self, lines: list[str]) -> list[Field]:
        """@brief Extract named struct fields from a Rust struct body."""
        out, pending = [], []
        for raw in lines:
            m = self._DOC_LINE.match(raw)
            if m:
                pending.append(m.group(1))
                continue
            stripped = re.sub(r"//.*$", "", raw).strip()
            if not stripped or stripped.startswith("#"):
                continue
            m = self._FIELD.match(stripped)
            if not m:
                pending = []
                continue
            visibility = "public" if m.group(1) else "private"
            out.append(
                Field(
                    name=m.group(2),
                    type=m.group(3).strip().rstrip(","),
                    visibility=visibility,
                    description=" ".join(pending),
                )
            )
            pending = []
        return out

    def _parse_variants(self, lines: list[str]) -> list[Variant]:
        """@brief Extract enum variants from an enum body."""
        out, pending = [], []
        for raw in lines:
            line = raw.strip()
            if not line:
                continue
            m = self._DOC_LINE.match(raw)
            if m:
                pending.append(m.group(1))
                continue
            m = self._VARIANT.match(line)
            if m:
                name = m.group(1)
                if name and name[0].isalpha():
                    out.append(Variant(name, " ".join(pending)))
                    pending = []
        return out

    def _parse_impl_body(self, lines: list[str]) -> list[Entity]:
        """@brief Extract methods from an `impl` or trait body."""
        out, pending = [], []
        i = 0
        while i < len(lines):
            raw = lines[i]
            m = self._DOC_LINE.match(raw)
            if m:
                pending.append(m.group(1))
                i += 1
                continue
            stripped = raw.strip()
            if not stripped or stripped.startswith("//") or self._ATTR.match(raw):
                i += 1
                continue
            m = self._FN.match(stripped)
            if m:
                name = m.group(1)
                args = self._split_args(m.group(2) or "")
                ret = (m.group(3) or "").strip()
                sig = f"fn {name}({', '.join(args)})" + (f" -> {ret}" if ret else "")
                doc = DOC_PARSER.parse("\n".join(pending))
                consumed = self.consume_block(lines, i)
                block = lines[i : i + consumed]
                fn = Entity(EntityKind.METHOD, name, sig, ret, doc)
                fn.abstract = not any("{" in ln for ln in block)
                fn.calls = self._calls_from_lines(lines[i + 1 : i + consumed - 1])
                out.append(fn)
                pending = []
                i += consumed
                continue
            pending = []
            i += 1
        return out

    @classmethod
    def _includes(cls, lines: list[str]) -> list[str]:
        """@brief Collect `use`, `mod`, and `extern crate` targets."""
        out: list[str] = []
        for raw in lines:
            m = cls._USE.match(raw)
            if m:
                target = m.group(1).split("{")[0].rstrip(":, ").strip()
            else:
                m = cls._MOD.match(raw) or cls._EXTERN.match(raw)
                target = m.group(1) if m else ""
            if target and target not in out:
                out.append(target)
        return out

    @classmethod
    def _calls_from_lines(cls, lines: list[str]) -> list[str]:
        """@brief Extract called function names from Rust body lines."""
        out: list[str] = []
        for raw in lines:
            code = re.sub(r"//.*$", "", raw)
            for m in cls._CALL.finditer(code):
                name = m.group(1)
                if name not in cls._CALL_STOP and name not in out:
                    out.append(name)
        return out

    @staticmethod
    def _split_args(raw: str) -> list[str]:
        """@brief Split a parameter list on top-level commas."""
        parts, depth, buf = [], 0, ""
        for ch in raw:
            if ch in "<([":
                depth += 1
            elif ch in ">)]":
                depth -= 1
            if ch == "," and depth == 0:
                parts.append(buf.strip())
                buf = ""
            else:
                buf += ch
        if buf.strip():
            parts.append(buf.strip())
        return parts


# ============================================================
# C++ LANGUAGE — doc-comment + column-0 declaration heuristic
# ============================================================


class CppLanguage(Language):
    """@brief Parse C++ declarations from `///` and `/** ... */` comments."""

    extensions = (".cpp", ".hpp", ".h", ".cc", ".cxx", ".hh")

    _DOC_LINE = re.compile(r"^\s*///\s?(.*)$")
    _ATTR = re.compile(r"^\s*\[\[.*\]\]\s*$")

    _FN = re.compile(
        r"^(?:template\s*<[^>]*>\s*)?"
        r"(?:(?:virtual|static|inline|constexpr|explicit|friend)\s+)*"
        r"([\w:]+(?:\s*<[^>]*>)?(?:\s*[*&])?)\s+"
        r"(?:(\w+)::)?(\w+)\s*"
        r"\(([^)]*)\)"
        r"\s*(?:const)?\s*(?:override)?\s*(?:=\s*0)?"
        r"\s*(?:\{|;)"
    )
    _CLASS = re.compile(
        r"^(?:template\s*<[^>]*>\s*)?"
        r"(class|struct)\s+(\w+)\s*(?::\s*([^{]+))?\s*\{"
    )
    _ENUM = re.compile(r"^(?:enum(?:\s+class)?)\s+(\w+)\s*\{")
    _ABSTRACT = re.compile(r"=\s*0\s*;")
    _FIELD = re.compile(
        r"^(?:(?:static|mutable|constexpr|inline|const)\s+)*"
        r"([\w:<>,\s\*&]+?)\s+(\w+)\s*(?:\[[^\]]*\])?\s*(?:=\s*[^;]+)?;$"
    )
    _INCLUDE = re.compile(r'^\s*#\s*include\s*[<"]([^>"]+)[>"]')
    _CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
    _CALL_STOP = {
        "alignof", "case", "catch", "const_cast", "decltype", "delete", "do",
        "dynamic_cast", "else", "for", "if", "new", "noexcept",
        "reinterpret_cast", "return", "sizeof", "static_cast", "switch",
        "throw", "while",
    }

    def parse(self, source: str, filename: str) -> list[Entity]:
        """@brief Parse C++ source text into entities."""
        lines = source.splitlines()
        module = Entity(EntityKind.MODULE, filename)
        module.line = 1
        module.includes = self._includes(lines)
        pending: list[str] = []
        entities: list[Entity] = [module]
        by_name: dict[str, Entity] = {}
        i = 0

        while i < len(lines):
            raw = lines[i]

            m = self._DOC_LINE.match(raw)
            if m:
                pending.append(m.group(1))
                i += 1
                continue

            if "/**" in raw:
                block, consumed = self._read_block_comment(lines, i)
                if consumed:
                    pending.extend(block)
                    i += consumed
                    continue

            stripped = raw.strip()
            if not stripped or stripped.startswith(("//", "/*", "#")):
                i += 1
                continue

            m = self._FN.match(raw)
            if m:
                ret = m.group(1).strip()
                cls = m.group(2)
                name = m.group(3)
                args_raw = m.group(4)
                full_name = f"{cls}::{name}" if cls else name
                sig = f"{ret} {full_name}({args_raw.strip()})"
                params = self._parse_cpp_params(args_raw)
                doc = DOC_PARSER.parse("\n".join(pending))
                self._merge_types(doc, params)
                abstract = bool(self._ABSTRACT.search(raw))
                consumed = self.consume_block(lines, i)
                calls = self._calls_from_lines(lines[i + 1 : i + consumed - 1])

                if cls:
                    ent = Entity(EntityKind.METHOD, full_name, sig, ret, doc, abstract)
                    ent.line = i + 1
                    ent.calls = calls
                    if cls in by_name:
                        by_name[cls].members.append(ent)
                    else:
                        entities.append(ent)
                else:
                    ent = Entity(EntityKind.FUNCTION, name, sig, ret, doc, abstract)
                    ent.line = i + 1
                    ent.calls = calls
                    entities.append(ent)
                    by_name[name] = ent
                i += consumed
                pending = []
                continue

            m = self._CLASS.match(raw)
            if m:
                keyword, name, base_clause = m.group(1), m.group(2), m.group(3)
                kind = EntityKind.CLASS if keyword == "class" else EntityKind.STRUCT
                default_visibility = (
                    "public" if kind == EntityKind.STRUCT else "private"
                )
                doc = DOC_PARSER.parse("\n".join(pending))
                consumed = self.consume_block(lines, i)
                body = lines[i + 1 : i + consumed - 1]
                methods, fields = self._parse_class_body(body, default_visibility)
                abstract = any(mt.abstract for mt in methods)
                ent = Entity(
                    kind, name, doc=doc, abstract=abstract,
                    members=methods, fields=fields,
                )
                ent.bases = self._base_names(base_clause)
                ent.line = i + 1
                entities.append(ent)
                by_name[name] = ent
                i += consumed
                pending = []
                continue

            m = self._ENUM.match(raw)
            if m:
                name = m.group(1)
                doc = DOC_PARSER.parse("\n".join(pending))
                consumed = self.consume_block(lines, i)
                body = lines[i + 1 : i + consumed - 1]
                variants = [
                    Variant(v.strip(), "")
                    for line in body
                    for v in line.split(",")
                    if v.strip() and v.strip().isidentifier()
                ]
                ent = Entity(EntityKind.ENUM, name, doc=doc, variants=variants)
                ent.line = i + 1
                entities.append(ent)
                i += consumed
                pending = []
                continue

            pending = []
            i += 1

        return entities

    def _parse_class_body(
        self, lines: list[str], default_visibility: str = "public"
    ) -> tuple[list[Entity], list[Field]]:
        """@brief Extract method and field declarations from a class body."""
        methods: list[Entity] = []
        fields: list[Field] = []
        pending: list[str] = []
        visibility = default_visibility
        i = 0
        while i < len(lines):
            raw = lines[i]
            m = self._DOC_LINE.match(raw)
            if m:
                pending.append(m.group(1))
                i += 1
                continue
            stripped = re.sub(r"//.*$", "", raw).strip()
            if not stripped or stripped.startswith("/*"):
                i += 1
                continue
            access = re.match(r"^(public|protected|private)\s*:\s*$", stripped)
            if access:
                visibility = access.group(1)
                pending = []
                i += 1
                continue
            if stripped.startswith(("using ", "typedef ", "friend ", "template ")):
                pending = []
                i += 1
                continue
            m = self._FN.match(stripped)
            if m:
                ret = m.group(1).strip()
                name = m.group(3)
                sig = f"{ret} {name}({m.group(4).strip()})"
                abstract = bool(self._ABSTRACT.search(stripped))
                doc = DOC_PARSER.parse("\n".join(pending))
                consumed = self.consume_block(lines, i)
                entity = Entity(EntityKind.METHOD, name, sig, ret, doc, abstract)
                entity.calls = self._calls_from_lines(
                    lines[i + 1 : i + consumed - 1]
                )
                methods.append(entity)
                pending = []
                i += consumed
                continue
            if "(" in stripped or "{" in stripped or "}" in stripped:
                pending = []
                i += 1
                continue
            m = self._FIELD.match(stripped)
            if m:
                type_ = m.group(1).strip()
                if type_ not in ("return", "delete"):
                    fields.append(
                        Field(
                            name=m.group(2),
                            type=type_,
                            visibility=visibility,
                            static=bool(re.match(r"^static\b", stripped)),
                            description=" ".join(pending),
                        )
                    )
                pending = []
                i += 1
                continue
            pending = []
            i += 1
        return methods, fields

    @staticmethod
    def _base_names(raw: str | None) -> list[str]:
        """@brief Extract base-class names from a C++ base clause."""
        names: list[str] = []
        if not raw:
            return names
        for part in raw.split(","):
            part = re.sub(r"\b(public|protected|private|virtual)\b", "", part)
            part = part.split("<")[0].strip()
            name = part.split("::")[-1].strip()
            if name and name not in names:
                names.append(name)
        return names

    @classmethod
    def _includes(cls, lines: list[str]) -> list[str]:
        """@brief Collect `#include` targets from C++ source lines."""
        out: list[str] = []
        for raw in lines:
            m = cls._INCLUDE.match(raw)
            if m and m.group(1) not in out:
                out.append(m.group(1))
        return out

    @classmethod
    def _calls_from_lines(cls, lines: list[str]) -> list[str]:
        """@brief Extract called function names from C++ body lines."""
        out: list[str] = []
        for raw in lines:
            code = re.sub(r"//.*$", "", raw)
            for m in cls._CALL.finditer(code):
                name = m.group(1)
                if name not in cls._CALL_STOP and name not in out:
                    out.append(name)
        return out

    @staticmethod
    def _parse_cpp_params(raw: str) -> dict[str, str]:
        """@brief Infer parameter types from a C++ parameter list."""
        types: dict[str, str] = {}
        for part in raw.split(","):
            part = part.strip()
            if not part or part == "void":
                continue
            tokens = part.replace("&", " ").replace("*", " ").split()
            if not tokens:
                continue
            name = tokens[-1]
            if "=" in name:
                name = name.split("=")[0].strip()
            typ = " ".join(tokens[:-1]).strip()
            if name.isidentifier():
                types[name] = typ
        return types

    @staticmethod
    def _merge_types(doc: Doc, types: dict[str, str]) -> None:
        """@brief Fill missing parameter types from a declaration."""
        for p in doc.params:
            p.type = p.type or types.get(p.name, "")


# ============================================================
# RENDERER — uniform reference layout
# ============================================================


class ReferenceRenderer:
    """@brief Render a parsed `Entity` tree as Markdown."""

    def render(self, entities: list[Entity]) -> str:
        """@brief Render top-level entities as one Markdown document."""
        return "\n\n---\n\n".join(e for e in (self._entity(en) for en in entities) if e)

    def _entity(self, entity: Entity) -> str:
        """@brief Dispatch one entity to its kind-specific renderer."""
        if entity.kind == EntityKind.ENUM:
            return self._enum(entity)
        if entity.kind in (
            EntityKind.CLASS,
            EntityKind.STRUCT,
            EntityKind.TRAIT,
            EntityKind.INTERFACE,
        ):
            return self._class(entity)
        if entity.kind == EntityKind.MODULE:
            parts = []
            if entity.doc.brief:
                parts.append(entity.doc.brief)
            if entity.doc.details:
                parts.append(entity.doc.details)
            if entity.doc.examples:
                parts.append(self._example_section(entity))
            return "\n\n".join(parts)
        return self._function(entity)

    def _function(self, e: Entity) -> str:
        """@brief Render a free function reference."""
        out = [f"## `{e.name}()`"]
        if e.signature:
            out.append(f"```\n{e.signature}\n```")
        out.append(e.doc.brief or "")
        if e.doc.details:
            out.append(f"### Details\n\n{e.doc.details}")
        if e.doc.params:
            out.append(self._params_table(e))
        if e.doc.returns:
            out.append(f"### Returns\n\n{e.doc.returns}")
        if e.doc.throws:
            out.append(self._throws_table(e))
        if e.doc.notes:
            out.append("### Notes\n\n" + "\n\n".join(f"> {n}" for n in e.doc.notes))
        if e.doc.warnings:
            out.append(
                "### Warnings\n\n" + "\n\n".join(f"> {w}" for w in e.doc.warnings)
            )
        if e.doc.examples:
            out.append(self._example_section(e))
        if e.doc.see:
            out.append("### See Also\n\n" + "\n".join(f"- `{s}`" for s in e.doc.see))
        return "\n\n".join(p for p in out if p)

    def _class(self, e: Entity) -> str:
        """@brief Render a class, struct, trait, or interface reference."""
        title = f"`{e.name}` {e.kind.value.title()} Reference"
        if e.abstract:
            title += " *abstract*"
        out = [f"## {title}"]
        if e.doc.brief:
            out.append(e.doc.brief)
        if e.doc.details:
            out.append(e.doc.details)
        relations = []
        if e.bases:
            relations.append(
                "**Inherits:** " + ", ".join(f"`{b}`" for b in e.bases)
            )
        if e.implements:
            relations.append(
                "**Implements:** " + ", ".join(f"`{i}`" for i in e.implements)
            )
        if relations:
            out.append("\n\n".join(relations))
        if e.doc.examples:
            out.append(self._example_section(e))
        if e.fields:
            out.append("### Fields\n\n" + self._fields_table(e.fields))
        if e.members:
            out.append(
                "### Public Member Functions\n\n" + self._members_table(e.members)
            )
            contracts = [m for m in e.members if self._needs_contract(m)]
            if contracts:
                out.append(
                    "### Method Contracts\n\n"
                    + "\n\n".join(self._member_contract(m) for m in contracts)
                )
        return "\n\n".join(out)

    @staticmethod
    def _needs_contract(member: Entity) -> bool:
        """@brief Check whether a member has more than a brief to render."""
        doc = member.doc
        return any((
            doc.details, doc.params, doc.returns, doc.throws,
            doc.notes, doc.warnings, doc.examples, doc.see,
        ))

    @classmethod
    def _member_contract(cls, member: Entity) -> str:
        """@brief Render the detailed contract for one class member."""
        parts = [f"#### `{member.name}()`"]
        if member.doc.details:
            parts.append(member.doc.details)
        if member.doc.params:
            parts.append("**Parameters**\n\n" + cls._params_rows(member.doc.params))
        if member.doc.returns:
            parts.append(f"**Returns:** {member.doc.returns}")
        if member.doc.throws:
            parts.append("**Exceptions**\n\n" + cls._throws_rows(member.doc.throws))
        if member.doc.notes:
            parts.append("**Notes**\n\n" + "\n\n".join(f"> {n}" for n in member.doc.notes))
        if member.doc.warnings:
            parts.append("**Warnings**\n\n" + "\n\n".join(f"> {w}" for w in member.doc.warnings))
        if member.doc.examples:
            parts.append(cls._example_section(member, "#### Example"))
        if member.doc.see:
            parts.append("**See Also:** " + ", ".join(f"`{s}`" for s in member.doc.see))
        return "\n\n".join(parts)

    @staticmethod
    def _example_section(e: Entity, heading: str = "### Example") -> str:
        """@brief Render example blocks from an entity."""
        parts = [heading]
        for ex in e.doc.examples:
            content = ex.strip("\n")
            if not content.strip():
                continue
            if "```" in content:
                parts.append(content)
            else:
                parts.append(f"```\n{content}\n```")
        return "\n\n".join(parts) if len(parts) > 1 else ""

    def _members_table(self, members: list[Entity]) -> str:
        """@brief Render the member summary table for a class-like entity."""
        rows = [
            "| Return Type | Signature & Description |",
            "|-------------|-------------------------|",
        ]
        for m in members:
            prefix = "abstract " if m.abstract else ""
            cell = f"{prefix}`{m.signature}`"
            if m.doc.brief:
                cell += f"<br>{m.doc.brief}"
            rows.append(
                f"| `{self._cell(m.return_type or '')}` | {self._cell(cell)} |"
            )
        return "\n".join(rows)

    @staticmethod
    def _fields_table(fields: list[Field]) -> str:
        """@brief Render the data-member table for a class-like entity."""
        rows = [
            "| Visibility | Type | Name | Description |",
            "|------------|------|------|-------------|",
        ]
        for field_ in fields:
            flags = "static " if field_.static else ""
            rows.append(
                f"| {flags}{ReferenceRenderer._cell(field_.visibility)} "
                f"| `{ReferenceRenderer._cell(field_.type)}` "
                f"| `{ReferenceRenderer._cell(field_.name)}` "
                f"| {ReferenceRenderer._cell(field_.description)} |"
            )
        return "\n".join(rows)

    def _enum(self, e: Entity) -> str:
        """@brief Render an enum reference."""
        out = [f"## `{e.name}` Enum Reference"]
        if e.doc.brief:
            out.append(e.doc.brief)
        if e.doc.details:
            out.append(e.doc.details)
        if e.variants:
            rows = ["| Enumerator | Description |", "|------------|-------------|"]
            for v in e.variants:
                rows.append(f"| `{self._cell(v.name)}` | {self._cell(v.description)} |")
            out.append("### Enumerators\n\n" + "\n".join(rows))
        return "\n\n".join(out)

    @staticmethod
    def _cell(value: str) -> str:
        """@brief Escape text for use inside a Markdown table cell."""
        return value.replace("|", "\\|")

    @staticmethod
    def _params_table(e: Entity) -> str:
        """@brief Render a function parameter table with a heading."""
        return "### Parameters\n\n" + ReferenceRenderer._params_rows(e.doc.params)

    @staticmethod
    def _params_rows(params: list[Param]) -> str:
        """@brief Render parameter rows as a Markdown table."""
        rows = [
            "| Direction | Name | Type | Description |",
            "|-----------|------|------|-------------|",
        ]
        for p in params:
            rows.append(
                f"| [{ReferenceRenderer._cell(p.direction)}] "
                f"| `{ReferenceRenderer._cell(p.name)}` "
                f"| `{ReferenceRenderer._cell(p.type)}` "
                f"| {ReferenceRenderer._cell(p.description)} |"
            )
        return "\n".join(rows)

    @staticmethod
    def _throws_table(e: Entity) -> str:
        """@brief Render a function exception table with a heading."""
        return "### Exceptions\n\n" + ReferenceRenderer._throws_rows(e.doc.throws)

    @staticmethod
    def _throws_rows(throws: list[tuple[str, str]]) -> str:
        """@brief Render exception rows as a Markdown table."""
        rows = ["| Exception | Description |", "|-----------|-------------|"]
        for exc, desc in throws:
            rows.append(
                f"| `{ReferenceRenderer._cell(exc)}` "
                f"| {ReferenceRenderer._cell(desc)} |"
            )
        return "\n".join(rows)


# ============================================================
# MERMAID RENDERER
# ============================================================


class MermaidRenderer:
    """@brief Render parsed entities as Mermaid diagrams."""

    KINDS = (
        "class",
        "inheritance",
        "collaboration",
        "call",
        "caller",
        "include",
        "directory",
    )

    TITLES = {
        "class":         "Class Diagram",
        "inheritance":   "Inheritance Diagram",
        "collaboration": "Collaboration Diagram",
        "call":          "Call Graph",
        "caller":        "Caller Graph",
        "include":       "Include Graph",
        "directory":     "Directory Diagram",
    }

    _BLOCK_KINDS = (
        EntityKind.CLASS,
        EntityKind.STRUCT,
        EntityKind.TRAIT,
        EntityKind.INTERFACE,
        EntityKind.ENUM,
    )
    _VISIBILITY = {
        "public": "+",
        "protected": "#",
        "private": "-",
        "package": "~",
    }
    _IDENTIFIER = re.compile(r"[^A-Za-z0-9_]")
    _TYPE_TOKEN = re.compile(r"[A-Za-z_]\w*")
    _BUILTIN_TYPES = {
        "bool", "char", "double", "f32", "f64", "float", "i8", "i16", "i32",
        "i64", "i128", "int", "isize", "long", "Self", "short", "signed",
        "str", "String", "u8", "u16", "u32", "u64", "u128", "unsigned",
        "usize", "void",
    }

    def render(self, entities: list[Entity], kind: str = "class") -> str:
        """@brief Render entities using the selected diagram kind."""
        renderers = {
            "class": self.class_diagram,
            "inheritance": self.inheritance_diagram,
            "collaboration": self.collaboration_diagram,
            "call": self.call_graph,
            "caller": self.caller_graph,
            "include": self.include_graph,
            "directory": self.directory_graph,
        }
        try:
            renderer = renderers[kind]
        except KeyError:
            choices = ", ".join(self.KINDS)
            raise ValueError(f"Unknown diagram kind {kind!r}: {choices}") from None
        return renderer(entities)

    def render_all(
        self, entities: list[Entity], kinds: Iterable[str] | None = None
    ) -> str:
        """@brief Render several diagrams into one Markdown document.

        @details Each diagram becomes a level-2 heading followed by a fenced
        ` ```mermaid ` block. This is the single-file layout produced by
        `diagram -k all`.

        @param[in] entities Top-level entities in source order.
        @param[in] kinds Diagram kinds to include, or `None` for every kind
        in `KINDS`.
        @return A complete Markdown document.
        @example
        md = MermaidRenderer().render_all(entities)
        Path("GRAPH.md").write_text(md)
        """
        selected = tuple(kinds) if kinds is not None else self.KINDS
        sections = ["# Contract Graph\n"]
        for kind in selected:
            title = self.TITLES.get(kind, kind.title())
            mermaid = self.render(entities, kind).rstrip()
            sections.append(f"## {title}\n\n```mermaid\n{mermaid}\n```\n")
        return "\n".join(sections)

    def class_diagram(self, entities: list[Entity]) -> str:
        """@brief Render the full class diagram."""
        return self._class_diagram(entities, detailed=True, dependencies=True)

    def inheritance_diagram(self, entities: list[Entity]) -> str:
        """@brief Render only the class inheritance tree."""
        return self._class_diagram(
            entities, detailed=False, dependencies=False, include_enums=False
        )

    def _class_diagram(
        self,
        entities: list[Entity],
        detailed: bool,
        dependencies: bool,
        include_enums: bool = True,
    ) -> str:
        """@brief Render a configurable Mermaid class diagram."""
        nodes = self._collect(entities, include_enums=include_enums)
        ids, by_name, by_file = self._identifiers(nodes)
        lines = ["classDiagram"]
        for entity in nodes:
            lines.extend(self._block(entity, ids, detailed=detailed))
        lines.extend(self._class_edges(nodes, ids, by_name, by_file, dependencies))
        return "\n".join(lines) + "\n"

    def _collect(self, entities: Iterable[Entity], include_enums: bool = True) -> list[Entity]:
        """@brief Flatten block-like entities nested under members."""
        nodes: list[Entity] = []
        for entity in entities:
            if entity.kind in self._BLOCK_KINDS:
                if include_enums or entity.kind != EntityKind.ENUM:
                    nodes.append(entity)
            nodes.extend(self._collect(entity.members, include_enums))
        return nodes

    def _identifiers(
        self, nodes: list[Entity]
    ) -> tuple[dict[int, str], dict[str, str], dict[tuple[str, str], str]]:
        """@brief Assign unique Mermaid identifiers to diagram nodes."""
        ids: dict[int, str] = {}
        by_name: dict[str, str] = {}
        by_file: dict[tuple[str, str], str] = {}
        used: dict[str, int] = {}
        for entity in nodes:
            base = self._IDENTIFIER.sub("_", entity.name) or "entity"
            count = used.get(base, 0) + 1
            used[base] = count
            identifier = base if count == 1 else f"{base}_{count}"
            ids[id(entity)] = identifier
            by_name.setdefault(entity.name, identifier)
            by_file.setdefault((entity.path, entity.name), identifier)
        return ids, by_name, by_file

    def _block(self, entity: Entity, ids: dict[int, str], detailed: bool = True) -> list[str]:
        """@brief Render one entity as a Mermaid class block."""
        lines = [f"  class {ids[id(entity)]} {{"]
        stereotype = self._stereotype(entity)
        if stereotype:
            lines.append(f"    {stereotype}")
        if detailed:
            for field_ in entity.fields:
                marker = self._VISIBILITY.get(field_.visibility, "+")
                type_ = self._display_type(field_.type)
                text = f"{type_} {field_.name}" if type_ else field_.name
                suffix = "$" if field_.static else ""
                lines.append(f"    {marker}{text}{suffix}")
            for member in entity.members:
                if member.kind not in (EntityKind.METHOD, EntityKind.FUNCTION):
                    continue
                lines.append(f"    +{member.name}()")
            if entity.kind == EntityKind.ENUM:
                for variant in entity.variants:
                    lines.append(f"    {variant.name}")
        lines.append("  }")
        return lines

    @staticmethod
    def _stereotype(entity: Entity) -> str:
        """@brief Choose the Mermaid stereotype for an entity."""
        if entity.kind == EntityKind.ENUM:
            return "<<enumeration>>"
        if entity.kind in (EntityKind.TRAIT, EntityKind.INTERFACE):
            return "<<interface>>"
        if entity.abstract:
            return "<<abstract>>"
        return ""

    @staticmethod
    def _display_type(raw: str) -> str:
        """@brief Normalize a type for Mermaid class-member syntax."""
        text = raw.replace("&", "").replace("'", "").replace("|", " or ")
        text = text.replace("<", "~").replace(">", "~")
        text = text.replace("[", "~").replace("]", "~")
        return re.sub(r"\s+", " ", text).strip()

    def _class_edges(
        self,
        nodes: list[Entity],
        ids: dict[int, str],
        by_name: dict[str, str],
        by_file: dict[tuple[str, str], str],
        dependencies: bool = True,
    ) -> list[str]:
        """@brief Render inheritance, realization, and dependency edges."""
        edges: list[str] = []
        seen: set[str] = set()

        def add(line: str) -> None:
            if line not in seen:
                seen.add(line)
                edges.append(line)

        for entity in nodes:
            child = ids[id(entity)]
            for base in entity.bases:
                parent = self._lookup(base, by_name, by_file, entity.path)
                if parent and parent != child:
                    add(f"  {parent} <|-- {child}")
            for interface in entity.implements:
                parent = self._lookup(interface, by_name, by_file, entity.path)
                if parent and parent != child:
                    add(f"  {parent} <|.. {child}")
            if not dependencies:
                continue
            for field_ in entity.fields:
                for type_name in self._type_names(field_.type):
                    target = self._lookup(type_name, by_name, by_file, entity.path)
                    if target and target != child:
                        add(f"  {child} ..> {target}")
        return edges

    @staticmethod
    def _lookup(
        name: str,
        by_name: dict[str, str],
        by_file: dict[tuple[str, str], str],
        path: str = "",
    ) -> str | None:
        """@brief Resolve a relation name to a known identifier."""
        simple = name.split("<")[0].split("::")[-1].strip()
        if "." in simple:
            qualified = by_file.get((path, simple)) or by_name.get(simple)
            if qualified:
                return qualified
            simple = simple.rsplit(".", 1)[-1]
        return by_file.get((path, simple)) or by_name.get(simple)

    def _type_names(self, raw: str) -> list[str]:
        """@brief Extract candidate entity names from a type expression."""
        return [
            token
            for token in self._TYPE_TOKEN.findall(raw)
            if token not in self._BUILTIN_TYPES
        ]

    def collaboration_diagram(self, entities: list[Entity]) -> str:
        """@brief Render which classes use which other classes."""
        nodes = self._collect(entities)
        ids, by_name, by_file = self._identifiers(nodes)
        lines = ["flowchart LR"]
        for entity in nodes:
            lines.append(f'  {ids[id(entity)]}["{self._label(entity.name)}"]')
        edges: list[str] = []
        seen: set[str] = set()
        for entity in nodes:
            source = ids[id(entity)]
            for type_name in self._used_types(entity):
                target = self._lookup(type_name, by_name, by_file, entity.path)
                if not target or target == source:
                    continue
                edge = f"  {source} -->|uses| {target}"
                if edge not in seen:
                    seen.add(edge)
                    edges.append(edge)
        lines.extend(edges)
        return "\n".join(lines) + "\n"

    def _used_types(self, entity: Entity) -> list[str]:
        """@brief Collect type names referenced by a class-like entity."""
        names: list[str] = []
        for field_ in entity.fields:
            names.extend(self._type_names(field_.type))
        for member in entity.members:
            names.extend(self._type_names(member.return_type))
            for param in member.doc.params:
                names.extend(self._type_names(param.type))
        out: list[str] = []
        for name in names:
            if name not in out:
                out.append(name)
        return out

    def call_graph(self, entities: list[Entity]) -> str:
        """@brief Render caller to callee edges."""
        return self._call_diagram(entities, inverse=False)

    def caller_graph(self, entities: list[Entity]) -> str:
        """@brief Render callee to caller edges."""
        return self._call_diagram(entities, inverse=True)

    def _call_diagram(self, entities: list[Entity], inverse: bool) -> str:
        """@brief Render a call graph in either direction."""
        functions = self._functions(entities)
        ids, labels, by_name, by_file = self._function_nodes(functions)
        lines = ["flowchart LR"]
        for entity, _owner in functions:
            lines.append(f'  {ids[id(entity)]}["{self._label(labels[id(entity)])}"]')
        edges: list[str] = []
        seen: set[str] = set()
        for entity, owner in functions:
            source = ids[id(entity)]
            for call in entity.calls:
                target = self._lookup(
                    self._call_target(call, owner), by_name, by_file, entity.path
                )
                if not target or target == source:
                    continue
                a, b = (source, target) if not inverse else (target, source)
                label = "calls" if not inverse else "called by"
                edge = f"  {a} -->|{label}| {b}"
                if edge not in seen:
                    seen.add(edge)
                    edges.append(edge)
        lines.extend(edges)
        return "\n".join(lines) + "\n"

    def _functions(self, entities: Iterable[Entity], owner: str = "") -> list[tuple[Entity, str]]:
        """@brief Flatten functions and methods with their owning class."""
        out: list[tuple[Entity, str]] = []
        for entity in entities:
            if entity.kind in (EntityKind.FUNCTION, EntityKind.METHOD):
                out.append((entity, owner))
            if entity.kind in self._BLOCK_KINDS:
                out.extend(self._functions(entity.members, entity.name))
        return out

    def _function_nodes(
        self, functions: list[tuple[Entity, str]]
    ) -> tuple[dict[int, str], dict[int, str], dict[str, str], dict[tuple[str, str], str]]:
        """@brief Assign flowchart identifiers and labels to functions."""
        ids: dict[int, str] = {}
        labels: dict[int, str] = {}
        by_name: dict[str, str] = {}
        by_file: dict[tuple[str, str], str] = {}
        used: dict[str, int] = {}
        counts: dict[str, int] = {}
        file_counts: dict[tuple[str, str], int] = {}
        for entity, _owner in functions:
            counts[entity.name] = counts.get(entity.name, 0) + 1
            key = (entity.path, entity.name)
            file_counts[key] = file_counts.get(key, 0) + 1
        for entity, owner in functions:
            qualified = self._qualified(entity, owner)
            base = "fn_" + (self._IDENTIFIER.sub("_", qualified) or "call")
            count = used.get(base, 0) + 1
            used[base] = count
            identifier = base if count == 1 else f"{base}_{count}"
            ids[id(entity)] = identifier
            labels[id(entity)] = qualified
            if counts[entity.name] == 1:
                by_name.setdefault(entity.name, identifier)
            by_name.setdefault(qualified, identifier)
            if owner:
                by_name.setdefault(f"{owner}.{entity.name}", identifier)
            if file_counts[(entity.path, entity.name)] == 1:
                by_file.setdefault((entity.path, entity.name), identifier)
            by_file.setdefault((entity.path, qualified), identifier)
            if owner:
                by_file.setdefault((entity.path, f"{owner}.{entity.name}"), identifier)
        return ids, labels, by_name, by_file

    @staticmethod
    def _call_target(call: str, owner: str) -> str:
        """@brief Resolve `self` and `cls` call prefixes to the owning class."""
        prefix, dot, rest = call.partition(".")
        if dot and prefix in ("self", "cls") and owner:
            return f"{owner}.{rest}"
        return call

    @staticmethod
    def _qualified(entity: Entity, owner: str) -> str:
        """@brief Build a readable flowchart label for an entity."""
        parts = [part for part in (entity.module, owner, entity.name) if part]
        return ".".join(parts) or entity.name

    def include_graph(self, entities: list[Entity]) -> str:
        """@brief Render imports and includes between files."""
        modules = self._modules(entities)
        ids, by_key = self._module_index(modules)
        lines = ["flowchart LR"]
        for module in modules:
            label = Path(module.path).name or module.name
            lines.append(f'  {ids[id(module)]}["{self._label(label)}"]')
        external: dict[str, str] = {}
        edges: list[str] = []
        seen: set[str] = set()
        for module in modules:
            source = ids[id(module)]
            for target in module.includes:
                destination = self._resolve_module(target, by_key)
                if destination is None:
                    key = target.strip()
                    if key not in external:
                        external[key] = f"ext_{len(external)}"
                        lines.append(f'  {external[key]}["{self._label(key)}"]')
                    destination = external[key]
                if destination == source:
                    continue
                edge = f"  {source} --> {destination}"
                if edge not in seen:
                    seen.add(edge)
                    edges.append(edge)
        lines.extend(edges)
        return "\n".join(lines) + "\n"

    def directory_graph(self, entities: list[Entity]) -> str:
        """@brief Render dependencies between directories."""
        modules = self._modules(entities)
        ids, by_key = self._module_index(modules)
        file_dirs = {
            ids[id(module)]: Path(module.path).parent.as_posix()
            for module in modules
        }
        dirs = sorted(set(file_dirs.values()))
        dir_ids = {directory: f"dir_{i}" for i, directory in enumerate(dirs)}
        lines = ["flowchart LR"]
        for directory in dirs:
            lines.append(f'  {dir_ids[directory]}["{self._label(directory or ".")}"]')
        edges: list[str] = []
        seen: set[str] = set()
        for module in modules:
            source_dir = file_dirs[ids[id(module)]]
            for target in module.includes:
                destination = self._resolve_module(target, by_key)
                if destination is None:
                    continue
                target_dir = file_dirs.get(destination)
                if not target_dir or target_dir == source_dir:
                    continue
                edge = f"  {dir_ids[source_dir]} --> {dir_ids[target_dir]}"
                if edge not in seen:
                    seen.add(edge)
                    edges.append(edge)
        lines.extend(edges)
        return "\n".join(lines) + "\n"

    @staticmethod
    def _modules(entities: Iterable[Entity]) -> list[Entity]:
        """@brief Select module entities that represent source files."""
        return [entity for entity in entities if entity.kind == EntityKind.MODULE]

    def _module_index(self, modules: list[Entity]) -> tuple[dict[int, str], dict[str, str]]:
        """@brief Assign flowchart identifiers to files and index their names."""
        ids: dict[int, str] = {}
        by_key: dict[str, str] = {}
        used: dict[str, int] = {}
        for module in modules:
            path = Path(module.path)
            base = "file_" + (self._IDENTIFIER.sub("_", path.as_posix()) or "module")
            count = used.get(base, 0) + 1
            used[base] = count
            identifier = base if count == 1 else f"{base}_{count}"
            ids[id(module)] = identifier
            for key in (path.name, path.stem, path.as_posix(), module.module):
                if key:
                    by_key.setdefault(key, identifier)
        return ids, by_key

    def _resolve_module(self, target: str, by_key: dict[str, str]) -> str | None:
        """@brief Resolve an include target to a parsed file identifier."""
        for candidate in self._include_candidates(target):
            if candidate in by_key:
                return by_key[candidate]
        return None

    @staticmethod
    def _include_candidates(target: str) -> list[str]:
        """@brief Build lookup candidates for a raw dependency target."""
        raw = target.strip().strip("<>\"")
        raw = raw.split("{")[0].rstrip(":, ").strip()
        candidates = [raw, Path(raw).name, Path(raw).stem]
        normalized = raw.replace("::", ".").replace("/", ".")
        parts = [
            part
            for part in normalized.split(".")
            if part and part not in ("crate", "super", "self")
        ]
        for size in range(len(parts), 0, -1):
            candidates.append(".".join(parts[:size]))
            candidates.append("/".join(parts[:size]))
            candidates.append(parts[size - 1])
        return candidates

    @staticmethod
    def _label(text: str) -> str:
        """@brief Escape a label for a Mermaid flowchart node."""
        return text.replace('"', "'").replace("\n", " ")


# ============================================================
# CONVERTER
# ============================================================


class Converter:
    """@brief Orchestrate file discovery, language parsing, and rendering."""

    def __init__(self, cfg: Config) -> None:
        self.cfg = cfg
        self.renderer = ReferenceRenderer()

    def entities(self, path: Path) -> list[Entity]:
        """@brief Parse one source file into entities."""
        cls = LANGUAGES.get(path.suffix.lower())
        if cls is None:
            return []
        source = path.read_text(encoding=self.cfg.encoding)
        entities = cls().parse(source, path.name)
        for entity in entities:
            self._stamp(entity, path)
        return entities

    @staticmethod
    def _stamp(entity: Entity, path: Path) -> None:
        """@brief Attach source metadata to an entity tree."""
        entity.path = str(path)
        entity.module = path.stem
        for member in entity.members:
            Converter._stamp(member, path)

    def file(self, path: Path) -> str:
        """@brief Convert one source file to Markdown."""
        return self.renderer.render(self.entities(path))

    def project(self, paths: Iterable[Path]) -> list[Entity]:
        """@brief Parse several source files into one entity list."""
        entities: list[Entity] = []
        for path in paths:
            entities.extend(self.entities(path))
        return entities

    def diagram(self, paths: Iterable[Path], output: Path, kind: str = "class") -> None:
        """@brief Convert several source files into one Markdown diagram file.

        @details The output contains one fenced ` ```mermaid ` block per
        selected kind. `kind="all"` selects every kind in
        `MermaidRenderer.KINDS`.

        @param[in] paths Source files to convert.
        @param[in] output Destination Markdown file.
        @param[in] kind Diagram kind, or `"all"`.
        @return Nothing; output is written using `self.cfg.encoding`.
        """
        kinds = MermaidRenderer.KINDS if kind == "all" else (kind,)
        output.write_text(
            MermaidRenderer().render_all(self.project(paths), kinds),
            encoding=self.cfg.encoding,
        )

    def files(self, paths: Iterable[Path], output: Path) -> None:
        """@brief Convert several source files into one Markdown file."""
        docs = []
        for path in paths:
            md = self.file(path)
            if md.strip():
                docs.append(f"# {path.name}\n\n{md}")
        output.write_text("\n\n".join(docs) + "\n", encoding=self.cfg.encoding)


# ============================================================
# SCAFFOLDER
# ============================================================


class Scaffolder:
    """@brief Create a starter `ini.cfg` in a target directory."""

    def __init__(self, target: Path, force: bool = False) -> None:
        self.target = target
        self.force = force

    def run(self) -> list[Path]:
        """@brief Write the default configuration file."""
        self.target.mkdir(parents=True, exist_ok=True)
        path = self.target / CONFIG_FILENAME
        if path.exists() and not self.force:
            raise SystemExit(f"Refusing to overwrite {path} (use --force).")
        path.write_text(DEFAULT_CONFIG_TEXT, encoding="utf-8")
        return [path]


# ============================================================
# CLI
# ============================================================


def build_parser() -> argparse.ArgumentParser:
    """@brief Build the command-line argument parser."""
    parser = argparse.ArgumentParser(prog="doxygen2md")
    parser.add_argument(
        "--version", action="version", version=f"%(prog)s {__version__}"
    )
    parser.add_argument("-c", "--config", type=Path, default=None)
    sub = parser.add_subparsers(dest="command")
    convert = sub.add_parser("convert")
    convert.add_argument("input", type=Path, nargs="?", default=None)
    convert.add_argument("-o", "--output", type=Path, default=None)
    convert.add_argument("-c", "--config", type=Path, default=argparse.SUPPRESS)
    diagram = sub.add_parser("diagram")
    diagram.add_argument("input", type=Path, nargs="?", default=None)
    diagram.add_argument("-o", "--output", type=Path, default=None)
    diagram.add_argument(
        "-k", "--kind", choices=(*MermaidRenderer.KINDS, "all"), default="class"
    )
    diagram.add_argument("-c", "--config", type=Path, default=argparse.SUPPRESS)
    init = sub.add_parser("init")
    init.add_argument("target", type=Path, nargs="?", default=Path("."))
    init.add_argument("--force", action="store_true")
    return parser


def _candidate_bases(base_dir: Path | None) -> list[Path]:
    """@brief Build ordered directories for resolving relative glob patterns."""
    candidates: list[Path] = []
    if base_dir is not None:
        resolved = base_dir.resolve()
        candidates.extend((resolved, resolved.parent))
    candidates.append(Path.cwd())
    unique: list[Path] = []
    for candidate in candidates:
        if candidate not in unique:
            unique.append(candidate)
    return unique


def collect_files(root: Path | str, base_dir: Path | None = None) -> list[Path]:
    """@brief Expand a file, directory, or glob pattern into source files."""
    pattern = str(root)
    if Path(pattern).is_absolute():
        search_bases: list[Path | None] = [None]
    else:
        search_bases = list(_candidate_bases(base_dir))

    candidates: list[Path] = []
    for base in search_bases:
        expanded = pattern if base is None else str(base / pattern)
        matches = [Path(p) for p in globlib.glob(expanded, recursive=True)]
        if matches:
            candidates = matches
            break

    files: set[Path] = set()
    for candidate in candidates:
        if candidate.is_file():
            if candidate.suffix.lower() in LANGUAGES:
                files.add(candidate)
        elif candidate.is_dir():
            files.update(
                path
                for path in candidate.rglob("*")
                if path.is_file() and path.suffix.lower() in LANGUAGES
            )
    return sorted(files)


def _resolve_output(value: Path | None, default: Path, base_dir: Path) -> Path:
    """@brief Choose an output path and resolve it against the config directory."""
    chosen = default if value is None else value
    return chosen if chosen.is_absolute() else base_dir / chosen


def _select_files(args: argparse.Namespace, cfg: Config) -> list[Path]:
    """@brief Resolve the input or config glob into source files."""
    spec = args.input if args.input is not None else cfg.glob
    files = collect_files(spec, base_dir=cfg.base_dir)
    if not files:
        raise SystemExit(f"No supported source files found for {spec!r}")
    return files


def cmd_convert(args: argparse.Namespace) -> None:
    """@brief Execute the `convert` subcommand."""
    cfg = Config.load(getattr(args, "config", None))
    files = _select_files(args, cfg)
    output = _resolve_output(args.output, cfg.output, cfg.base_dir)
    output.parent.mkdir(parents=True, exist_ok=True)
    Converter(cfg).files(files, output)
    print(f"Generated: {output}")


def cmd_diagram(args: argparse.Namespace) -> None:
    """@brief Execute the `diagram` subcommand.

    @details `--kind all` writes every supported diagram into one Markdown
    file as separate ` ```mermaid ` code blocks. A single kind writes just
    that diagram, also wrapped in a fenced block. The output path comes from
    `-o`, or from the config `diagram` key.

    @param[in] args Parsed arguments containing `input`, `config`, `output`,
    and `kind`.
    @return Nothing; the Markdown file is written as a side effect.
    @throws SystemExit When no supported source files are found.
    """
    cfg = Config.load(getattr(args, "config", None))
    files = _select_files(args, cfg)
    output = _resolve_output(args.output, cfg.diagram, cfg.base_dir)
    output.parent.mkdir(parents=True, exist_ok=True)
    Converter(cfg).diagram(files, output, args.kind)
    print(f"Generated: {output}")


def cmd_init(args: argparse.Namespace) -> None:
    """@brief Execute the `init` subcommand."""
    for path in Scaffolder(args.target, force=args.force).run():
        print(f"Wrote: {path}")


def main() -> None:
    """@brief Run the command-line interface."""
    parser = build_parser()
    argv_list = sys.argv[1:]
    if not argv_list:
        parser.print_help()
        return
    subcommands = {"convert", "diagram", "init"}
    if not subcommands.intersection(argv_list):
        if {"-h", "--help", "--version"}.intersection(argv_list):
            parser.parse_args(argv_list)  # prints help/version and exits
        argv_list = ["convert", *argv_list]
    argv = parser.parse_args(argv_list)
    handlers = {
        "init": cmd_init,
        "diagram": cmd_diagram,
        "convert": cmd_convert,
    }
    handlers[argv.command](argv)

if __name__ == "__main__":
    main()

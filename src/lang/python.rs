//! Python source parsing through the Ruff AST.

use std::collections::HashMap;

use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
use ruff_python_ast::{AnyNodeRef, Expr, Operator, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_codegen::{Generator, Indentation, Mode};
use ruff_python_parser::parse_module;
use ruff_source_file::LineEnding;

use crate::doc::{DOC_PARSER, cleandoc};
use crate::lang::Language;
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};

const ABSTRACT_DECOS: &[&str] = &["abstractmethod", "abstractproperty"];
const ENUM_BASES: &[&str] = &["Enum", "IntEnum", "StrEnum", "Flag", "IntFlag"];
const ABSTRACT_BASES: &[&str] = &["ABC", "ABCMeta"];

/// Parse Python modules, classes, and functions through the Ruff AST.
pub struct PythonLanguage;

impl Language for PythonLanguage {
    fn extensions(&self) -> &'static [&'static str] {
        &[".py"]
    }

    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let parsed = match parse_module(source) {
            Ok(parsed) => parsed,
            Err(_) => return Vec::new(),
        };
        if parsed.has_invalid_syntax() {
            return Vec::new();
        }

        let module = parsed.syntax();
        let mut out = Vec::new();

        let mut module_entity = Entity::new(EntityKind::Module, filename);
        module_entity.line = 1;
        module_entity.doc = DOC_PARSER.parse(&docstring(&module.body).unwrap_or_default());
        module_entity.includes = imports(&module.body);
        out.push(module_entity);

        for stmt in &module.body {
            match stmt {
                Stmt::FunctionDef(_) => {
                    let mut entity = function(stmt, EntityKind::Function);
                    entity.line = line_of(source, stmt);
                    out.push(entity);
                }
                Stmt::ClassDef(class) => {
                    let kind = if is_enum(class) {
                        EntityKind::Enum
                    } else {
                        EntityKind::Class
                    };
                    let mut entity = class_entity(class, kind, source);
                    entity.line = line_of(source, stmt);
                    out.push(entity);
                }
                _ => {}
            }
        }

        out
    }
}

fn line_of(source: &str, stmt: &Stmt) -> usize {
    let offset = match stmt {
        Stmt::FunctionDef(item) => item.range.start().to_usize(),
        Stmt::ClassDef(item) => item.range.start().to_usize(),
        _ => 0,
    };
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn docstring(body: &[Stmt]) -> Option<String> {
    let Stmt::Expr(item) = body.first()? else {
        return None;
    };
    let Expr::StringLiteral(literal) = item.value.as_ref() else {
        return None;
    };
    Some(cleandoc(literal.value.to_str()))
}

fn unparse(expr: &Expr) -> String {
    match expr {
        Expr::Subscript(subscript) => format!(
            "{}[{}]",
            unparse(&subscript.value),
            unparse_slice(subscript.slice.as_ref())
        ),
        Expr::Tuple(tuple) => {
            let parts: Vec<String> = tuple.elts.iter().map(unparse).collect();
            match parts.len() {
                0 => "()".to_owned(),
                1 => format!("({},)", parts[0]),
                _ => format!("({})", parts.join(", ")),
            }
        }
        Expr::List(list) => format!(
            "[{}]",
            list.elts.iter().map(unparse).collect::<Vec<_>>().join(", ")
        ),
        Expr::BinOp(bin) if matches!(bin.op, Operator::BitOr) => {
            format!("{} | {}", unparse(&bin.left), unparse(&bin.right))
        }
        _ => Generator::new(&Indentation::default(), LineEnding::Lf)
            .with_mode(Mode::AstUnparse)
            .expr(expr),
    }
}

/// Render a subscript slice the way CPython's `ast.unparse` does: tuple
/// slices lose their parentheses, while a lone `Slice` keeps its `:` form.
fn unparse_slice(slice: &Expr) -> String {
    match slice {
        Expr::Tuple(tuple) if !tuple.elts.is_empty() => {
            let parts: Vec<String> = tuple.elts.iter().map(unparse_slice_element).collect();
            if parts.len() == 1 {
                format!("{},", parts[0])
            } else {
                parts.join(", ")
            }
        }
        other => unparse(other),
    }
}

fn unparse_slice_element(expr: &Expr) -> String {
    match expr {
        Expr::Slice(slice) => {
            let mut out = String::new();
            if let Some(lower) = &slice.lower {
                out.push_str(&unparse(lower));
            }
            out.push(':');
            if let Some(upper) = &slice.upper {
                out.push_str(&unparse(upper));
            }
            if let Some(step) = &slice.step {
                out.push(':');
                out.push_str(&unparse(step));
            }
            out
        }
        other => unparse(other),
    }
}

fn function(stmt: &Stmt, kind: EntityKind) -> Entity {
    let Stmt::FunctionDef(node) = stmt else {
        unreachable!("function() expects a function statement")
    };

    let doc_text = docstring(&node.body).unwrap_or_default();
    let mut doc = DOC_PARSER.parse(&doc_text);

    let annotations: HashMap<String, String> = node
        .parameters
        .args
        .iter()
        .filter_map(|arg| {
            arg.parameter
                .annotation
                .as_ref()
                .map(|annotation| (arg.parameter.name.to_string(), unparse(annotation)))
        })
        .collect();

    for param in &mut doc.params {
        if param.r#type.is_empty()
            && let Some(annotation) = annotations.get(&param.name)
        {
            param.r#type = annotation.clone();
        }
    }

    let mut args = Vec::new();
    for arg in &node.parameters.args {
        let name = arg.parameter.name.as_str();
        if name == "self" || name == "cls" {
            continue;
        }
        let mut rendered = name.to_owned();
        if let Some(annotation) = &arg.parameter.annotation {
            rendered.push_str(": ");
            rendered.push_str(&unparse(annotation));
        }
        args.push(rendered);
    }

    let return_type = node.returns.as_deref().map(unparse).unwrap_or_default();

    let prefix = if node.is_async { "async def" } else { "def" };
    let mut signature = format!("{prefix} {}({})", node.name, args.join(", "));
    if !return_type.is_empty() {
        signature.push_str(" -> ");
        signature.push_str(&return_type);
    }

    let mut entity = Entity::new(kind, node.name.to_string());
    entity.signature = signature;
    entity.return_type = return_type;
    entity.doc = doc;
    entity.calls = calls(stmt);
    entity
}

fn class_entity(node: &StmtClassDef, kind: EntityKind, source: &str) -> Entity {
    let doc_text = docstring(&node.body).unwrap_or_default();
    let doc = DOC_PARSER.parse(&doc_text);
    let abstract_ = is_abstract(node);

    let mut entity = Entity::new(kind, node.name.to_string());
    entity.doc = doc;
    entity.r#abstract = abstract_;

    if kind == EntityKind::Enum {
        entity.variants = enum_variants(node);
        return entity;
    }

    entity.bases = base_names(node);
    let init = node.body.iter().find(|item| match item {
        Stmt::FunctionDef(function) => !function.is_async && function.name.as_str() == "__init__",
        _ => false,
    });
    entity.fields = fields(node, init);

    for item in &node.body {
        let Stmt::FunctionDef(function) = item else {
            continue;
        };
        let name = function.name.as_str();
        let is_public = !name.starts_with('_') || name == "__init__" || name == "__del__";
        if !is_public {
            continue;
        }
        let mut method = function_instance(item);
        method.kind = EntityKind::Method;
        method.r#abstract = is_abstract_method(function);
        method.line = line_of(source, item);
        entity.members.push(method);
    }

    entity
}

fn function_instance(stmt: &Stmt) -> Entity {
    function(stmt, EntityKind::Method)
}

fn enum_variants(node: &StmtClassDef) -> Vec<Variant> {
    let mut variants = Vec::new();
    for item in &node.body {
        let Stmt::Assign(assign) = item else {
            continue;
        };
        for target in &assign.targets {
            if let Expr::Name(name) = target
                && !name.id.as_str().starts_with('_')
            {
                variants.push(Variant {
                    name: name.id.to_string(),
                    description: String::new(),
                });
            }
        }
    }
    variants
}

fn base_names(node: &StmtClassDef) -> Vec<String> {
    let Some(arguments) = &node.arguments else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for base in &arguments.args {
        let name = node_name(base);
        if !name.is_empty()
            && !ABSTRACT_BASES.contains(&name.as_str())
            && !ENUM_BASES.contains(&name.as_str())
            && name != "object"
        {
            dedup_push(&mut names, name);
        }
    }
    names
}

fn fields(node: &StmtClassDef, init: Option<&Stmt>) -> Vec<Field> {
    let mut found: Vec<Field> = Vec::new();
    let mut index_by_name: HashMap<String, usize> = HashMap::new();

    let mut hints: HashMap<String, String> = HashMap::new();
    if let Some(Stmt::FunctionDef(function)) = init {
        for arg in &function.parameters.args {
            if let Some(annotation) = &arg.parameter.annotation {
                hints.insert(arg.parameter.name.to_string(), unparse(annotation));
            }
        }
    }

    for item in &node.body {
        if let Stmt::AnnAssign(assign) = item
            && let Expr::Name(target) = assign.target.as_ref()
        {
            add_field(
                &mut found,
                &mut index_by_name,
                target.id.as_str(),
                &unparse(&assign.annotation),
                "public",
            );
        }
    }

    if let Some(init) = init {
        for level in collect_bfs(std::slice::from_ref(init), false) {
            for node in level {
                match node {
                    AnyNodeRef::StmtAnnAssign(assign) => {
                        if let Some(name) = self_attribute(assign.target.as_ref()) {
                            add_field(
                                &mut found,
                                &mut index_by_name,
                                &name,
                                &unparse(&assign.annotation),
                                &visibility(&name),
                            );
                        }
                    }
                    AnyNodeRef::StmtAssign(assign) => {
                        for target in &assign.targets {
                            let Some(name) = self_attribute(target) else {
                                continue;
                            };
                            let inferred = match assign.value.as_ref() {
                                Expr::Name(value) => {
                                    hints.get(value.id.as_str()).cloned().unwrap_or_default()
                                }
                                _ => String::new(),
                            };
                            add_field(
                                &mut found,
                                &mut index_by_name,
                                &name,
                                &inferred,
                                &visibility(&name),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let descriptions: HashMap<String, String> = init
        .and_then(|init| match init {
            Stmt::FunctionDef(function) => docstring(&function.body),
            _ => None,
        })
        .map(|text| DOC_PARSER.parse(&text).params)
        .unwrap_or_default()
        .into_iter()
        .map(|param| (param.name, param.description))
        .collect();

    for field in &mut found {
        field.description = descriptions.get(&field.name).cloned().unwrap_or_default();
    }

    found
}

fn add_field(
    found: &mut Vec<Field>,
    index_by_name: &mut HashMap<String, usize>,
    name: &str,
    type_: &str,
    visibility: &str,
) {
    if name.starts_with("__") && name.ends_with("__") {
        return;
    }
    if let Some(&index) = index_by_name.get(name) {
        if !type_.is_empty() && found[index].r#type.is_empty() {
            found[index].r#type = type_.to_owned();
        }
        return;
    }
    let mut field = Field::new(name);
    field.r#type = type_.to_owned();
    field.visibility = visibility.to_owned();
    index_by_name.insert(name.to_owned(), found.len());
    found.push(field);
}

fn self_attribute(expr: &Expr) -> Option<String> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    let Expr::Name(value) = attribute.value.as_ref() else {
        return None;
    };
    if value.id.as_str() == "self" {
        Some(attribute.attr.to_string())
    } else {
        None
    }
}

fn visibility(name: &str) -> String {
    if name.starts_with("__") {
        "private".to_owned()
    } else if name.starts_with('_') {
        "protected".to_owned()
    } else {
        "public".to_owned()
    }
}

fn node_name(expr: &Expr) -> String {
    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Attribute(attribute) => attribute.attr.to_string(),
        Expr::Subscript(subscript) => node_name(&subscript.value),
        _ => String::new(),
    }
}

fn imports(body: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for level in collect_bfs(body, false) {
        for node in level {
            match node {
                AnyNodeRef::StmtImport(import) => {
                    for alias in &import.names {
                        dedup_push(&mut out, alias.name.to_string());
                    }
                }
                AnyNodeRef::StmtImportFrom(import) => {
                    let dots = ".".repeat(import.level as usize);
                    let module = import
                        .module
                        .as_ref()
                        .map(|module| module.to_string())
                        .unwrap_or_default();
                    dedup_push(&mut out, format!("{dots}{module}"));
                }
                _ => {}
            }
        }
    }
    out
}

fn calls(stmt: &Stmt) -> Vec<String> {
    let mut out = Vec::new();
    for level in collect_bfs(std::slice::from_ref(stmt), true) {
        for node in level {
            if let AnyNodeRef::ExprCall(call) = node {
                let name = call_name(&call.func);
                if !name.is_empty() {
                    dedup_push(&mut out, name);
                }
            }
        }
    }
    out
}

fn call_name(expr: &Expr) -> String {
    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Attribute(attribute) => {
            let owner = match attribute.value.as_ref() {
                Expr::Name(value) => value.id.to_string(),
                Expr::Call(call) => call_name(&call.func),
                _ => String::new(),
            };
            if owner.is_empty() {
                attribute.attr.to_string()
            } else {
                format!("{}.{}", owner, attribute.attr)
            }
        }
        _ => String::new(),
    }
}

fn is_enum(node: &StmtClassDef) -> bool {
    let Some(arguments) = &node.arguments else {
        return false;
    };
    arguments.args.iter().any(|base| match base {
        Expr::Name(name) => ENUM_BASES.contains(&name.id.as_str()),
        Expr::Attribute(attribute) => ENUM_BASES.contains(&attribute.attr.as_str()),
        _ => false,
    })
}

fn is_abstract(node: &StmtClassDef) -> bool {
    if let Some(arguments) = &node.arguments {
        for base in &arguments.args {
            let name = match base {
                Expr::Name(name) => name.id.as_str(),
                Expr::Attribute(attribute) => attribute.attr.as_str(),
                _ => "",
            };
            if ABSTRACT_BASES.contains(&name) {
                return true;
            }
        }
    }

    node.body.iter().any(|item| match item {
        Stmt::FunctionDef(function) => !function.is_async && is_abstract_method(function),
        _ => false,
    })
}

fn is_abstract_method(node: &StmtFunctionDef) -> bool {
    node.decorator_list.iter().any(|decorator| {
        let name = match &decorator.expression {
            Expr::Name(name) => name.id.as_str(),
            Expr::Attribute(attribute) => attribute.attr.as_str(),
            _ => "",
        };
        ABSTRACT_DECOS.contains(&name)
    })
}

/// Collect AST nodes into breadth-first levels, optionally pruning nested
/// function/class/lambda bodies.
fn collect_bfs<'a>(stmts: &'a [Stmt], skip_defs: bool) -> Vec<Vec<AnyNodeRef<'a>>> {
    let mut visitor = BfsVisitor {
        depth: 0,
        levels: Vec::new(),
        skip_defs,
    };
    for stmt in stmts {
        visitor.visit_stmt(stmt);
    }
    visitor.levels
}

struct BfsVisitor<'a> {
    depth: usize,
    levels: Vec<Vec<AnyNodeRef<'a>>>,
    skip_defs: bool,
}

impl<'a> SourceOrderVisitor<'a> for BfsVisitor<'a> {
    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let skip = self.skip_defs
            && self.depth > 0
            && matches!(
                node,
                AnyNodeRef::StmtFunctionDef(_)
                    | AnyNodeRef::StmtClassDef(_)
                    | AnyNodeRef::ExprLambda(_)
            );

        while self.levels.len() <= self.depth {
            self.levels.push(Vec::new());
        }
        self.levels[self.depth].push(node);
        self.depth += 1;

        if skip {
            TraversalSignal::Skip
        } else {
            TraversalSignal::Traverse
        }
    }

    fn leave_node(&mut self, _node: AnyNodeRef<'a>) {
        self.depth = self.depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_animal_fixture() {
        let source = std::fs::read_to_string("tests/fixtures/animal.py").unwrap();
        let entities = PythonLanguage.parse(&source, "animal.py");
        assert_eq!(entities[0].kind, EntityKind::Module);
        let animal = entities.iter().find(|e| e.name == "Animal").unwrap();
        assert_eq!(animal.kind, EntityKind::Class);
        assert!(animal.r#abstract);
        assert!(animal.fields.is_empty());
        assert!(animal.members.iter().any(|m| m.name == "make_sound"));
    }

    #[test]
    fn unparses_annotations_like_ast_unparse() {
        let source =
            "x: list[\"Entity\"]\ny: Path | None\nz: dict[str, tuple[int, str]]\nw: X[a:b, c]\n";
        let parsed = parse_module(source).unwrap();
        let annotations: Vec<String> = parsed
            .syntax()
            .body
            .iter()
            .map(|stmt| {
                let Stmt::AnnAssign(assign) = stmt else {
                    panic!("expected annotation");
                };
                unparse(&assign.annotation)
            })
            .collect();
        assert_eq!(
            annotations,
            vec![
                "list['Entity']",
                "Path | None",
                "dict[str, tuple[int, str]]",
                "X[a:b, c]",
            ]
        );
    }
}

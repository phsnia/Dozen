//! @brief Python source parsing through Tree-sitter.
//! @details Lowers Python syntax nodes into the shared Rust entity model.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::ast::Visibility;
use crate::documentation::{DOC_PARSER, cleandoc};
use crate::lang::Language;
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};
use crate::parser::tree_sitter::parse_python;

const ABSTRACT_DECOS: &[&str] = &["abstractmethod", "abstractproperty"];
const ENUM_BASES: &[&str] = &["Enum", "IntEnum", "StrEnum", "Flag", "IntFlag"];
const ABSTRACT_BASES: &[&str] = &["ABC", "ABCMeta"];

/// @brief Rust-native expression subset used for Python annotations and call names.
/// @details This small AST avoids exposing Tree-sitter grammar nodes to renderers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PyExpr {
    Raw(String),
    Name(String),
    Attribute(Box<PyExpr>, String),
    Subscript(Box<PyExpr>, Box<PyExpr>),
    Tuple(Vec<PyExpr>),
    List(Vec<PyExpr>),
    BinOp(Box<PyExpr>, String, Box<PyExpr>),
    String(String),
    Call(Box<PyExpr>, Vec<PyExpr>),
}

/// @brief Parse Python modules, classes, and functions through Tree-sitter.
pub struct PythonLanguage;

impl Language for PythonLanguage {
    /// @brief Return Python source extensions.
    /// @return The `.py` extension.
    fn extensions(&self) -> &'static [&'static str] {
        &[".py"]
    }

    /// @brief Parse one Python source file.
    /// @param[in] source Python source text.
    /// @param[in] filename Source filename used as the module name.
    /// @return Contract entities, or an empty list for invalid syntax.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let Some(tree) = parse_python(source) else {
            return Vec::new();
        };

        let root = tree.root_node();
        let mut out = Vec::new();

        let mut module = Entity::new(EntityKind::Module, filename);
        module.line = 1;
        module.doc = DOC_PARSER.parse(&module_docstring(source, &root).unwrap_or_default());
        module.includes = collect_imports(source, &root);
        out.push(module);

        let mut cursor = root.walk();
        for child in root.named_children(&mut cursor) {
            let declaration = unwrap_decorated(&child);
            if declaration.kind() == "function_definition" {
                out.push(function_entity(source, &declaration, EntityKind::Function));
            } else if declaration.kind() == "class_definition" {
                let kind = if is_enum_class(source, &declaration) {
                    EntityKind::Enum
                } else {
                    EntityKind::Class
                };
                out.push(class_entity(source, &declaration, kind));
            }
        }
        out
    }
}

/// @brief Unwrap one decorated class or function definition.
/// @param[in] node Tree-sitter decorated-definition node.
/// @return The underlying function or class node.
fn unwrap_decorated<'a>(node: &Node<'a>) -> Node<'a> {
    if node.kind() == "decorated_definition" {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if matches!(child.kind(), "function_definition" | "class_definition") {
                return child;
            }
        }
    }
    *node
}

/// @brief Read the exact source text covered by a syntax node.
/// @param[in] source Complete source text.
/// @param[in] node Tree-sitter syntax node.
/// @return Source text for the node.
fn text_at(source: &str, node: &Node<'_>) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

/// @brief Extract and clean a module docstring.
/// @param[in] source Complete source text.
/// @param[in] root Python module syntax root.
/// @return Cleaned module docstring, if present.
fn module_docstring(source: &str, root: &Node<'_>) -> Option<String> {
    let mut cursor = root.walk();
    let first = root.named_children(&mut cursor).next()?;
    let expression = if first.kind() == "expression_statement" {
        first
    } else {
        return None;
    };
    let string = expression.named_child(0)?;
    if matches!(string.kind(), "string" | "concatenated_string") {
        Some(cleandoc(&clean_string(&text_at(source, &string))))
    } else {
        None
    }
}

/// @brief Extract and clean the leading docstring of a declaration body.
/// @param[in] source Complete source text.
/// @param[in] body Declaration body node.
/// @return Cleaned docstring, if present.
fn docstring(source: &str, body: &Node<'_>) -> Option<String> {
    let mut cursor = body.walk();
    let child = body.named_children(&mut cursor).next()?;
    if child.kind() == "expression_statement"
        && let Some(string) = child.named_child(0)
        && matches!(string.kind(), "string" | "concatenated_string")
    {
        return Some(cleandoc(&clean_string(&text_at(source, &string))));
    }
    None
}

/// @brief Remove Python string prefixes and quote delimiters.
/// @param[in] raw Source spelling of a string literal.
/// @return String content suitable for `inspect.cleandoc`-style cleanup.
fn clean_string(raw: &str) -> String {
    let text = raw.trim();
    let text = text
        .strip_prefix("r")
        .or_else(|| text.strip_prefix("b"))
        .or_else(|| text.strip_prefix("u"))
        .or_else(|| text.strip_prefix("f"))
        .unwrap_or(text);
    if text.len() >= 6
        && ((text.starts_with("\"\"\"") && text.ends_with("\"\"\""))
            || (text.starts_with("'''") && text.ends_with("'''")))
    {
        text[3..text.len() - 3].to_owned()
    } else if text.len() >= 2 {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

/// @brief Convert a Tree-sitter row to a one-based source line.
/// @param[in] node Tree-sitter syntax node.
/// @return One-based source line.
fn line_of(node: &Node<'_>) -> usize {
    node.start_position().row + 1
}

/// @brief Lower a Python function or method into an entity.
/// @param[in] source Complete source text.
/// @param[in] node Function-definition syntax node.
/// @param[in] kind Function or method entity kind.
/// @return The lowered contract entity.
fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity {
    let name = node
        .child_by_field_name("name")
        .map(|name| text_at(source, &name))
        .unwrap_or_default();
    let body = node.child_by_field_name("body");
    let doc_text = body
        .as_ref()
        .and_then(|body| docstring(source, body))
        .unwrap_or_default();
    let mut doc = DOC_PARSER.parse(&doc_text);

    let parameters = node.child_by_field_name("parameters");
    let annotations = parameter_annotations(source, parameters.as_ref());
    for param in &mut doc.params {
        if param.r#type.is_empty()
            && let Some(annotation) = annotations.get(&param.name)
        {
            param.r#type = annotation.clone();
        }
    }

    let mut args = Vec::new();
    if let Some(parameters) = parameters {
        let mut cursor = parameters.walk();
        for parameter in parameters.named_children(&mut cursor) {
            let text = text_at(source, &parameter);
            if matches!(text.as_str(), "*" | "/" | "**") {
                continue;
            }
            let name = parameter_name(source, &parameter);
            if name.is_empty() || name == "self" || name == "cls" {
                continue;
            }
            let mut rendered = name.clone();
            if let Some(annotation) = parameter_annotation(source, &parameter) {
                rendered.push_str(": ");
                rendered.push_str(&annotation);
            } else if let Some(annotation) = annotations.get(&name) {
                rendered.push_str(": ");
                rendered.push_str(annotation);
            }
            args.push(rendered);
        }
    }

    let return_type = node
        .child_by_field_name("return_type")
        .map(|return_type| unparse_text(source, &return_type))
        .unwrap_or_default();
    let is_async = text_at(source, node).starts_with("async ");
    let prefix = if is_async { "async def" } else { "def" };
    let mut signature = format!("{prefix} {name}({})", args.join(", "));
    if !return_type.is_empty() {
        signature.push_str(" -> ");
        signature.push_str(&return_type);
    }

    let mut entity = Entity::new(kind, name);
    entity.signature = signature;
    entity.return_type = return_type;
    entity.doc = doc;
    entity.line = line_of(node);
    entity.calls = collect_calls(source, node);
    entity
}

fn parameter_annotations(source: &str, parameters: Option<&Node<'_>>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(parameters) = parameters else {
        return out;
    };
    let mut cursor = parameters.walk();
    for parameter in parameters.named_children(&mut cursor) {
        let name = parameter_name(source, &parameter);
        if let Some(annotation) = parameter_annotation(source, &parameter) {
            out.insert(name, annotation);
        }
    }
    out
}

fn parameter_name(source: &str, parameter: &Node<'_>) -> String {
    if let Some(name) = parameter.child_by_field_name("name") {
        return text_at(source, &name);
    }
    let text = text_at(source, parameter);
    text.split(':')
        .next()
        .unwrap_or("")
        .split('=')
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('*')
        .trim()
        .to_owned()
}

fn parameter_annotation(source: &str, parameter: &Node<'_>) -> Option<String> {
    if let Some(annotation) = parameter.child_by_field_name("type") {
        return Some(unparse_text(source, &annotation));
    }
    let text = text_at(source, parameter);
    let after_colon = text.split_once(':')?.1;
    let annotation = after_colon.split('=').next().unwrap_or("").trim();
    (!annotation.is_empty()).then(|| unparse_annotation_text(annotation))
}

fn unparse_annotation_text(text: &str) -> String {
    let text = text.trim();
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        format!("'{}'", &text[1..text.len() - 1])
    } else {
        text.to_owned()
    }
}

fn class_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity {
    let name = node
        .child_by_field_name("name")
        .map(|name| text_at(source, &name))
        .unwrap_or_default();
    let body = node.child_by_field_name("body");
    let doc_text = body
        .as_ref()
        .and_then(|body| docstring(source, body))
        .unwrap_or_default();
    let mut entity = Entity::new(kind, name);
    entity.doc = DOC_PARSER.parse(&doc_text);
    entity.line = line_of(node);
    entity.r#abstract = is_abstract_class(source, node);

    if kind == EntityKind::Enum {
        entity.variants = enum_variants(source, node);
        return entity;
    }

    entity.bases = class_bases(source, node);
    let init = find_method(source, node, "__init__");
    entity.fields = collect_fields(source, node, init.as_ref());

    let Some(body) = body else {
        return entity;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        let mut definition = child;
        if child.kind() == "decorated_definition" {
            definition = unwrap_decorated(&child);
        }
        if definition.kind() != "function_definition" {
            continue;
        }
        let name = definition
            .child_by_field_name("name")
            .map(|name| text_at(source, &name))
            .unwrap_or_default();
        let is_public = !name.starts_with('_') || name == "__init__" || name == "__del__";
        if !is_public {
            continue;
        }
        let mut method = function_entity(source, &definition, EntityKind::Method);
        method.r#abstract = is_abstract_method(source, &child);
        entity.members.push(method);
    }
    entity
}

fn find_method<'a>(source: &str, class: &Node<'a>, wanted: &str) -> Option<Node<'a>> {
    let body = class.child_by_field_name("body")?;
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        let definition = unwrap_decorated(&child);
        if definition.kind() != "function_definition" {
            continue;
        }
        if let Some(name) = definition.child_by_field_name("name")
            && text_at(source, &name) == wanted
        {
            return Some(definition);
        }
    }
    None
}

fn is_enum_class(source: &str, node: &Node<'_>) -> bool {
    let bases = class_bases(source, node);
    if bases.iter().any(|base| ENUM_BASES.contains(&base.as_str())) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        child.kind() == "argument_list"
            && ENUM_BASES
                .iter()
                .any(|base| text_at(source, &child).contains(base))
    })
}

fn is_abstract_class(source: &str, node: &Node<'_>) -> bool {
    if class_bases(source, node)
        .iter()
        .any(|base| ABSTRACT_BASES.contains(&base.as_str()))
    {
        return true;
    }
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    let mut cursor = body.walk();
    body.named_children(&mut cursor).any(|child| {
        let definition = unwrap_decorated(&child);
        definition.kind() == "function_definition" && is_abstract_method(source, &child)
    })
}

fn is_abstract_method(source: &str, original: &Node<'_>) -> bool {
    if original.kind() != "decorated_definition" {
        return false;
    }
    let mut cursor = original.walk();
    original.named_children(&mut cursor).any(|child| {
        if child.kind() == "decorator" {
            let mut inner = child.walk();
            if let Some(expression) = child.named_children(&mut inner).next() {
                let name = node_name(source, &expression);
                return ABSTRACT_DECOS.contains(&name.as_str());
            }
        }
        false
    })
}

fn class_bases(source: &str, node: &Node<'_>) -> Vec<String> {
    let superclasses = node.child_by_field_name("superclasses").or_else(|| {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find(|child| child.kind() == "argument_list")
    });
    let Some(superclasses) = superclasses else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut cursor = superclasses.walk();
    for child in superclasses.named_children(&mut cursor) {
        let name = node_name(source, &child);
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

fn enum_variants(source: &str, node: &Node<'_>) -> Vec<Variant> {
    let mut variants = Vec::new();
    let Some(body) = node.child_by_field_name("body") else {
        return variants;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        let mut assignment = child;
        if child.kind() == "expression_statement"
            && let Some(named) = child.named_child(0)
            && named.kind() == "assignment"
        {
            assignment = named;
        }
        if assignment.kind() == "assignment"
            && let Some(left) = assignment.child_by_field_name("left")
        {
            let name = text_at(source, &left);
            if !name.is_empty() && !name.starts_with('_') {
                variants.push(Variant {
                    name,
                    description: String::new(),
                });
            }
        }
    }
    variants
}

fn collect_fields(source: &str, class: &Node<'_>, init: Option<&Node<'_>>) -> Vec<Field> {
    let mut found = Vec::new();
    let mut index_by_name: HashMap<String, usize> = HashMap::new();
    let mut hints = HashMap::new();

    if let Some(init) = init
        && let Some(parameters) = init.child_by_field_name("parameters")
    {
        let mut cursor = parameters.walk();
        for parameter in parameters.named_children(&mut cursor) {
            let name = parameter_name(source, &parameter);
            if let Some(annotation) = parameter_annotation(source, &parameter) {
                hints.insert(name, annotation);
            }
        }
    }

    if let Some(body) = class.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            let mut candidate = child;
            if child.kind() == "expression_statement"
                && let Some(named) = child.named_child(0)
                && named.kind() == "assignment"
            {
                candidate = named;
            }
            if candidate.kind() == "assignment"
                && let Some(left) = candidate.child_by_field_name("left")
            {
                let name = text_at(source, &left);
                if let Some(type_node) = candidate.child_by_field_name("type") {
                    let annotation = unparse_text(source, &type_node);
                    add_field(&mut found, &mut index_by_name, &name, &annotation, "public");
                } else if !name.is_empty() && !name.contains('.') {
                    add_field(&mut found, &mut index_by_name, &name, "", "public");
                }
            }
            if child.kind() == "type_for_in_statement" || child.kind().starts_with("type") {
                let text = text_at(source, &child);
                if let Some((name, annotation)) = text.split_once(':') {
                    add_field(
                        &mut found,
                        &mut index_by_name,
                        name.trim(),
                        &unparse_annotation_text(annotation.trim()),
                        "public",
                    );
                }
            }
        }
    }

    if let Some(init) = init
        && let Some(body) = init.child_by_field_name("body")
    {
        walk_fields(source, &body, &hints, &mut found, &mut index_by_name);
    }

    if let Some(doc) = init
        .and_then(|init| init.child_by_field_name("body"))
        .and_then(|body| docstring(source, &body))
    {
        let descriptions: HashMap<String, String> = DOC_PARSER
            .parse(&doc)
            .params
            .into_iter()
            .map(|param| (param.name, param.description))
            .collect();
        for field in &mut found {
            field.description = descriptions.get(&field.name).cloned().unwrap_or_default();
        }
    }
    found
}

fn walk_fields(
    source: &str,
    node: &Node<'_>,
    hints: &HashMap<String, String>,
    found: &mut Vec<Field>,
    index_by_name: &mut HashMap<String, usize>,
) {
    match node.kind() {
        "function_definition" | "class_definition" | "lambda" => return,
        "typed_assignment" | "assignment" | "expression_statement" => {}
        _ => {}
    }

    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left")
            && let Some(name) = self_attribute(source, &left)
        {
            let mut inferred = String::new();
            if let Some(right) = node.child_by_field_name("right")
                && right.kind() == "identifier"
            {
                inferred = hints
                    .get(&text_at(source, &right))
                    .cloned()
                    .unwrap_or_default();
            }
            add_field(found, index_by_name, &name, &inferred, visibility(&name));
        }
    } else if node.kind() == "typed_assignment"
        && let Some(left) = node.child_by_field_name("left")
        && let Some(name) = self_attribute(source, &left)
    {
        let annotation = node
            .child_by_field_name("type")
            .map(|type_| unparse_text(source, &type_));
        add_field(
            found,
            index_by_name,
            &name,
            annotation.as_deref().unwrap_or_default(),
            visibility(&name),
        );
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_fields(source, &child, hints, found, index_by_name);
    }
}

fn self_attribute(source: &str, node: &Node<'_>) -> Option<String> {
    if node.kind() != "attribute" {
        return None;
    }
    let object = node.child_by_field_name("object")?;
    let attribute = node.child_by_field_name("attribute")?;
    if text_at(source, &object) == "self" {
        Some(text_at(source, &attribute))
    } else {
        None
    }
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
    field.visibility = Visibility::from_keyword(visibility);
    index_by_name.insert(name.to_owned(), found.len());
    found.push(field);
}

fn visibility(name: &str) -> &'static str {
    if name.starts_with("__") {
        "private"
    } else if name.starts_with('_') {
        "protected"
    } else {
        "public"
    }
}

fn collect_imports(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    collect_import_nodes(source, root, &mut out);
    out
}

fn collect_import_nodes(source: &str, node: &Node<'_>, out: &mut Vec<String>) {
    match node.kind() {
        "future_import_statement" => {
            let text_owned = text_at(source, node);
            let text = text_owned.trim();
            let after_from = text.strip_prefix("from ").unwrap_or(text);
            let module = after_from
                .split_once(" import ")
                .map(|(module, _)| module.trim().trim_start_matches('.'))
                .unwrap_or_default();
            if !module.is_empty() {
                dedup_push(out, module.to_owned());
            }
        }
        "import_statement" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
                    dedup_push(out, import_name(source, &child));
                }
            }
        }
        "import_from_statement" => {
            let text_owned = text_at(source, node);
            let text = text_owned.trim();
            let after_from = text.strip_prefix("from ").unwrap_or(text);
            let module_clause = after_from
                .split_once(" import ")
                .map(|(module, _)| module.trim())
                .unwrap_or(after_from.trim());
            let dots = module_clause.chars().take_while(|ch| *ch == '.').count();
            let module = module_clause.trim_start_matches('.').to_owned();
            let name = if dots == 0 {
                module
            } else {
                format!("{}{}", ".".repeat(dots), module)
            };
            if !name.is_empty() {
                dedup_push(out, name);
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_import_nodes(source, &child, out);
    }
}

fn import_name(source: &str, node: &Node<'_>) -> String {
    let text = text_at(source, node);
    text.split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn collect_calls(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    walk_calls(source, root, 0, &mut out);
    out
}

fn walk_calls(source: &str, node: &Node<'_>, depth: usize, out: &mut Vec<String>) {
    if depth > 0
        && matches!(
            node.kind(),
            "function_definition" | "class_definition" | "lambda"
        )
    {
        return;
    }
    if node.kind() == "call"
        && let Some(function) = node.child_by_field_name("function")
    {
        let name = call_name(source, &function);
        if !name.is_empty() {
            dedup_push(out, name);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_calls(source, &child, depth + 1, out);
    }
}

fn call_name(source: &str, node: &Node<'_>) -> String {
    match node.kind() {
        "identifier" => text_at(source, node),
        "attribute" => {
            let owner = node
                .child_by_field_name("object")
                .map(|object| call_name(source, &object))
                .unwrap_or_default();
            let attribute = node
                .child_by_field_name("attribute")
                .map(|attribute| text_at(source, &attribute))
                .unwrap_or_default();
            if owner.is_empty() {
                attribute
            } else {
                format!("{owner}.{attribute}")
            }
        }
        "call" => node
            .child_by_field_name("function")
            .map(|function| call_name(source, &function))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn node_name(source: &str, node: &Node<'_>) -> String {
    match node.kind() {
        "identifier" => text_at(source, node),
        "attribute" => node
            .child_by_field_name("attribute")
            .map(|attribute| text_at(source, &attribute))
            .unwrap_or_default(),
        "subscript" => node
            .child_by_field_name("value")
            .map(|value| node_name(source, &value))
            .unwrap_or_default(),
        "call" => node
            .child_by_field_name("function")
            .map(|function| node_name(source, &function))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn unparse_text(source: &str, node: &Node<'_>) -> String {
    render_expr(&parse_expr(source, node))
}

fn parse_expr(source: &str, node: &Node<'_>) -> PyExpr {
    match node.kind() {
        "identifier" => PyExpr::Name(text_at(source, node)),
        "attribute" => {
            let value = node
                .child_by_field_name("object")
                .map(|object| parse_expr(source, &object))
                .unwrap_or_else(|| PyExpr::Raw(text_at(source, node)));
            let attribute = node
                .child_by_field_name("attribute")
                .map(|attribute| text_at(source, &attribute))
                .unwrap_or_default();
            PyExpr::Attribute(Box::new(value), attribute)
        }
        "subscript" => {
            let value = node
                .child_by_field_name("value")
                .map(|value| parse_expr(source, &value))
                .unwrap_or_else(|| PyExpr::Raw(text_at(source, node)));
            let slice = node
                .child_by_field_name("subscript")
                .map(|slice| parse_expr(source, &slice))
                .unwrap_or_else(|| PyExpr::Raw(text_at(source, node)));
            PyExpr::Subscript(Box::new(value), Box::new(slice))
        }
        "tuple" => {
            let mut cursor = node.walk();
            let elements = node
                .named_children(&mut cursor)
                .map(|child| parse_expr(source, &child))
                .collect();
            PyExpr::Tuple(elements)
        }
        "list" => {
            let mut cursor = node.walk();
            let elements = node
                .named_children(&mut cursor)
                .map(|child| parse_expr(source, &child))
                .collect();
            PyExpr::List(elements)
        }
        "binary_operator" => {
            let text = text_at(source, node);
            let mut cursor = node.walk();
            let children = node.named_children(&mut cursor).collect::<Vec<_>>();
            if children.len() >= 2 {
                let left = parse_expr(source, &children[0]);
                let right = parse_expr(source, &children[1]);
                let operator = if text.contains('|') {
                    "|"
                } else {
                    text.split_whitespace().nth(1).unwrap_or("")
                };
                PyExpr::BinOp(Box::new(left), operator.to_owned(), Box::new(right))
            } else {
                PyExpr::Raw(text)
            }
        }
        "string" | "concatenated_string" => PyExpr::String(text_at(source, node)),
        "type" => {
            let mut cursor = node.walk();
            if let Some(child) = node.named_children(&mut cursor).next() {
                parse_expr(source, &child)
            } else {
                PyExpr::Raw(text_at(source, node))
            }
        }
        "generic_type" => {
            let mut cursor = node.walk();
            let children = node.named_children(&mut cursor).collect::<Vec<_>>();
            if let Some(base) = children.first() {
                let base = parse_expr(source, base);
                if let Some(parameter) = children
                    .iter()
                    .find(|child| child.kind() == "type_parameter")
                {
                    let mut parameter_cursor = parameter.walk();
                    let parts = parameter
                        .named_children(&mut parameter_cursor)
                        .map(|child| render_expr(&parse_expr(source, &child)))
                        .collect::<Vec<_>>();
                    PyExpr::Raw(format!("{}[{}]", render_expr(&base), parts.join(", ")))
                } else {
                    PyExpr::Raw(text_at(source, node))
                }
            } else {
                PyExpr::Raw(text_at(source, node))
            }
        }
        "constrained_type" => PyExpr::Raw(text_at(source, node)),
        "type_parameter" => {
            let mut cursor = node.walk();
            let elements = node
                .named_children(&mut cursor)
                .map(|child| parse_expr(source, &child))
                .collect::<Vec<_>>();
            if elements.len() == 1 {
                elements.into_iter().next().unwrap()
            } else {
                PyExpr::Tuple(elements)
            }
        }
        "call" => {
            let function = node
                .child_by_field_name("function")
                .map(|function| parse_expr(source, &function))
                .unwrap_or_else(|| PyExpr::Raw(String::new()));
            let mut arguments = Vec::new();
            if let Some(argument_list) = node.child_by_field_name("arguments") {
                let mut cursor = argument_list.walk();
                for child in argument_list.named_children(&mut cursor) {
                    arguments.push(parse_expr(source, &child));
                }
            }
            PyExpr::Call(Box::new(function), arguments)
        }
        _ => PyExpr::Raw(text_at(source, node)),
    }
}

fn render_expr(expr: &PyExpr) -> String {
    match expr {
        PyExpr::Raw(text) => text.to_owned(),
        PyExpr::Name(name) => name.clone(),
        PyExpr::Attribute(value, attribute) => format!("{}.{}", render_expr(value), attribute),
        PyExpr::Subscript(value, slice) => {
            format!("{}[{}]", render_expr(value), render_expr(slice))
        }
        PyExpr::Tuple(elements) => match elements.len() {
            0 => "()".to_owned(),
            1 => format!("({},)", render_expr(&elements[0])),
            _ => format!(
                "({})",
                elements
                    .iter()
                    .map(render_expr)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        PyExpr::List(elements) => format!(
            "[{}]",
            elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        PyExpr::BinOp(left, operator, right) => {
            format!("{} {} {}", render_expr(left), operator, render_expr(right))
        }
        PyExpr::String(raw) => unparse_string(raw),
        PyExpr::Call(function, arguments) => format!(
            "{}({})",
            render_expr(function),
            arguments
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn unparse_string(raw: &str) -> String {
    let text = raw.trim();
    let text = text
        .strip_prefix('r')
        .or_else(|| text.strip_prefix('b'))
        .or_else(|| text.strip_prefix('u'))
        .or_else(|| text.strip_prefix('f'))
        .unwrap_or(text);
    if text.len() >= 6 && text.starts_with("\"\"\"") && text.ends_with("\"\"\"") {
        format!("'''{}'''", &text[3..text.len() - 3])
    } else if text.len() >= 2 {
        format!("'{}'", &text[1..text.len() - 1])
    } else {
        text.to_owned()
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
        let animal = entities
            .iter()
            .find(|entity| entity.name == "Animal")
            .unwrap();
        assert_eq!(animal.kind, EntityKind::Class);
        assert!(animal.r#abstract);
        assert!(
            animal
                .members
                .iter()
                .any(|member| member.name == "make_sound")
        );
    }

    #[test]
    fn rejects_incomplete_python_syntax() {
        assert!(
            PythonLanguage
                .parse("def broken(:\n    pass\n", "broken.py")
                .is_empty()
        );
    }

    #[test]
    fn unparses_annotations_like_ast_unparse() {
        let source =
            "x: list[\"Entity\"]\ny: Path | None\nz: dict[str, tuple[int, str]]\nw: X[a:b, c]\n";
        let Some(tree) = parse_python(source) else {
            panic!("expected valid Python");
        };
        let root = tree.root_node();
        let mut cursor = root.walk();
        let annotations = root
            .named_children(&mut cursor)
            .filter_map(|statement| {
                let assignment = statement.named_child(0)?;
                let type_node = assignment.child_by_field_name("type")?;
                Some(unparse_text(source, &type_node))
            })
            .collect::<Vec<_>>();
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

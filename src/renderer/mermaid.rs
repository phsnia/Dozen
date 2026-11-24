//! Mermaid diagram renderer.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::bail;
use regex::Regex;

use crate::model::{Entity, EntityKind};

/// Every diagram kind supported by the CLI and `render_all`.
pub const KINDS: &[&str] = &[
    "class",
    "inheritance",
    "collaboration",
    "call",
    "caller",
    "include",
    "directory",
];

static IDENTIFIER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9_]").unwrap());
static TYPE_TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

const BUILTIN_TYPES: &[&str] = &[
    "bool", "char", "double", "f32", "f64", "float", "i8", "i16", "i32", "i64", "i128", "int",
    "isize", "long", "Self", "short", "signed", "str", "String", "u8", "u16", "u32", "u64", "u128",
    "unsigned", "usize", "void",
];

const BLOCK_KINDS: &[EntityKind] = &[
    EntityKind::Class,
    EntityKind::Struct,
    EntityKind::Trait,
    EntityKind::Interface,
    EntityKind::Enum,
];

/// Render parsed entities as Mermaid diagrams.
#[derive(Clone, Copy, Debug, Default)]
pub struct MermaidRenderer;

impl MermaidRenderer {
    /// Return the human-readable title used for a diagram section.
    pub fn title(kind: &str) -> String {
        match kind {
            "class" => "Class Diagram",
            "inheritance" => "Inheritance Diagram",
            "collaboration" => "Collaboration Diagram",
            "call" => "Call Graph",
            "caller" => "Caller Graph",
            "include" => "Include Graph",
            "directory" => "Directory Diagram",
            _ => return kind.to_owned(),
        }
        .to_owned()
    }

    /// Render one diagram kind, or fail with the list of valid kinds.
    pub fn render(&self, entities: &[Entity], kind: &str) -> anyhow::Result<String> {
        let diagram = match kind {
            "class" => self.class_diagram(entities),
            "inheritance" => self.inheritance_diagram(entities),
            "collaboration" => self.collaboration_diagram(entities),
            "call" => self.call_graph(entities),
            "caller" => self.caller_graph(entities),
            "include" => self.include_graph(entities),
            "directory" => self.directory_graph(entities),
            _ => bail!("Unknown diagram kind '{kind}': {}", KINDS.join(", ")),
        };
        Ok(diagram)
    }

    /// Render a titled Mermaid document containing the selected kinds.
    pub fn render_all(&self, entities: &[Entity], kinds: Option<&[String]>) -> String {
        let selected: Vec<String> = kinds
            .map(|kinds| kinds.to_vec())
            .unwrap_or_else(|| KINDS.iter().map(|kind| (*kind).to_owned()).collect());

        std::iter::once("# Contract Graph\n".to_owned())
            .chain(selected.into_iter().map(|kind| {
                let title = Self::title(&kind);
                let mermaid = self
                    .render(entities, &kind)
                    .unwrap_or_default()
                    .trim_end()
                    .to_owned();
                format!("## {title}\n\n```mermaid\n{mermaid}\n```\n")
            }))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render the detailed class diagram.
    pub fn class_diagram(&self, entities: &[Entity]) -> String {
        self.class_diagram_impl(entities, true, true, true)
    }

    /// Render the inheritance-only diagram.
    pub fn inheritance_diagram(&self, entities: &[Entity]) -> String {
        self.class_diagram_impl(entities, false, false, false)
    }

    /// Render a class diagram with configurable detail and edge kinds.
    fn class_diagram_impl(
        &self,
        entities: &[Entity],
        detailed: bool,
        dependencies: bool,
        include_enums: bool,
    ) -> String {
        let nodes = collect(entities, include_enums);
        let ids = identifiers(&nodes);
        let mut lines = vec!["classDiagram".to_owned()];
        for (index, entity) in nodes.iter().enumerate() {
            lines.extend(block(entity, &ids.ids[&index], detailed));
        }
        lines.extend(class_edges(&nodes, &ids, dependencies));
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Render a flowchart of types used by fields, returns, and parameters.
    pub fn collaboration_diagram(&self, entities: &[Entity]) -> String {
        let nodes = collect(entities, true);
        let ids = identifiers(&nodes);
        let mut lines = vec!["flowchart LR".to_owned()];
        for (index, entity) in nodes.iter().enumerate() {
            lines.push(format!(
                "  {}[\"{}\"]",
                ids.ids[&index],
                label(&entity.name)
            ));
        }

        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        for (index, entity) in nodes.iter().enumerate() {
            let source = &ids.ids[&index];
            for type_name in used_types(entity) {
                let Some(target) = lookup(&type_name, &ids.by_name, &ids.by_file, &entity.path)
                else {
                    continue;
                };
                if target == *source {
                    continue;
                }
                let edge = format!("  {source} -->|uses| {target}");
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        lines.extend(edges);
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Render the callee-oriented call graph.
    pub fn call_graph(&self, entities: &[Entity]) -> String {
        self.call_diagram(entities, false)
    }

    /// Render the caller-oriented call graph.
    pub fn caller_graph(&self, entities: &[Entity]) -> String {
        self.call_diagram(entities, true)
    }

    /// Render a call graph, optionally inverted into a caller graph.
    fn call_diagram(&self, entities: &[Entity], inverse: bool) -> String {
        let functions = functions(entities, "");
        let nodes = function_nodes(&functions);
        let mut lines = vec!["flowchart LR".to_owned()];
        for (index, _entity) in functions.iter().enumerate() {
            let id = &nodes.ids[&index];
            let text = &nodes.labels[&index];
            lines.push(format!("  {id}[\"{}\"]", label(text)));
        }

        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        for (index, (entity, owner)) in functions.iter().enumerate() {
            let source = &nodes.ids[&index];
            for call in &entity.calls {
                let resolved = call_target(call, owner);
                let Some(target) = lookup(&resolved, &nodes.by_name, &nodes.by_file, &entity.path)
                else {
                    continue;
                };
                if target == *source {
                    continue;
                }
                let (a, b) = if inverse {
                    (&target, source)
                } else {
                    (source, &target)
                };
                let edge_label = if inverse { "called by" } else { "calls" };
                let edge = format!("  {a} -->|{edge_label}| {b}");
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        lines.extend(edges);
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Render the module include graph.
    pub fn include_graph(&self, entities: &[Entity]) -> String {
        let modules = modules(entities);
        let (ids, by_key) = module_index(&modules);
        let mut lines = vec!["flowchart LR".to_owned()];
        for (index, module) in modules.iter().enumerate() {
            let file_label = path_file_name(&module.path)
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| module.name.clone());
            lines.push(format!("  {}[\"{}\"]", ids[&index], label(&file_label)));
        }

        let mut external: HashMap<String, String> = HashMap::new();
        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        for (index, module) in modules.iter().enumerate() {
            let source = &ids[&index];
            for target in &module.includes {
                let destination = match resolve_module(target, &by_key) {
                    Some(destination) => destination,
                    None => {
                        let key = target.trim().to_owned();
                        if !external.contains_key(&key) {
                            let identifier = format!("ext_{}", external.len());
                            external.insert(key.clone(), identifier.clone());
                            lines.push(format!("  {identifier}[\"{}\"]", label(&key)));
                        }
                        external[&key].clone()
                    }
                };
                if destination == *source {
                    continue;
                }
                let edge = format!("  {source} --> {destination}");
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        lines.extend(edges);
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Render the directory-to-directory include graph.
    pub fn directory_graph(&self, entities: &[Entity]) -> String {
        let modules = modules(entities);
        let (ids, by_key) = module_index(&modules);
        let file_dirs: HashMap<String, String> = modules
            .iter()
            .enumerate()
            .map(|(index, module)| (ids[&index].clone(), parent_posix(&module.path)))
            .collect();

        let mut dirs: Vec<String> = file_dirs.values().cloned().collect();
        dirs.sort();
        dirs.dedup();
        let dir_ids: HashMap<String, String> = dirs
            .iter()
            .enumerate()
            .map(|(index, dir)| (dir.clone(), format!("dir_{index}")))
            .collect();

        let mut lines = vec!["flowchart LR".to_owned()];
        for dir in &dirs {
            let text = if dir.is_empty() { "." } else { dir.as_str() };
            lines.push(format!("  {}[\"{}\"]", dir_ids[dir], label(text)));
        }

        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        for (index, module) in modules.iter().enumerate() {
            let source_dir = &file_dirs[&ids[&index]];
            for target in &module.includes {
                let Some(destination) = resolve_module(target, &by_key) else {
                    continue;
                };
                let Some(target_dir) = file_dirs.get(&destination) else {
                    continue;
                };
                if target_dir == source_dir {
                    continue;
                }
                let edge = format!("  {} --> {}", dir_ids[source_dir], dir_ids[target_dir]);
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        lines.extend(edges);
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}

/// Identifiers assigned to class-like nodes and lookup tables for edge targets.
struct Identifiers {
    /// Identifier for each collected node index.
    ids: HashMap<usize, String>,
    /// First identifier registered for each entity name.
    by_name: HashMap<String, String>,
    /// First identifier registered for each `(path, name)` pair.
    by_file: HashMap<(String, String), String>,
}

/// Collect class-like declarations, optionally including enums.
fn collect(entities: &[Entity], include_enums: bool) -> Vec<&Entity> {
    entities
        .iter()
        .flat_map(|entity| {
            let current = (BLOCK_KINDS.contains(&entity.kind)
                && (include_enums || !entity.kind.is_enum()))
            .then_some(entity);
            current
                .into_iter()
                .chain(collect(&entity.members, include_enums))
        })
        .collect()
}

/// Assign unique Mermaid identifiers to each collected node.
fn identifiers(nodes: &[&Entity]) -> Identifiers {
    let mut ids = HashMap::new();
    let mut by_name = HashMap::new();
    let mut by_file = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();

    for (index, entity) in nodes.iter().enumerate() {
        let mut base = sanitize(&entity.name);
        if base.is_empty() {
            base = "entity".to_owned();
        }
        let count = used.entry(base.clone()).or_insert(0);
        *count += 1;
        let identifier = if *count == 1 {
            base
        } else {
            format!("{base}_{count}")
        };
        ids.insert(index, identifier.clone());
        by_name
            .entry(entity.name.clone())
            .or_insert_with(|| identifier.clone());
        by_file
            .entry((entity.path.clone(), entity.name.clone()))
            .or_insert(identifier);
    }

    Identifiers {
        ids,
        by_name,
        by_file,
    }
}

/// Render one class block, including fields, methods, and enum cases.
fn block(entity: &Entity, identifier: &str, detailed: bool) -> Vec<String> {
    let mut lines = vec![format!("  class {identifier} {{")];
    let stereotype = stereotype(entity);
    if !stereotype.is_empty() {
        lines.push(format!("    {stereotype}"));
    }
    if detailed {
        for field in &entity.fields {
            let marker = match field.visibility.as_str() {
                "public" => "+",
                "protected" => "#",
                "private" => "-",
                "package" => "~",
                _ => "+",
            };
            let type_ = display_type(&field.r#type);
            let text = if type_.is_empty() {
                field.name.clone()
            } else {
                format!("{type_} {}", field.name)
            };
            let suffix = if field.r#static { "$" } else { "" };
            lines.push(format!("    {marker}{text}{suffix}"));
        }
        for member in &entity.members {
            if !matches!(member.kind, EntityKind::Method | EntityKind::Function) {
                continue;
            }
            lines.push(format!("    +{}()", member.name));
        }
        if entity.kind == EntityKind::Enum {
            for variant in &entity.variants {
                lines.push(format!("    {}", variant.name));
            }
        }
    }
    lines.push("  }".to_owned());
    lines
}

/// Return the Mermaid stereotype for an entity, or an empty string.
fn stereotype(entity: &Entity) -> &'static str {
    if entity.kind == EntityKind::Enum {
        return "<<enumeration>>";
    }
    if matches!(entity.kind, EntityKind::Trait | EntityKind::Interface) {
        return "<<interface>>";
    }
    if entity.r#abstract {
        return "<<abstract>>";
    }
    ""
}

/// Convert a source type into Mermaid-safe display text.
fn display_type(raw: &str) -> String {
    let text = raw
        .replace(['&', '\''], "")
        .replace(['<', '>', '[', ']'], "~")
        .replace('|', " or ");
    WHITESPACE.replace_all(&text, " ").trim().to_owned()
}

/// Build inheritance, implementation, and optional dependency edges.
fn class_edges(nodes: &[&Entity], ids: &Identifiers, dependencies: bool) -> Vec<String> {
    let mut edges = Vec::new();
    let mut seen = HashSet::new();

    for (index, entity) in nodes.iter().enumerate() {
        let child = &ids.ids[&index];
        for base in &entity.bases {
            if let Some(parent) = lookup(base, &ids.by_name, &ids.by_file, &entity.path)
                && parent != *child
            {
                let edge = format!("  {parent} <|-- {child}");
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        for interface in &entity.implements {
            if let Some(parent) = lookup(interface, &ids.by_name, &ids.by_file, &entity.path)
                && parent != *child
            {
                let edge = format!("  {parent} <|.. {child}");
                if seen.insert(edge.clone()) {
                    edges.push(edge);
                }
            }
        }
        if !dependencies {
            continue;
        }
        for field in &entity.fields {
            for type_name in type_names(&field.r#type) {
                if let Some(target) = lookup(&type_name, &ids.by_name, &ids.by_file, &entity.path)
                    && target != *child
                {
                    let edge = format!("  {child} ..> {target}");
                    if seen.insert(edge.clone()) {
                        edges.push(edge);
                    }
                }
            }
        }
    }

    edges
}

/// Resolve a type name against same-file nodes first, then by name.
fn lookup(
    name: &str,
    by_name: &HashMap<String, String>,
    by_file: &HashMap<(String, String), String>,
    path: &str,
) -> Option<String> {
    let mut simple = name.split('<').next().unwrap_or("").trim();
    simple = simple.rsplit("::").next().unwrap_or("").trim();

    if simple.contains('.') {
        let key = (path.to_owned(), simple.to_owned());
        if let Some(found) = by_file.get(&key).or_else(|| by_name.get(simple)) {
            return Some(found.clone());
        }
        simple = simple.rsplit('.').next().unwrap_or(simple).trim();
    }

    by_file
        .get(&(path.to_owned(), simple.to_owned()))
        .or_else(|| by_name.get(simple))
        .cloned()
}

/// Extract referenced type tokens, ignoring built-in scalar types.
fn type_names(raw: &str) -> Vec<String> {
    TYPE_TOKEN
        .find_iter(raw)
        .map(|found| found.as_str())
        .filter(|token| !BUILTIN_TYPES.contains(token))
        .map(str::to_owned)
        .collect()
}

/// Collect unique type names referenced by fields, returns, and parameters.
fn used_types(entity: &Entity) -> Vec<String> {
    entity
        .fields
        .iter()
        .flat_map(|field| type_names(&field.r#type))
        .chain(
            entity
                .members
                .iter()
                .flat_map(|member| type_names(&member.return_type)),
        )
        .chain(entity.members.iter().flat_map(|member| {
            member
                .doc
                .params
                .iter()
                .flat_map(|param| type_names(&param.r#type))
        }))
        .fold(Vec::new(), |mut out, name| {
            if !out.contains(&name) {
                out.push(name);
            }
            out
        })
}

/// Flatten functions and methods, tagging each with its owning type name.
fn functions<'a>(entities: &'a [Entity], owner: &str) -> Vec<(&'a Entity, String)> {
    entities
        .iter()
        .flat_map(|entity| {
            let current = entity
                .kind
                .is_callable()
                .then(|| (entity, owner.to_owned()));
            let children = if BLOCK_KINDS.contains(&entity.kind) {
                functions(&entity.members, &entity.name)
            } else {
                Vec::new()
            };
            current.into_iter().chain(children)
        })
        .collect()
}

/// Identifiers and labels assigned to function and method nodes.
struct FunctionNodes {
    /// Identifier for each callable node index.
    ids: HashMap<usize, String>,
    /// Display label for each callable node index.
    labels: HashMap<usize, String>,
    /// First identifier registered for each name.
    by_name: HashMap<String, String>,
    /// First identifier registered for each `(path, name)` pair.
    by_file: HashMap<(String, String), String>,
}

/// Assign stable identifiers and qualified labels to callable nodes.
fn function_nodes(functions: &[(&Entity, String)]) -> FunctionNodes {
    let mut ids = HashMap::new();
    let mut labels = HashMap::new();
    let mut by_name = HashMap::new();
    let mut by_file = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut file_counts: HashMap<(String, String), usize> = HashMap::new();

    for (entity, _owner) in functions {
        *counts.entry(entity.name.clone()).or_insert(0) += 1;
        *file_counts
            .entry((entity.path.clone(), entity.name.clone()))
            .or_insert(0) += 1;
    }

    for (index, (entity, owner)) in functions.iter().enumerate() {
        let qualified = qualified(entity, owner);
        let mut base = sanitize(&qualified);
        if base.is_empty() {
            base = "call".to_owned();
        }
        base = format!("fn_{base}");
        let count = used.entry(base.clone()).or_insert(0);
        *count += 1;
        let identifier = if *count == 1 {
            base
        } else {
            format!("{base}_{count}")
        };
        ids.insert(index, identifier.clone());
        labels.insert(index, qualified.clone());

        if counts.get(&entity.name) == Some(&1) {
            by_name
                .entry(entity.name.clone())
                .or_insert_with(|| identifier.clone());
        }
        by_name
            .entry(qualified.clone())
            .or_insert_with(|| identifier.clone());
        if !owner.is_empty() {
            by_name
                .entry(format!("{owner}.{}", entity.name))
                .or_insert_with(|| identifier.clone());
        }

        let key = (entity.path.clone(), entity.name.clone());
        if file_counts.get(&key) == Some(&1) {
            by_file.entry(key).or_insert_with(|| identifier.clone());
        }
        by_file
            .entry((entity.path.clone(), qualified.clone()))
            .or_insert_with(|| identifier.clone());
        if !owner.is_empty() {
            by_file
                .entry((entity.path.clone(), format!("{owner}.{}", entity.name)))
                .or_insert(identifier);
        }
    }

    FunctionNodes {
        ids,
        labels,
        by_name,
        by_file,
    }
}

/// Qualify `self.method` and `cls.method` calls with the owning type.
fn call_target(call: &str, owner: &str) -> String {
    if let Some((prefix, rest)) = call.split_once('.')
        && (prefix == "self" || prefix == "cls")
        && !owner.is_empty()
    {
        return format!("{owner}.{rest}");
    }
    call.to_owned()
}

/// Build a dotted label from module, owner, and entity names.
fn qualified(entity: &Entity, owner: &str) -> String {
    let mut parts = Vec::new();
    if !entity.module.is_empty() {
        parts.push(entity.module.clone());
    }
    if !owner.is_empty() {
        parts.push(owner.to_owned());
    }
    if !entity.name.is_empty() {
        parts.push(entity.name.clone());
    }
    if parts.is_empty() {
        entity.name.clone()
    } else {
        parts.join(".")
    }
}

/// Return the module entities from a flat entity list.
fn modules(entities: &[Entity]) -> Vec<&Entity> {
    entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Module)
        .collect()
}

/// Assign module identifiers and index them by path, stem, and name.
fn module_index(modules: &[&Entity]) -> (HashMap<usize, String>, HashMap<String, String>) {
    let mut ids = HashMap::new();
    let mut by_key = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();

    for (index, module) in modules.iter().enumerate() {
        let path = as_posix(&module.path);
        let mut base = sanitize(&path);
        if base.is_empty() {
            base = "module".to_owned();
        }
        base = format!("file_{base}");
        let count = used.entry(base.clone()).or_insert(0);
        *count += 1;
        let identifier = if *count == 1 {
            base
        } else {
            format!("{base}_{count}")
        };
        ids.insert(index, identifier.clone());

        for key in [
            path_file_name(&module.path),
            path_file_stem(&module.path),
            Some(path),
            (!module.module.is_empty()).then(|| module.module.clone()),
        ]
        .into_iter()
        .flatten()
        {
            by_key.entry(key).or_insert_with(|| identifier.clone());
        }
    }

    (ids, by_key)
}

/// Resolve an include target to a known module identifier.
fn resolve_module(target: &str, by_key: &HashMap<String, String>) -> Option<String> {
    for candidate in include_candidates(target) {
        if let Some(found) = by_key.get(&candidate) {
            return Some(found.clone());
        }
    }
    None
}

/// Expand one include string into the names `dox.py` tries to match.
fn include_candidates(target: &str) -> Vec<String> {
    let mut raw = target
        .trim()
        .trim_matches(|ch| matches!(ch, '<' | '>' | '"'))
        .to_owned();
    raw = raw
        .split('{')
        .next()
        .unwrap_or("")
        .trim_end_matches([':', ',', ' '])
        .trim()
        .to_owned();

    let mut candidates = vec![raw.clone()];
    if let Some(name) = path_file_name(&raw) {
        candidates.push(name);
    }
    if let Some(stem) = path_file_stem(&raw) {
        candidates.push(stem);
    }

    let normalized = raw.replace("::", ".").replace('/', ".");
    let parts: Vec<&str> = normalized
        .split('.')
        .filter(|part| !part.is_empty() && !matches!(*part, "crate" | "super" | "self"))
        .collect();
    for size in (1..=parts.len()).rev() {
        candidates.push(parts[..size].join("."));
        candidates.push(parts[..size].join("/"));
        candidates.push(parts[size - 1].to_owned());
    }
    candidates
}

/// Replace every character Mermaid cannot use in an identifier.
fn sanitize(value: &str) -> String {
    IDENTIFIER.replace_all(value, "_").into_owned()
}

/// Escape text for a double-quoted Mermaid label.
fn label(text: &str) -> String {
    text.replace('"', "'").replace('\n', " ")
}

/// Normalize a path to POSIX separators for display.
fn as_posix(path: &str) -> String {
    if path.is_empty() {
        ".".to_owned()
    } else {
        Path::new(path).to_string_lossy().replace('\\', "/")
    }
}

/// Return the final path component, when there is one.
fn path_file_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// Return the final path component without its extension.
fn path_file_stem(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
}

/// Return the parent directory in POSIX form, using `.` for none.
fn parent_posix(path: &str) -> String {
    let path = Path::new(path);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            parent.to_string_lossy().replace('\\', "/")
        }
        _ if path.is_absolute() => "/".to_owned(),
        _ => ".".to_owned(),
    }
}

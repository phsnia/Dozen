//! Parse source files and write rendered output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use encoding_rs::Encoding;

use crate::config::Config;
use crate::model::Entity;
use crate::registry;
use crate::renderer::mermaid::{KINDS, MermaidRenderer};
use crate::renderer::reference::ReferenceRenderer;

pub struct Converter {
    cfg: Config,
    renderer: ReferenceRenderer,
}

impl Converter {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            renderer: ReferenceRenderer,
        }
    }

    /// Parse one source file into entities.
    pub fn entities(&self, path: &Path) -> Result<Vec<Entity>> {
        let Some(language) = registry::language_for(path) else {
            return Ok(Vec::new());
        };
        let bytes =
            std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        let source = decode(&bytes, &self.cfg.encoding)
            .with_context(|| format!("Failed to decode {}", path.display()))?;
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let mut entities = language.parse(&source, &filename);
        for entity in &mut entities {
            stamp(entity, path);
        }
        Ok(entities)
    }

    /// Convert one source file to Markdown.
    pub fn file(&self, path: &Path) -> Result<String> {
        Ok(self.renderer.render(&self.entities(path)?))
    }

    /// Parse several source files into one entity list.
    pub fn project(&self, paths: &[PathBuf]) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        for path in paths {
            entities.extend(self.entities(path)?);
        }
        Ok(entities)
    }

    /// Convert several source files into one Markdown diagram file.
    pub fn diagram(&self, paths: &[PathBuf], output: &Path, kind: &str) -> Result<()> {
        let kinds: Vec<String> = if kind == "all" {
            KINDS.iter().map(|kind| (*kind).to_owned()).collect()
        } else {
            vec![kind.to_owned()]
        };
        let text = MermaidRenderer.render_all(&self.project(paths)?, Some(&kinds));
        write_text(output, &text, &self.cfg.encoding)
    }

    /// Convert several source files into one Markdown file.
    pub fn files(&self, paths: &[PathBuf], output: &Path) -> Result<()> {
        let mut docs = Vec::new();
        for path in paths {
            let markdown = self.file(path)?;
            if !markdown.trim().is_empty() {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                docs.push(format!("# {name}\n\n{markdown}"));
            }
        }
        let text = format!("{}\n", docs.join("\n\n"));
        write_text(output, &text, &self.cfg.encoding)
    }
}

fn stamp(entity: &mut Entity, path: &Path) {
    entity.path = path.to_string_lossy().into_owned();
    entity.module = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    for member in &mut entity.members {
        stamp(member, path);
    }
}

fn decode(bytes: &[u8], encoding: &str) -> Result<String> {
    let encoding = Encoding::for_label(encoding.as_bytes())
        .with_context(|| format!("Unknown encoding: {encoding}"))?;
    let (text, _, had_errors) = encoding.decode(bytes);
    if had_errors {
        bail!("Invalid byte sequence for {}", encoding.name());
    }
    Ok(text.into_owned())
}

fn encode(text: &str, encoding: &str) -> Result<Vec<u8>> {
    let encoding = Encoding::for_label(encoding.as_bytes())
        .with_context(|| format!("Unknown encoding: {encoding}"))?;
    let (bytes, _, had_errors) = encoding.encode(text);
    if had_errors {
        bail!("Text cannot be represented in {}", encoding.name());
    }
    Ok(bytes.into_owned())
}

fn write_text(path: &Path, text: &str, encoding: &str) -> Result<()> {
    let bytes = encode(text, encoding)?;
    std::fs::write(path, bytes).with_context(|| format!("Failed to write {}", path.display()))
}

//! Parse source files and write rendered output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use encoding_rs::Encoding;

use crate::engine::config::Config;
use crate::lang::registry;
use crate::model::Entity;
use crate::renderer::markdown::MarkdownRenderer;
use crate::renderer::mermaid::{KINDS, MermaidRenderer};

/// Character encoding boundary for all files the tool reads or writes.
pub struct TextCodec {
    /// Encoding label used for both directions.
    encoding: String,
}

impl TextCodec {
    /// Create a codec for `encoding`, a label such as `utf-8` or `latin-1`.
    pub fn new(encoding: impl Into<String>) -> Self {
        Self {
            encoding: encoding.into(),
        }
    }

    /// Read and decode a UTF-8 path according to the configured encoding.
    pub fn read(&self, path: &Path) -> Result<String> {
        let bytes =
            std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        self.decode(&bytes)
            .with_context(|| format!("Failed to decode {}", path.display()))
    }

    /// Decode bytes, rejecting unknown labels and invalid sequences.
    pub fn decode(&self, bytes: &[u8]) -> Result<String> {
        let encoding = encoding_for(&self.encoding)?;
        let (text, _, had_errors) = encoding.decode(bytes);
        if had_errors {
            bail!("Invalid byte sequence for {}", encoding.name());
        }
        Ok(text.into_owned())
    }

    /// Encode text for output, rejecting unknown labels and unrepresentable text.
    pub fn encode(&self, text: &str) -> Result<Vec<u8>> {
        let encoding = encoding_for(&self.encoding)?;
        let (bytes, _, had_errors) = encoding.encode(text);
        if had_errors {
            bail!("Text cannot be represented in {}", encoding.name());
        }
        Ok(bytes.into_owned())
    }

    /// Encode and write `text`, creating no directories on its own.
    pub fn write(&self, path: &Path, text: &str) -> Result<()> {
        let bytes = self.encode(text)?;
        std::fs::write(path, bytes).with_context(|| format!("Failed to write {}", path.display()))
    }
}

/// Resolve an encoding label with Python-compatible error reporting.
fn encoding_for(label: &str) -> Result<&'static Encoding> {
    Encoding::for_label(label.as_bytes()).with_context(|| format!("Unknown encoding: {label}"))
}

/// The conversion pipeline: source files in, Markdown or Mermaid out.
pub struct Converter {
    /// Markdown reference renderer.
    markdown: MarkdownRenderer,
    /// Mermaid diagram renderer.
    mermaid: MermaidRenderer,
    /// Encoding boundary for source and output files.
    codec: TextCodec,
}

impl Converter {
    /// Create a converter using the encoding in `cfg`.
    pub fn new(cfg: Config) -> Self {
        let codec = TextCodec::new(cfg.encoding.clone());
        Self {
            markdown: MarkdownRenderer::new(),
            mermaid: MermaidRenderer,
            codec,
        }
    }

    /// Parse one source file into entities.
    pub fn entities(&self, path: &Path) -> Result<Vec<Entity>> {
        let Some(language) = registry::REGISTRY.language_for(path) else {
            return Ok(Vec::new());
        };

        let source = self.codec.read(path)?;
        let filename = display_name(path);
        let mut entities = language.parse(&source, &filename);
        entities.iter_mut().for_each(|entity| entity.stamp(path));
        Ok(entities)
    }

    /// Convert one source file to Markdown.
    pub fn file(&self, path: &Path) -> Result<String> {
        Ok(self.markdown.render(&self.entities(path)?))
    }

    /// Parse several source files into one entity list.
    pub fn project(&self, paths: &[PathBuf]) -> Result<Vec<Entity>> {
        paths
            .iter()
            .map(|path| self.entities(path))
            .collect::<Result<Vec<_>>>()
            .map(|groups| groups.into_iter().flatten().collect())
    }

    /// Convert several source files into one Markdown diagram file.
    pub fn diagram(&self, paths: &[PathBuf], output: &Path, kind: &str) -> Result<()> {
        let kinds: Vec<String> = if kind == "all" {
            KINDS.iter().map(|kind| (*kind).to_owned()).collect()
        } else {
            vec![kind.to_owned()]
        };
        let entities = self.project(paths)?;
        self.codec
            .write(output, &self.mermaid.render_all(&entities, Some(&kinds)))
    }

    /// Convert several source files into one Markdown file.
    pub fn files(&self, paths: &[PathBuf], output: &Path) -> Result<()> {
        let text = paths
            .iter()
            .map(|path| self.document(path))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n\n");
        self.codec.write(output, &format!("{text}\n"))
    }

    /// Render one file as a titled document, or `None` when it has no content.
    fn document(&self, path: &Path) -> Result<Option<String>> {
        let markdown = self.file(path)?;
        Ok(
            (!markdown.trim().is_empty())
                .then(|| format!("# {}\n\n{markdown}", display_name(path))),
        )
    }
}

/// Use the file name when present, otherwise the full path.
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;

use crate::error::ZeniiError;

/// File extensions that require external binary conversion (non-UTF-8 or structured binary formats).
/// Everything else is read directly as UTF-8 text.
const BINARY_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "doc", "pptx", "ppt", "xlsx", "xls", "jpg", "jpeg", "png", "gif", "bmp", "tiff",
    "webp", "zip", "epub", "html", "htm",
];

/// Converts a file at a given path to a markdown string.
///
/// Implementations must be async-safe and non-blocking. For subprocess-based converters,
/// use `tokio::process::Command` (epoll-backed, never blocks the tokio runtime).
///
/// To swap converters, implement this trait and update the `doc_converter_bin` config field.
/// The handler code requires zero changes.
#[async_trait]
pub trait DocumentConverter: Send + Sync {
    /// Returns true if this converter handles the given lowercase extension (no leading dot).
    fn supports(&self, extension: &str) -> bool;

    /// Convert the file at `path` to a markdown string.
    async fn convert(&self, path: &Path) -> Result<String, ZeniiError>;
}

/// Converts documents using the `markitdown` CLI tool.
///
/// Install: `pip install markitdown[all]`
///
/// Supports: PDF, DOCX, PPTX, XLSX, HTML, images, ZIP, EPUB and more.
/// For image descriptions and OCR, set the OPENAI_API_KEY environment variable.
pub struct MarkItDownConverter {
    /// Path to the markitdown binary (default: "markitdown", resolved via PATH).
    bin: String,
    /// Maximum time to wait for the subprocess before killing it and returning an error.
    /// Prevents a hanging PDF from blocking a Tokio thread indefinitely (L9 fix).
    timeout: Duration,
}

impl MarkItDownConverter {
    pub fn new(bin: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            timeout: Duration::from_secs(60),
        }
    }

    /// Create a converter with an explicit subprocess timeout (L9 fix).
    pub fn with_timeout(bin: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            bin: bin.into(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }
}

#[async_trait]
impl DocumentConverter for MarkItDownConverter {
    fn supports(&self, ext: &str) -> bool {
        BINARY_EXTENSIONS.contains(&ext)
    }

    async fn convert(&self, path: &Path) -> Result<String, ZeniiError> {
        // tokio::process::Command is epoll/kqueue-backed — does NOT block the tokio runtime.
        // Wrap with tokio::time::timeout to kill hanging subprocesses (L9 fix).
        let output_future = tokio::process::Command::new(&self.bin).arg(path).output();

        let output = tokio::time::timeout(self.timeout, output_future)
            .await
            .map_err(|_| {
                ZeniiError::Conversion(format!(
                    "'{}' timed out after {}s — increase wiki_convert_timeout_secs in config",
                    self.bin,
                    self.timeout.as_secs()
                ))
            })?
            .map_err(|e| {
                ZeniiError::Conversion(format!(
                    "'{}' not found or failed to launch: {} — install with: pip install markitdown[all]",
                    self.bin, e
                ))
            })?;

        if !output.status.success() {
            return Err(ZeniiError::Conversion(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Convert a file to markdown using the provided converter.
///
/// - Binary extensions (PDF, DOCX, etc.): delegates to `converter.convert()`
/// - All other extensions: reads the file as UTF-8 text directly (no subprocess)
pub async fn convert_file(
    path: &Path,
    converter: &dyn DocumentConverter,
) -> Result<String, ZeniiError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if converter.supports(&ext) {
        converter.convert(path).await
    } else {
        tokio::fs::read_to_string(path)
            .await
            .map_err(ZeniiError::Io)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn converter() -> MarkItDownConverter {
        MarkItDownConverter::new("markitdown")
    }

    // Text extensions: MarkItDownConverter should NOT claim support.
    #[test]
    fn text_extensions_not_supported_by_markitdown() {
        let c = converter();
        for ext in &["md", "txt", "rs", "ts", "py", "go", "json", "toml", "yaml"] {
            assert!(!c.supports(ext), "should not support .{ext}");
        }
    }

    // Binary extensions: MarkItDownConverter should claim support.
    #[test]
    fn binary_extensions_supported_by_markitdown() {
        let c = converter();
        for ext in &["pdf", "docx", "pptx", "xlsx", "jpg", "png", "epub", "html"] {
            assert!(c.supports(ext), "should support .{ext}");
        }
    }

    // When markitdown binary is absent, convert() returns ZeniiError::Conversion
    // with the install hint.
    #[tokio::test]
    async fn markitdown_missing_returns_conversion_error() {
        let c = MarkItDownConverter::new("/nonexistent/markitdown-xyz");
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "dummy").unwrap();
        let err = c.convert(tmp.path()).await.unwrap_err();
        match err {
            ZeniiError::Conversion(msg) => {
                assert!(msg.contains("markitdown[all]"), "hint missing: {msg}");
            }
            other => panic!("expected Conversion, got {other:?}"),
        }
    }

    // Text file with unknown/text extension is read directly as UTF-8.
    #[tokio::test]
    async fn convert_file_reads_text_directly() {
        let c = converter();
        let mut tmp = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(tmp, "# Hello").unwrap();
        let result = convert_file(tmp.path(), &c).await.unwrap();
        assert!(result.contains("# Hello"));
    }
}

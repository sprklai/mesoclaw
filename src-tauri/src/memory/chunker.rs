//! Text chunking for the memory subsystem.
//!
//! Long documents are split into overlapping chunks so that each chunk fits
//! within an embedding model's context window.  Overlap ensures semantic
//! continuity across chunk boundaries.

// ─── ChunkConfig ─────────────────────────────────────────────────────────────

/// Configuration for the text chunker.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target number of words per chunk.  Default: 512.
    pub chunk_size: usize,
    /// Number of words shared between adjacent chunks.  Default: 50.
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            overlap: 50,
        }
    }
}

// ─── Chunk ───────────────────────────────────────────────────────────────────

/// A single chunk produced by [`split_into_chunks`].
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The text content of this chunk.
    pub text: String,
    /// Zero-based index of this chunk within the original document.
    pub chunk_index: usize,
    /// Index of the first word (inclusive) from the source word list.
    pub start_word: usize,
    /// Index of the last word (exclusive) from the source word list.
    pub end_word: usize,
}

// ─── split_into_chunks ───────────────────────────────────────────────────────

/// Split `text` into overlapping word-boundary chunks using `config`.
///
/// Returns a `Vec<Chunk>`.  If `text` is empty or shorter than one
/// chunk, a single chunk (or an empty vec for truly empty input) is returned.
pub fn split_into_chunks(text: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    let step = if config.chunk_size > config.overlap {
        config.chunk_size - config.overlap
    } else {
        // Degenerate config: step by 1 to avoid infinite loop.
        1
    };

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    while start < words.len() {
        let end = (start + config.chunk_size).min(words.len());
        let text = words[start..end].join(" ");
        chunks.push(Chunk {
            text,
            chunk_index: idx,
            start_word: start,
            end_word: end,
        });
        if end == words.len() {
            break;
        }
        start += step;
        idx += 1;
    }

    chunks
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_empty() {
        let chunks = split_into_chunks("", &ChunkConfig::default());
        assert!(chunks.is_empty(), "empty text → no chunks");
    }

    #[test]
    fn whitespace_only_returns_empty() {
        let chunks = split_into_chunks("   \n\t  ", &ChunkConfig::default());
        assert!(chunks.is_empty(), "whitespace-only → no chunks");
    }

    #[test]
    fn short_text_single_chunk() {
        let text = "one two three";
        let config = ChunkConfig {
            chunk_size: 512,
            overlap: 50,
        };
        let chunks = split_into_chunks(text, &config);
        assert_eq!(chunks.len(), 1, "short text → single chunk");
        assert_eq!(chunks[0].text, text);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].start_word, 0);
        assert_eq!(chunks[0].end_word, 3);
    }

    #[test]
    fn long_text_multiple_chunks() {
        // 20 words, chunk_size=10, overlap=2 → step=8 → chunks at 0, 8, 16
        let words: Vec<String> = (1..=20).map(|i| format!("word{i}")).collect();
        let text = words.join(" ");
        let config = ChunkConfig {
            chunk_size: 10,
            overlap: 2,
        };
        let chunks = split_into_chunks(&text, &config);
        assert!(
            chunks.len() >= 2,
            "long text → multiple chunks, got {}",
            chunks.len()
        );
    }

    #[test]
    fn chunk_overlap_maintained() {
        // With chunk_size=5 and overlap=2 the last 2 words of chunk N
        // should appear as the first 2 words of chunk N+1.
        let words: Vec<String> = (1..=12).map(|i| format!("w{i}")).collect();
        let text = words.join(" ");
        let config = ChunkConfig {
            chunk_size: 5,
            overlap: 2,
        };
        let chunks = split_into_chunks(&text, &config);
        assert!(chunks.len() >= 2, "need at least 2 chunks for overlap test");

        let last_words_of_first: Vec<&str> =
            chunks[0].text.split_whitespace().rev().take(2).collect();
        let first_words_of_second: Vec<&str> = chunks[1].text.split_whitespace().take(2).collect();

        // The last 2 of chunk 0 reversed should equal the first 2 of chunk 1.
        let mut expected = last_words_of_first;
        expected.reverse();
        assert_eq!(
            expected, first_words_of_second,
            "last 2 words of chunk 0 should be first 2 words of chunk 1"
        );
    }

    #[test]
    fn chunk_config_defaults() {
        let config = ChunkConfig::default();
        assert_eq!(config.chunk_size, 512);
        assert_eq!(config.overlap, 50);
    }

    #[test]
    fn exact_chunk_boundary() {
        // 10 words, chunk_size=10 → exactly one chunk
        let words: Vec<String> = (1..=10).map(|i| format!("w{i}")).collect();
        let text = words.join(" ");
        let config = ChunkConfig {
            chunk_size: 10,
            overlap: 0,
        };
        let chunks = split_into_chunks(&text, &config);
        assert_eq!(chunks.len(), 1, "exactly chunk_size words → 1 chunk");
        assert_eq!(chunks[0].end_word, 10);
    }

    #[test]
    fn chunk_indices_sequential() {
        let words: Vec<String> = (1..=30).map(|i| format!("w{i}")).collect();
        let text = words.join(" ");
        let config = ChunkConfig {
            chunk_size: 10,
            overlap: 2,
        };
        let chunks = split_into_chunks(&text, &config);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i, "chunk_index should be sequential");
        }
    }
}

#[cfg(feature = "channels")]
pub mod channel;
pub mod chat;
pub mod config;
pub mod daemon;
pub mod embedding;
pub mod key;
pub mod memory;
pub mod onboard;
pub mod plugin;
pub mod provider;
pub mod run;
#[cfg(feature = "scheduler")]
pub mod schedule;
pub mod wiki;
pub mod workflow;

use percent_encoding::{percent_encode, AsciiSet, CONTROLS};

// Encodes characters that would break URL path segments (/, ?, #, %, space).
// Preserves : and other safe path characters so existing credential keys are unchanged.
const PATH_CHARS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'%');

// Encodes characters that would corrupt URL query string values (&, =, +, #, %).
#[cfg(feature = "channels")]
const QUERY_CHARS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'&')
    .add(b'=')
    .add(b'+')
    .add(b'#')
    .add(b'%');

pub fn encode_path_segment(s: &str) -> String {
    percent_encode(s.as_bytes(), PATH_CHARS).to_string()
}

pub fn encode_query_value(s: &str) -> String {
    percent_encode(s.as_bytes(), QUERY_CHARS).to_string()
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars().take(max.saturating_sub(3)).collect::<String>()
        )
    }
}

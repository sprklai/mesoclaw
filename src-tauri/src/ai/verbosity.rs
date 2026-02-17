use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Verbosity level for AI-generated explanations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Concise,
    #[default]
    Balanced,
    Detailed,
}

impl FromStr for Verbosity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "concise" => Ok(Verbosity::Concise),
            "balanced" => Ok(Verbosity::Balanced),
            "detailed" => Ok(Verbosity::Detailed),
            _ => Err(format!(
                "Invalid verbosity: '{}'. Valid options: concise, balanced, detailed",
                s
            )),
        }
    }
}

impl Verbosity {
    /// Returns the prompt suffix for this verbosity level
    pub fn suffix(self) -> &'static str {
        match self {
            Verbosity::Concise => {
                "Keep your explanation extremely brief - 1-2 sentences maximum, covering only the essential purpose."
            }
            Verbosity::Balanced => {
                "Provide a standard explanation with 3-4 sentences covering purpose and key details."
            }
            Verbosity::Detailed => {
                "Provide a comprehensive explanation with 6-8 sentences, including purpose, usage patterns, edge cases, and examples where relevant."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_from_str_valid() {
        assert_eq!(Verbosity::from_str("concise"), Ok(Verbosity::Concise));
        assert_eq!(Verbosity::from_str("Concise"), Ok(Verbosity::Concise));
        assert_eq!(Verbosity::from_str("CONCISE"), Ok(Verbosity::Concise));
        assert_eq!(Verbosity::from_str("balanced"), Ok(Verbosity::Balanced));
        assert_eq!(Verbosity::from_str("detailed"), Ok(Verbosity::Detailed));
    }

    #[test]
    fn test_verbosity_from_str_invalid() {
        assert!(Verbosity::from_str("invalid").is_err());
        let err = Verbosity::from_str("invalid").unwrap_err();
        assert!(err.contains("Invalid verbosity"));
    }

    #[test]
    fn test_verbosity_suffix() {
        assert!(Verbosity::Concise.suffix().contains("1-2 sentences"));
        assert!(Verbosity::Balanced.suffix().contains("3-4 sentences"));
        assert!(Verbosity::Detailed.suffix().contains("6-8 sentences"));
    }

    #[test]
    fn test_verbosity_default() {
        assert_eq!(Verbosity::default(), Verbosity::Balanced);
    }
}

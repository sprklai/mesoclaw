/// AI utility functions for extracting structured data from LLM responses.

/// Extract confidence score from LLM response.
///
/// Looks for various patterns of confidence indicators in the response text
/// and returns a numeric confidence score.
///
/// # Returns
/// - `0.9` for High confidence
/// - `0.6` for Medium confidence
/// - `0.3` for Low confidence
/// - `0.5` for Unknown/No confidence (default)
pub fn extract_confidence_from_llm_response(content: &str) -> f32 {
    let content_lower = content.to_lowercase();

    // Check for High confidence patterns
    if content_lower.contains("**confidence**: high")
        || content_lower.contains("confidence: high")
        || content_lower.contains("confidence**: high")
        || content_lower.contains("high confidence")
        || content_lower.contains("confidence:** high")
    {
        return 0.9;
    }

    // Check for Medium confidence patterns
    if content_lower.contains("**confidence**: medium")
        || content_lower.contains("confidence: medium")
        || content_lower.contains("confidence**: medium")
        || content_lower.contains("medium confidence")
        || content_lower.contains("confidence:** medium")
    {
        return 0.6;
    }

    // Check for Low confidence patterns
    if content_lower.contains("**confidence**: low")
        || content_lower.contains("confidence: low")
        || content_lower.contains("confidence**: low")
        || content_lower.contains("low confidence")
        || content_lower.contains("confidence:** low")
    {
        return 0.3;
    }

    0.5
}

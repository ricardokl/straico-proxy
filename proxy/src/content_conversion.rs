//! Content conversion utilities for transforming OpenAI format to Straico format.
//!
//! This module provides comprehensive conversion functions to handle the dual content
//! format support required by the OpenAI API compatibility layer.

use crate::openai_types::{OpenAiContent, OpenAiContentObject};
use straico_client::endpoints::chat::ContentObject;

/// Normalizes OpenAI content to always be in array format.
///
/// This is useful for consistent processing regardless of input format.
///
/// # Arguments
/// * `content` - The OpenAI content to normalize
///
/// # Returns
/// Vector of OpenAiContentObject representing the content
pub fn normalize_openai_content_to_array(content: OpenAiContent) -> Vec<OpenAiContentObject> {
    match content {
        OpenAiContent::String(text) => {
            vec![OpenAiContentObject {
                content_type: "text".to_string(),
                text,
            }]
        }
        OpenAiContent::Array(objects) => objects,
        OpenAiContent::Null => vec![], // Empty array for null content
    }
}

/// Extracts text content from any content format.
///
/// # Arguments
/// * `content` - The content to extract text from
///
/// # Returns
/// Concatenated text string
pub fn extract_text_from_content(content: &[ContentObject]) -> String {
    content
        .iter()
        .map(|obj| &obj.text)
        .cloned()
        .collect::<Vec<_>>()
        .join("")
}

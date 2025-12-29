use super::types::{ChatFunctionCall, ModelProvider, ToolCall};
use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

static XML_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<tool_calls>(.*?)</tool_calls>").unwrap());

static XML_SINGLE_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<tool_call>(.*?)</tool_call>").unwrap());

static XML_ARG_KEY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<arg_key>(.*?)</arg_key>").unwrap());

static XML_ARG_VALUE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<arg_value>(.*?)</arg_value>").unwrap());

static MOONSHOT_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<\|tool_call_begin\|>(.*?)<\|tool_call_end\|>").unwrap());

/// Converts a ChatFunctionCall into a full ToolCall with generated ID
pub fn function_call_to_tool_call(function: ChatFunctionCall) -> ToolCall {
    ToolCall {
        id: format!("call_{}", Uuid::new_v4()),
        tool_type: "function".to_string(),
        function,
        index: None,
    }
}

/// Try parsing JSON tool calls from a <tool_calls> XML tag
pub fn try_parse_json_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    let raw_json = XML_TOOL_CALL_REGEX
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())?;

    // First try the simplified format: array of {"name", "arguments"}
    if let Ok(functions) = serde_json::from_str::<Vec<ChatFunctionCall>>(&raw_json) {
        return Some(
            functions
                .into_iter()
                .map(function_call_to_tool_call)
                .collect(),
        );
    }

    // Fallback: try the legacy OpenAI tool_call schema for backwards compatibility
    serde_json::from_str::<Vec<ToolCall>>(&raw_json).ok()
}

pub fn try_parse_xml_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    let mut tool_calls = Vec::new();

    for cap in XML_SINGLE_TOOL_CALL_REGEX.captures_iter(content) {
        let mut inner = match cap.get(1) {
            Some(m) => m.as_str().trim(),
            None => continue,
        };

        // Handle markdown code blocks if present
        if inner.starts_with("```")
            && let Some(end_idx) = inner.rfind("```")
            && end_idx > 3
        {
            let block_content = &inner[3..end_idx].trim();
            // Skip optional language identifier (like 'json')
            if let Some(newline_idx) = block_content.find('\n') {
                let potential_lang = block_content[..newline_idx].trim();
                if !potential_lang.contains('{') && !potential_lang.contains('[') {
                    inner = block_content[newline_idx..].trim();
                } else {
                    inner = block_content;
                }
            } else {
                inner = block_content;
            }
        }

        // 1. First try parsing the inner content as JSON (Qwen format: {"name": "...", "arguments": {...}})
        if let Ok(func) = serde_json::from_str::<ChatFunctionCall>(inner) {
            tool_calls.push(function_call_to_tool_call(func));
            continue;
        }

        // 2. Fallback to parsing the XML arg_key/arg_value format
        // Extract function name (first line/word)
        let mut lines = inner.lines();
        let function_name = match lines.next() {
            Some(line) => line.trim().to_string(),
            None => continue,
        };

        if function_name.is_empty() {
            continue;
        }

        // Build JSON arguments by collecting keys and values separately
        let keys: Vec<_> = XML_ARG_KEY_REGEX
            .captures_iter(inner)
            .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .collect();

        let values: Vec<_> = XML_ARG_VALUE_REGEX
            .captures_iter(inner)
            .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .collect();

        if !keys.is_empty() && keys.len() == values.len() {
            let mut args_map = serde_json::Map::new();
            for (k, v) in keys.into_iter().zip(values) {
                // Ensure values are properly JSON-escaped by storing them as serde_json::Value::String
                args_map.insert(k, serde_json::Value::String(v));
            }

            tool_calls.push(function_call_to_tool_call(ChatFunctionCall {
                name: function_name,
                arguments: serde_json::Value::Object(args_map),
            }));
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Helper to try parsing Moonshot tool calls
pub fn try_parse_moonshot_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    if !content.contains("<|tool_calls_section_begin|>") {
        return None;
    }

    let mut tool_calls = Vec::new();

    for cap in MOONSHOT_TOOL_CALL_REGEX.captures_iter(content) {
        let inner = match cap.get(1) {
            Some(m) => m.as_str(),
            None => continue,
        };

        // Split into function name and arguments
        // Format: functions.view:0<|tool_call_argument_begin|>{"file_path": "..."}
        let parts: Vec<&str> = inner.split("<|tool_call_argument_begin|>").collect();
        if parts.len() != 2 {
            continue;
        }

        let raw_function_name = parts[0].trim();
        let args_json_str = parts[1].trim();

        // Clean up function name: remove "functions." prefix and ":0" suffix
        let function_name = raw_function_name
            .trim_start_matches("functions.")
            .split(':')
            .next()
            .unwrap_or(raw_function_name)
            .to_string();

        // Validate and parse JSON
        if let Ok(args_value) = serde_json::from_str::<serde_json::Value>(args_json_str) {
            tool_calls.push(function_call_to_tool_call(ChatFunctionCall {
                name: function_name,
                arguments: args_value,
            }));
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Dispatches parsing to the appropriate function based on provider and content
pub fn parse_tool_calls(content: &str, provider: ModelProvider) -> Option<Vec<ToolCall>> {
    match provider {
        ModelProvider::Zai => try_parse_xml_tool_call(content)
            .or_else(|| try_parse_json_tool_call(content))
            .or_else(|| try_parse_moonshot_tool_call(content)),
        ModelProvider::MoonshotAI => {
            try_parse_moonshot_tool_call(content).or_else(|| try_parse_json_tool_call(content))
        }
        ModelProvider::Qwen => {
            try_parse_xml_tool_call(content).or_else(|| try_parse_json_tool_call(content))
        }
        ModelProvider::Anthropic
        | ModelProvider::Google
        | ModelProvider::OpenAI
        | ModelProvider::Unknown => try_parse_json_tool_call(content)
            .or_else(|| try_parse_xml_tool_call(content))
            .or_else(|| try_parse_moonshot_tool_call(content)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen_xml_json_parsing() {
        // Test clean JSON
        let content1 =
            "<tool_call>\n{\"name\": \"func1\", \"arguments\": {\"k\": \"v\"}}\n</tool_call>";
        let tool_calls1 = try_parse_xml_tool_call(content1).expect("Should parse clean JSON");
        assert_eq!(tool_calls1[0].function.name, "func1");

        // Test JSON in markdown block
        let content2 = "<tool_call>\n```json\n{\"name\": \"func2\", \"arguments\": {\"k\": \"v\"}}\n```\n</tool_call>";
        let tool_calls2 = try_parse_xml_tool_call(content2).expect("Should parse markdown JSON");
        assert_eq!(tool_calls2[0].function.name, "func2");
    }

    #[test]
    fn test_xml_custom_format_parsing() {
        let content = r#"<tool_call>read
<arg_key>filePath</arg_key>
<arg_value>/tmp/test_file.txt</arg_value>
</tool_call>"#;
        let tool_calls = try_parse_xml_tool_call(content).expect("Should parse XML custom format");
        assert_eq!(tool_calls[0].function.name, "read");
        assert_eq!(
            tool_calls[0].function.arguments["filePath"],
            "/tmp/test_file.txt"
        );
    }

    #[test]
    fn test_moonshot_parsing() {
        let content = r#"<|tool_calls_section_begin|><|tool_call_begin|>functions.view:0<|tool_call_argument_begin|>{"file_path": "/tmp/random_file.txt"}<|tool_call_end|><|tool_calls_section_end|>"#;
        let tool_calls =
            try_parse_moonshot_tool_call(content).expect("Should parse Moonshot format");
        assert_eq!(tool_calls[0].function.name, "view");
        assert_eq!(
            tool_calls[0].function.arguments["file_path"],
            "/tmp/random_file.txt"
        );
    }
}

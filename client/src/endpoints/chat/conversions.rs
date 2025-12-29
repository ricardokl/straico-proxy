use super::{
    ChatContent, ChatError, ChatFunctionCall, ChatMessage, OpenAiChatMessage, ToolCall,
    common_types::ModelProvider,
    request_types::{ChatRequest, OpenAiChatRequest, OpenAiTool, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};
use log::debug;
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

/// Shared preamble for all providers, standardizing the function definitions section.
fn build_tools_preamble(
    functions: &[&crate::endpoints::chat::request_types::OpenAiFunction],
) -> Result<String, ChatError> {
    Ok(format!(
        "You are provided with function signatures within <tools></tools> XML tags:\n<tools>\n{}\n</tools>",
        serde_json::to_string_pretty(&functions)?
    ))
}

fn zai_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_call>{function_name}
<arg_key>{parameter_name}</arg_key>
<arg_value>{parameter_value}</arg_value>
</tool_call>

Each tool call must be wrapped in <tool_call> tags with the function name immediately after the opening tag. Parameters are specified using <arg_key> and <arg_value> pairs.

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_call>get_weather
<arg_key>location</arg_key>
<arg_value>Boston, MA</arg_value>
</tool_call>

Example of multiple tool calls:

<tool_call>search_web
<arg_key>query</arg_key>
<arg_value>latest AI news</arg_value>
</tool_call>
<tool_call>summarize_text
<arg_key>text</arg_key>
<arg_value>A long text to be summarized...</arg_value>
</tool_call>"#.to_string()
}

fn qwen_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_call>
{"name": "function_name", "arguments": {"arg_name": "arg_value"}}
</tool_call>

Each tool call must be a JSON object containing "name" and "arguments" fields, wrapped in <tool_call> XML tags.

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_call>
{"name": "get_weather", "arguments": {"location": "Boston, MA"}}
</tool_call>

Example of multiple tool calls:

<tool_call>
{"name": "search_web", "arguments": {"query": "latest AI news"}}
</tool_call>
<tool_call>
{"name": "summarize_text", "arguments": {"text": "A long text to be summarized..."}}
</tool_call>"#.to_string()
}

fn moonshot_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<|tool_calls_section_begin|><|tool_call_begin|>{function_name}<|tool_call_argument_begin|>{arguments}<|tool_call_end|><|tool_calls_section_end|>

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<|tool_calls_section_begin|><|tool_call_begin|>get_weather<|tool_call_argument_begin|>{"location": "Boston, MA"}<|tool_call_end|><|tool_calls_section_end|>

Example of multiple tool calls:

<|tool_calls_section_begin|><|tool_call_begin|>search_web<|tool_call_argument_begin|>{"query": "latest AI news"}<|tool_call_end|><|tool_call_begin|>summarize_text<|tool_call_argument_begin|>{"text": "A long text to be summarized..."}<|tool_call_end|><|tool_calls_section_end|>"#.to_string()
}

fn json_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_calls>
[
  {
    "name": "function_name",
    "arguments": {"arg_name": "arg_value"}
  }
]
</tool_calls>

A JSON array inside <tool_calls> tags where each object contains:
- "name": The function name (string)
- "arguments": The function arguments (JSON object)

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_calls>
[
  {
    "name": "get_weather",
    "arguments": {"location": "Boston, MA"}
  }
]
</tool_calls>

Example of multiple tool calls:

<tool_calls>
[
  {
    "name": "search_web",
    "arguments": {"query": "latest AI news"}
  },
  {
    "name": "summarize_text",
    "arguments": {"text": "A long text to be summarized..."}
  }
]
</tool_calls>"#
        .to_string()
}

fn build_tool_system_message(
    provider: ModelProvider,
    functions: &[&crate::endpoints::chat::request_types::OpenAiFunction],
) -> Result<String, ChatError> {
    let preamble = build_tools_preamble(functions)?;
    let calling_instructions = match provider {
        ModelProvider::Zai => zai_calling_instructions(),
        ModelProvider::Qwen => qwen_calling_instructions(),
        ModelProvider::MoonshotAI => moonshot_calling_instructions(),
        _ => json_calling_instructions(),
    };

    Ok(format!(
        r###"# Tools

You may call one or more functions to assist with the user query.

{}

{}
"###,
        preamble, calling_instructions
    ))
}

/// Converts a ChatFunctionCall into a full ToolCall with generated ID
fn function_call_to_tool_call(function: ChatFunctionCall) -> ToolCall {
    ToolCall {
        id: format!("call_{}", Uuid::new_v4()),
        tool_type: "function".to_string(),
        function,
        index: None,
    }
}

/// Try parsing JSON tool calls from a <tool_calls> XML tag
fn try_parse_json_tool_call(content: &str) -> Option<Vec<ToolCall>> {
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

fn tools_system_message(
    tools: &[OpenAiTool],
    provider: ModelProvider,
) -> Result<ChatMessage, ChatError> {
    let functions = tools
        .iter()
        .map(|tool| {
            let OpenAiTool::Function(function) = tool;
            function
        })
        .collect::<Vec<_>>();

    let system_message = build_tool_system_message(provider, &functions)?;

    Ok(ChatMessage::system(system_message))
}

fn try_parse_xml_tool_call(content: &str) -> Option<Vec<ToolCall>> {
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
fn try_parse_moonshot_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    if !content.contains("<|tool_calls_section_begin|>") {
        return None;
    }

    let mut tool_calls = Vec::new();

    for cap in MOONSHOT_TOOL_CALL_REGEX.captures_iter(content) {
        let inner = cap.get(1)?.as_str();

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

// This function is now correctly covered by the refactored tools_system_message above

fn format_tool_calls(
    tool_calls: &[ToolCall],
    provider: ModelProvider,
) -> Result<String, ChatError> {
    match provider {
        ModelProvider::MoonshotAI => {
            let mut formatted = String::from("<|tool_calls_section_begin|>");
            for tool_call in tool_calls {
                let args = match tool_call.function.arguments.as_str() {
                    Some(s) => s.to_string(),
                    None => serde_json::to_string(&tool_call.function.arguments)?,
                };

                let name = if tool_call.function.name.is_empty() {
                    &tool_call.id
                } else {
                    &tool_call.function.name
                };

                formatted.push_str(&format!(
                    "<|tool_call_begin|>{}<|tool_call_argument_begin|>{}<|tool_call_end|>",
                    name, args
                ));
            }
            formatted.push_str("<|tool_calls_section_end|>");
            Ok(formatted)
        }
        ModelProvider::Qwen => {
            let mut formatted = String::new();
            for tool_call in tool_calls {
                let name = if tool_call.function.name.is_empty() {
                    &tool_call.id
                } else {
                    &tool_call.function.name
                };

                let call_obj = serde_json::json!({
                    "name": name,
                    "arguments": tool_call.function.arguments
                });
                formatted.push_str(&format!(
                    "<tool_call>\n{}\n</tool_call>\n",
                    serde_json::to_string(&call_obj)?
                ));
            }
            Ok(formatted.trim().to_string())
        }
        ModelProvider::Zai => {
            let mut formatted = String::new();
            for tool_call in tool_calls {
                let name = if tool_call.function.name.is_empty() {
                    &tool_call.id
                } else {
                    &tool_call.function.name
                };

                formatted.push_str(&format!("<tool_call>{}\n", name));
                if let Some(obj) = tool_call.function.arguments.as_object() {
                    for (k, v) in obj {
                        let val_str = if v.is_string() {
                            v.as_str().unwrap().to_string()
                        } else {
                            v.to_string()
                        };
                        formatted.push_str(&format!(
                            "<arg_key>{}</arg_key>\n<arg_value>{}</arg_value>\n",
                            k, val_str
                        ));
                    }
                }
                formatted.push_str("</tool_call>\n");
            }
            Ok(formatted.trim().to_string())
        }
        _ => {
            let simplified: Vec<_> = tool_calls
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "name": if tc.function.name.is_empty() { &tc.id } else { &tc.function.name },
                        "arguments": tc.function.arguments
                    })
                })
                .collect();
            Ok(format!(
                "<tool_calls>\n{}\n</tool_calls>",
                serde_json::to_string_pretty(&simplified)?
            ))
        }
    }
}

pub fn convert_openai_message_with_provider(
    message: OpenAiChatMessage,
    provider: ModelProvider,
) -> Result<ChatMessage, ChatError> {
    Ok(match message {
        OpenAiChatMessage::System { content } => ChatMessage::System { content },
        OpenAiChatMessage::User { content } => ChatMessage::User { content },
        OpenAiChatMessage::Assistant {
            content,
            tool_calls,
        } => {
            let mut final_content = content.map(|c| c.to_string()).unwrap_or_default();

            if let Some(tool_calls) = tool_calls {
                let formatted_tools = format_tool_calls(&tool_calls, provider)?;
                if !final_content.is_empty() {
                    final_content.push_str("\n\n");
                }
                final_content.push_str(&formatted_tools);
            }

            ChatMessage::Assistant {
                content: ChatContent::String(final_content),
            }
        }
        OpenAiChatMessage::Tool { .. } => ChatMessage::User {
            content: ChatContent::String(serde_json::to_string_pretty(&message)?),
        },
    })
}

impl TryFrom<OpenAiChatRequest> for StraicoChatRequest {
    type Error = ChatError;

    fn try_from(mut request: OpenAiChatRequest) -> Result<Self, Self::Error> {
        let provider = ModelProvider::from_model_id(&request.chat_request.model);

        let messages: Vec<ChatMessage> = request
            .chat_request
            .messages
            .into_iter()
            .map(|msg| convert_openai_message_with_provider(msg, provider))
            .collect::<Result<_, _>>()?;

        let mut builder = ChatRequest::builder()
            .model(std::mem::take(&mut request.chat_request.model))
            .max_tokens(request.chat_request.max_tokens)
            .temperature(request.chat_request.temperature)
            .messages(messages);

        if let Some(tools) = request.tools
            && !tools.is_empty()
        {
            builder = builder.message(tools_system_message(&tools, provider)?);
        }

        Ok(builder.build())
    }
}

impl TryFrom<OpenAiChatMessage> for ChatMessage {
    type Error = ChatError;

    fn try_from(message: OpenAiChatMessage) -> Result<Self, Self::Error> {
        // Default to Unknown provider when converting without explicit context
        convert_openai_message_with_provider(message, ModelProvider::Unknown)
    }
}

pub fn convert_message_with_provider(
    message: ChatMessage,
    provider: ModelProvider,
) -> Result<OpenAiChatMessage, ChatError> {
    match message {
        ChatMessage::System { content } => Ok(OpenAiChatMessage::System { content }),
        ChatMessage::User { content } => Ok(OpenAiChatMessage::User { content }),
        ChatMessage::Assistant { content } => {
            let content_str = content.to_string();

            let final_tool_calls = match provider {
                ModelProvider::Zai => try_parse_xml_tool_call(&content_str)
                    .or_else(|| try_parse_json_tool_call(&content_str))
                    .or_else(|| try_parse_moonshot_tool_call(&content_str)),
                ModelProvider::MoonshotAI => try_parse_moonshot_tool_call(&content_str)
                    .or_else(|| try_parse_json_tool_call(&content_str)),
                ModelProvider::Qwen => try_parse_xml_tool_call(&content_str)
                    .or_else(|| try_parse_json_tool_call(&content_str)),
                ModelProvider::Anthropic
                | ModelProvider::Google
                | ModelProvider::OpenAI
                | ModelProvider::Unknown => try_parse_json_tool_call(&content_str)
                    .or_else(|| try_parse_xml_tool_call(&content_str))
                    .or_else(|| try_parse_moonshot_tool_call(&content_str)),
            };

            if let Some(mut tool_calls) = final_tool_calls
                && !tool_calls.is_empty()
            {
                // Assign indices if they are missing
                for (i, tc) in tool_calls.iter_mut().enumerate() {
                    if tc.index.is_none() {
                        tc.index = Some(i);
                    }
                }

                return Ok(OpenAiChatMessage::Assistant {
                    content: None,
                    tool_calls: Some(tool_calls),
                });
            }

            // If no tool calls are found, return content as is.
            debug!(
                "No tool call identified in assistant message. Content: {}",
                content
            );

            Ok(OpenAiChatMessage::Assistant {
                content: Some(content),
                tool_calls: None,
            })
        }
    }
}

impl TryFrom<ChatMessage> for OpenAiChatMessage {
    type Error = ChatError;

    fn try_from(message: ChatMessage) -> Result<Self, Self::Error> {
        // Default to Unknown provider when converting back without context
        convert_message_with_provider(message, ModelProvider::Unknown)
    }
}

impl TryFrom<StraicoChatResponse> for OpenAiChatResponse {
    type Error = ChatError;

    fn try_from(response: StraicoChatResponse) -> Result<Self, Self::Error> {
        let provider = ModelProvider::from_model_id(&response.response.model);

        let choices = response
            .response
            .choices
            .into_iter()
            .map(|choice| {
                let open_ai_message: OpenAiChatMessage =
                    convert_message_with_provider(choice.message, provider)?;
                let finish_reason = match &open_ai_message {
                    OpenAiChatMessage::Assistant { tool_calls, .. } => {
                        if tool_calls.is_some() {
                            "tool_calls".to_string()
                        } else {
                            choice.finish_reason
                        }
                    }
                    _ => choice.finish_reason,
                };

                Ok(ChatChoice {
                    index: choice.index,
                    message: open_ai_message,
                    finish_reason,
                    logprobs: None,
                })
            })
            .collect::<Result<Vec<ChatChoice<OpenAiChatMessage>>, ChatError>>()?;

        Ok(OpenAiChatResponse {
            id: response.response.id,
            object: response.response.object,
            created: response.response.created,
            model: response.response.model,
            choices,
            usage: response.response.usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoints::chat::{ChatContent, ChatFunctionCall, ToolCall};

    #[test]
    fn test_openai_to_chat_message_assistant_with_tools() {
        let tool_calls = vec![ToolCall {
            id: "tool1".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({}),
            },
            index: None,
        }];
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: None,
            tool_calls: Some(tool_calls.clone()),
        };
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();
                assert!(content_str.contains("<tool_calls>"));
                assert!(content_str.contains("test_func"));
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_qwen_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg": "val"}),
            },
            index: None,
        }];
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: Some(ChatContent::String("Thinking...".to_string())),
            tool_calls: Some(tool_calls),
        };

        let chat_msg =
            convert_openai_message_with_provider(open_ai_msg, ModelProvider::Qwen).unwrap();

        match chat_msg {
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();
                assert!(content_str.contains("Thinking..."));
                assert!(content_str.contains("<tool_call>"));
                assert!(content_str.contains("\"name\":\"test_func\""));
                assert!(content_str.contains("\"arguments\":{\"arg\":\"val\"}"));
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_zai_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_456".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg1": "val1", "arg2": 2}),
            },
            index: None,
        }];
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: None,
            tool_calls: Some(tool_calls),
        };

        let chat_msg =
            convert_openai_message_with_provider(open_ai_msg, ModelProvider::Zai).unwrap();

        match chat_msg {
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();
                assert!(content_str.contains("<tool_call>test_func"));
                assert!(content_str.contains("<arg_key>arg1</arg_key>"));
                assert!(content_str.contains("<arg_value>val1</arg_value>"));
                assert!(content_str.contains("<arg_key>arg2</arg_key>"));
                assert!(content_str.contains("<arg_value>2</arg_value>"));
                assert!(content_str.contains("</tool_call>"));
            }
            _ => panic!("Incorrect message type"),
        }
    }

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
    fn test_openai_to_chat_message_tool() {
        let open_ai_msg = OpenAiChatMessage::Tool {
            content: ChatContent::String("Tool output".to_string()),
            tool_call_id: "tool1".to_string(),
        };
        let chat_msg: ChatMessage = open_ai_msg.clone().try_into().unwrap();
        match chat_msg {
            ChatMessage::User { content } => {
                let expected_str = serde_json::to_string_pretty(&open_ai_msg).unwrap();
                assert_eq!(content.to_string(), expected_str);
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_tools() {
        // Test the simplified format (just name and arguments)
        let tool_calls_json =
            r#"[{"name":"view","arguments": { "file_path":"client/Cargo.toml" }}]"#;
        let content_str = format!("<tool_calls>\n{}\n</tool_calls>", tool_calls_json);
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert!(tool_calls[0].id.starts_with("call_"));
                assert_eq!(tool_calls[0].tool_type, "function");
                assert_eq!(tool_calls[0].function.name, "view");
                assert_eq!(
                    tool_calls[0].function.arguments,
                    serde_json::json!({ "file_path": "client/Cargo.toml" })
                );
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_tools_legacy_format() {
        // Test backwards compatibility with full ToolCall format
        let tool_calls_json = r#"[{"id":"tool_call_0","type":"function","function":{"name":"view","arguments": { "file_path":"client/Cargo.toml" }}}]"#;
        let content_str = format!("<tool_calls>\n{}\n</tool_calls>", tool_calls_json);
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].id, "tool_call_0");
                assert_eq!(tool_calls[0].tool_type, "function");
                assert_eq!(tool_calls[0].function.name, "view");
                assert_eq!(
                    tool_calls[0].function.arguments,
                    serde_json::json!({ "file_path": "client/Cargo.toml" })
                );
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_malformed_tools() {
        let content_str = "<tool_calls>\nmalformed json\n</tool_calls>";
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        // This should not error, but result in a message with content and no tool_calls
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content.unwrap().to_string(), content_str);
                assert!(tool_calls.is_none());
            }
            _ => panic!("Incorrect message type"),
        }
    }
    #[test]
    fn test_openai_to_chat_message_assistant_with_nested_backticks() {
        // This simulates a tool call where the argument contains a markdown code block
        // The current regex `(?s)```json\s*(.*?)``` ` is non-greedy and will stop at the first ```
        let arguments = serde_json::json!({
            "file_path": "README.md",
            "content": "# Title\n\n```bash\ncargo build\n```\n"
        });

        // We need to construct the tool call structure manually to match what the LLM sends
        // The LLM sends a JSON array of tool calls (simplified format)
        let tool_calls = vec![serde_json::json!({
            "name": "write",
            "arguments": arguments // arguments is a JSON object now
        })];

        let tool_calls_json = serde_json::to_string_pretty(&tool_calls).unwrap();
        let content_str = format!("<tool_calls>\n{}\n</tool_calls>", tool_calls_json);

        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str),
        };

        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(
                    content.is_none(),
                    "Content should be None when tool calls are parsed"
                );
                let tool_calls = tool_calls.expect("Tool calls should be parsed");
                assert_eq!(tool_calls.len(), 1);
                assert!(tool_calls[0].id.starts_with("call_"));
                assert_eq!(tool_calls[0].function.name, "write");
                // Verify the argument contains the nested backticks
                let args_str = tool_calls[0].function.arguments.to_string();
                assert!(args_str.contains("```bash"));
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_request_conversion() {
        let request = OpenAiChatRequest {
            chat_request: ChatRequest {
                model: "gpt-4".to_string(),
                messages: vec![OpenAiChatMessage::User {
                    content: ChatContent::String("Hello".to_string()),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
            },
            stream: false,
            tools: None,
            tool_choice: None,
        };

        let straico_request: StraicoChatRequest = request.try_into().unwrap();
        assert_eq!(straico_request.model, "gpt-4");
        assert_eq!(straico_request.temperature, Some(0.7));
        assert_eq!(straico_request.max_tokens, Some(100));
        assert_eq!(straico_request.messages.len(), 1);
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_xml_tools() {
        let content_str = r#"<tool_calls>
[
  {"name": "test_func", "arguments": {"arg": "val"}}
]
</tool_calls>"#;
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg = OpenAiChatMessage::try_from(chat_msg).unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant { tool_calls, .. } => {
                let tool_calls = tool_calls.expect("Should have parsed tool calls");
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].function.name, "test_func");
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_chatml_tools() {
        let content_str = "<|im_start|>tool\nsome tool call\n<|im_end|>";
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        // Should not panic anymore, just return content
        let _ = OpenAiChatMessage::try_from(chat_msg).unwrap();
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_custom_xml_tools() {
        let content_str = r#"<tool_call>read
<arg_key>filePath</arg_key>
<arg_value>/tmp/test_file.txt</arg_value>
</tool_call>"#;
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert!(tool_calls[0].id.starts_with("call_"));
                assert_eq!(tool_calls[0].tool_type, "function");
                assert_eq!(tool_calls[0].function.name, "read");
                assert_eq!(
                    tool_calls[0].function.arguments["filePath"],
                    "/tmp/test_file.txt"
                );
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_complex_xml_tools() {
        let content_str = r#"<tool_call>write
<arg_key>content</arg_key>
<arg_value>Line 1
"Quoted Text"
Line 2</arg_value>
</tool_call>"#;
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant { tool_calls, .. } => {
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].function.name, "write");
                assert_eq!(
                    tool_calls[0].function.arguments["content"],
                    "Line 1\n\"Quoted Text\"\nLine 2"
                );
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_moonshot_tools() {
        let content_str = r#"<|tool_calls_section_begin|><|tool_call_begin|>functions.view:0<|tool_call_argument_begin|>{"file_path": "/tmp/random_file.txt"}<|tool_call_end|><|tool_calls_section_end|>"#;
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert!(tool_calls[0].id.starts_with("call_"));
                assert_eq!(tool_calls[0].tool_type, "function");
                assert_eq!(tool_calls[0].function.name, "view");
                assert_eq!(
                    tool_calls[0].function.arguments["file_path"],
                    "/tmp/random_file.txt"
                );
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_moonshot_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_12345".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg": "val"}),
            },
            index: None,
        }];

        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: None,
            tool_calls: Some(tool_calls),
        };

        // We explicitly use MoonshotAI provider
        let result =
            convert_openai_message_with_provider(open_ai_msg, ModelProvider::MoonshotAI).unwrap();

        match result {
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();
                // Expectation: function name "test_func" should be used, not the ID "call_12345"
                // The format is: <|tool_call_begin|>FUNCTION_NAME<|tool_call_argument_begin|>ARGUMENTS<|tool_call_end|>
                let expected_part = "<|tool_call_begin|>test_func<|tool_call_argument_begin|>";

                assert!(
                    content_str.contains(expected_part),
                    "Content did not contain expected part '{}'. Actual content: '{}'",
                    expected_part,
                    content_str
                );
                assert!(content_str.contains("{\"arg\":\"val\"}"));
            }
            _ => panic!("Incorrect message type"),
        }
    }
}

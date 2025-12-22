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

static JSON_TOOL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```json\s*(.*?)\]\s*```").unwrap());

static XML_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<tool_call>(.*?)</tool_call>").unwrap());

static XML_ARG_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<arg_key>(.*?)</arg_value>").unwrap());

static PIPE_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<\|tool_call_begin\|>(.*?)<\|tool_call_end\|>").unwrap());

/// Unified template macro for generating tool system messages across different formats.
/// This macro consolidates common instruction text while allowing format-specific customization.
///
/// # Parameters
/// - `$function_signatures` - The formatted function signatures (JSON array, XML list, or Qwen XML)
/// - `$wrapper_syntax` - The exact wrapper syntax (e.g., "```json ... ```" or "<tool_call>...</tool_call>")
/// - `$call_structure` - Description of the structure within the wrapper
/// - `$single_example` - Complete example of a single tool call
/// - `$multiple_example` - Complete example of multiple tool calls
macro_rules! unified_tool_system_message {
    (
        $function_signatures:expr,
        $wrapper_syntax:expr,
        $call_structure:expr,
        $single_example:expr,
        $multiple_example:expr
    ) => {
        format!(
            r###"
# Tools

You may call one or more functions to assist with the user query.

{}

# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

{}

{}

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

The following examples demonstrate the REQUIRED format. Your responses must match this structure exactly.

Example of a single tool call:

{}

Example of multiple tool calls:

{}
"###,
            $function_signatures,
            $wrapper_syntax,
            $call_structure,
            $single_example,
            $multiple_example
        )
    };
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

/// Try parsing JSON tool calls from a ```json code block
fn try_parse_json_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    // Look for ```json ... ]\n```
    // The extra "]" ensures we don't match on backtick blocks that are not tool calls.
    let raw_json = JSON_TOOL_REGEX
        .captures(content)
        .and_then(|c| c.get(1))
        // We need to re-introduce the square bracket since the regex excludes it
        .map(|m| format!("{}]", m.as_str().trim()))
        // Try parsing JSON tool calls from a raw JSON array without code block wrapper
        .or_else(|| {
            let trimmed = content.trim();
            (trimmed.starts_with('[') && trimmed.ends_with(']')).then_some(trimmed.to_string())
        })?;

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

/// Generates JSON tool system message
fn json_tools_message(tools: &[OpenAiTool]) -> Result<ChatMessage, ChatError> {
    // This removes the wrapper that only adds the "type: function"
    let functions = tools
        .iter()
        .map(|tool| {
            let OpenAiTool::Function(function) = tool;
            function
        })
        .collect::<Vec<_>>();

    let function_signatures = format!(
        "You are provided with available function signatures within the following JSON array:\n\n```json\n{}\n```",
        serde_json::to_string_pretty(&functions)?
    );

    let wrapper_syntax = "```json\n[...]\n```";

    let call_structure = "A JSON array where each object contains:\n- \"name\": The function name (string)\n- \"arguments\": The function arguments (JSON object)";

    let single_example = r#"```json
[
  {
    "name": "get_weather",
    "arguments": {"location": "Boston, MA"}
  }
]
```"#;

    let multiple_example = r#"```json
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
```"#;

    let system_message = unified_tool_system_message!(
        function_signatures,
        wrapper_syntax,
        call_structure,
        single_example,
        multiple_example
    );

    Ok(ChatMessage::system(system_message))
}

/// Generates XML tool system message
fn xml_tools_message(tools: &[OpenAiTool]) -> Result<ChatMessage, ChatError> {
    let functions = tools
        .iter()
        .map(|tool| {
            let OpenAiTool::Function(function) = tool;
            function
        })
        .collect::<Vec<_>>();

    let function_signatures = {
        let formatted = functions
            .iter()
            .map(|f| {
                let params = serde_json::to_string_pretty(&f.parameters).unwrap_or_default();
                format!(
                    "- Function: {}\n  Description: {}\n  Parameters: {}",
                    f.name,
                    f.description.as_deref().unwrap_or(""),
                    params
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("Available functions:\n{}", formatted)
    };

    let wrapper_syntax = "<tool_call>{function_name}\n<arg_key>{parameter_name}</arg_key>\n<arg_value>{parameter_value}</arg_value>\n</tool_call>";

    let call_structure = "Each tool call must be wrapped in <tool_call> tags with the function name immediately after the opening tag. Parameters are specified using <arg_key> and <arg_value> pairs.";

    let single_example = r#"<tool_call>get_weather
<arg_key>location</arg_key>
<arg_value>Boston, MA</arg_value>
</tool_call>"#;

    let multiple_example = r#"<tool_call>search_web
<arg_key>query</arg_key>
<arg_value>latest AI news</arg_value>
</tool_call>
<tool_call>summarize_text
<arg_key>text</arg_key>
<arg_value>A long text to be summarized...</arg_value>
</tool_call>"#;

    let system_message = unified_tool_system_message!(
        function_signatures,
        wrapper_syntax,
        call_structure,
        single_example,
        multiple_example
    );

    Ok(ChatMessage::system(system_message))
}

/// Generates Qwen tool system message
fn qwen_tools_message(tools: &[OpenAiTool]) -> Result<ChatMessage, ChatError> {
    let functions = tools
        .iter()
        .map(|tool| {
            let OpenAiTool::Function(function) = tool;
            function
        })
        .collect::<Vec<_>>();

    let function_signatures = {
        let json_functions = functions
            .iter()
            .map(|f| serde_json::to_string(f).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "You are provided with function signatures within <tools></tools> XML tags:\n<tools>\n{}\n</tools>",
            json_functions
        )
    };

    let wrapper_syntax =
        "<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>";

    let call_structure = "Each tool call is a JSON object wrapped in <tool_call> XML tags. The JSON object must contain \"name\" and \"arguments\" fields.";

    let single_example = r#"<tool_call>
{"name": "get_weather", "arguments": {"location": "Boston, MA"}}
</tool_call>"#;

    let multiple_example = r#"<tool_call>
{"name": "search_web", "arguments": {"query": "latest AI news"}}
</tool_call>
<tool_call>
{"name": "summarize_text", "arguments": {"text": "A long text to be summarized..."}}
</tool_call>"#;

    let system_message = unified_tool_system_message!(
        function_signatures,
        wrapper_syntax,
        call_structure,
        single_example,
        multiple_example
    );

    Ok(ChatMessage::system(system_message))
}

/// Helper to try parsing XML tool calls
fn try_parse_xml_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    let mut tool_calls = Vec::new();

    for cap in XML_TOOL_CALL_REGEX.captures_iter(content) {
        let inner = cap.get(1)?.as_str().trim();

        // Extract function name (first line/word)
        let mut lines = inner.lines();
        let function_name = lines.next()?.trim().to_string();

        if function_name.is_empty() {
            continue;
        }

        // Build JSON arguments
        let mut args_json_str = String::from("{");
        let mut first = true;

        for arg_cap in XML_ARG_REGEX.captures_iter(inner) {
            if !first {
                args_json_str.push(',');
            }
            let arg_content = arg_cap.get(1)?.as_str();
            // Prepend quote to make it valid JSON key
            args_json_str.push_str(&format!("\"{}", arg_content));
            first = false;
        }
        args_json_str.push('}');

        // Validate and parse JSON
        if let Ok(args_value) = serde_json::from_str::<serde_json::Value>(&args_json_str) {
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

/// Helper to try parsing pipe tool calls
fn try_parse_pipe_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    if !content.contains("<|tool_calls_section_begin|>") {
        return None;
    }

    let mut tool_calls = Vec::new();

    for cap in PIPE_TOOL_CALL_REGEX.captures_iter(content) {
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

fn try_parse_qwen_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    let mut tool_calls = Vec::new();

    for cap in XML_TOOL_CALL_REGEX.captures_iter(content) {
        let inner = cap.get(1)?.as_str().trim();

        // Qwen puts the whole JSON object inside <tool_call>
        if let Ok(call_obj) = serde_json::from_str::<serde_json::Value>(inner) {
            if let (Some(name), Some(args)) = (
                call_obj.get("name").and_then(|n| n.as_str()),
                call_obj.get("arguments"),
            ) {
                tool_calls.push(function_call_to_tool_call(ChatFunctionCall {
                    name: name.to_string(),
                    arguments: args.clone(),
                }));
            }
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

fn tools_system_message(
    tools: &[OpenAiTool],
    provider: ModelProvider,
) -> Result<ChatMessage, ChatError> {
    match provider {
        ModelProvider::Zai => xml_tools_message(tools),
        ModelProvider::Qwen => qwen_tools_message(tools),
        ModelProvider::Kimi => json_tools_message(tools), // Fallback to JSON for Kimi as no system prompt provided
        ModelProvider::Anthropic | ModelProvider::OpenAI | ModelProvider::Unknown => {
            json_tools_message(tools)
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
        } => ChatMessage::Assistant {
            content: if let Some(tool_calls) = tool_calls {
                match provider {
                    ModelProvider::Kimi => {
                        // Kimi specific formatting
                        let mut formatted = String::from("<|tool_calls_section_begin|>");
                        for tool_call in &tool_calls {
                            let args = if tool_call.function.arguments.is_string() {
                                tool_call.function.arguments.as_str().unwrap().to_string()
                            } else {
                                serde_json::to_string(&tool_call.function.arguments)?
                            };

                            formatted.push_str(&format!(
                                "<|tool_call_begin|>{}<|tool_call_argument_begin|>{}<|tool_call_end|>",
                                tool_call.id, args
                            ));
                        }
                        formatted.push_str("<|tool_calls_section_end|>");
                        ChatContent::String(formatted)
                    }
                    _ => ChatContent::String(serde_json::to_string_pretty(&tool_calls)?),
                }
            } else {
                content.unwrap_or(ChatContent::String("".to_string()))
            },
        },
        OpenAiChatMessage::Tool { .. } => ChatMessage::User {
            content: ChatContent::String(serde_json::to_string_pretty(&message)?),
        },
    })
}

impl TryFrom<OpenAiChatRequest> for StraicoChatRequest {
    type Error = ChatError;

    fn try_from(request: OpenAiChatRequest) -> Result<Self, Self::Error> {
        let provider = ModelProvider::from_model_id(&request.chat_request.model);

        let mut messages: Vec<ChatMessage> = request
            .chat_request
            .messages
            .into_iter()
            .map(|msg| convert_openai_message_with_provider(msg, provider))
            .collect::<Result<_, _>>()?;

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                messages.insert(0, tools_system_message(tools, provider)?);
            }
        }

        Ok(ChatRequest::builder()
            .model(&request.chat_request.model)
            .max_tokens(request.chat_request.max_tokens)
            .temperature(request.chat_request.temperature)
            .messages(messages)
            .build())
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
                    .or_else(|| try_parse_pipe_tool_call(&content_str)),
                ModelProvider::Kimi => try_parse_pipe_tool_call(&content_str)
                    .or_else(|| try_parse_json_tool_call(&content_str)),
                ModelProvider::Qwen => try_parse_qwen_tool_call(&content_str)
                    .or_else(|| try_parse_json_tool_call(&content_str)),
                ModelProvider::Anthropic | ModelProvider::OpenAI | ModelProvider::Unknown => {
                    try_parse_json_tool_call(&content_str)
                        .or_else(|| try_parse_xml_tool_call(&content_str))
                        .or_else(|| try_parse_pipe_tool_call(&content_str))
                }
            };

            if let Some(mut tool_calls) = final_tool_calls {
                if !tool_calls.is_empty() {
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
                let expected_json = serde_json::to_string_pretty(&tool_calls).unwrap();
                assert_eq!(content.to_string(), expected_json);
            }
            _ => panic!("Incorrect message type"),
        }
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
        let content_str = format!("```json\n{}\n```", tool_calls_json);
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
        let content_str = format!("```json\n{}\n```", tool_calls_json);
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
        let content_str = "```json\nmalformed json\n```";
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
        let content_str = format!("```json\n{}\n```", tool_calls_json);

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
        let content_str = "<tool_calls>some tool call</tool_calls>";
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        // Should not panic anymore, just return content
        let _ = OpenAiChatMessage::try_from(chat_msg).unwrap();
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
<arg_key>filePath": "/tmp/test_file.txt"</arg_value>
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
    fn test_openai_to_chat_message_assistant_with_pipe_tools() {
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
}

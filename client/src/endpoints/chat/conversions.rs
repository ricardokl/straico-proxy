use super::{
    ChatContent, ChatError, ChatMessage, OpenAiChatMessage, ToolCall,
    request_types::{ChatRequest, OpenAiChatRequest, OpenAiTool, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};
use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;

static JSON_TOOL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```json\s*(.*?)\]\s*```").unwrap());

static XML_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<tool_call>(.*?)</tool_call>").unwrap());

static XML_ARG_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<arg_key>(.*?)</arg_value>").unwrap());

static PIPE_TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<\|tool_call_begin\|>(.*?)<\|tool_call_end\|>").unwrap());

/// Helper to try parsing JSON tool calls
fn try_parse_json_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    // Look for ```json ... ]\n```
    // The extra "]" ensures we don't match on backtick blocks that are not tool calls.
    if let Some(captures) = JSON_TOOL_REGEX.captures(content) {
        if let Some(match_) = captures.get(1) {
            // We need to re-introduce the square bracket
            // since the regex excludes it
            let raw_json = format!("{}]", match_.as_str().trim());

            // Remove extra "function," lines that some models add
            let cleaned_lines = raw_json
                .lines()
                .filter(|line| line.trim() != "\"function\",")
                .collect::<Vec<_>>()
                .join("\n");

            if let Ok(calls) = serde_json::from_str::<Vec<ToolCall>>(&cleaned_lines) {
                return Some(calls);
            }
        }
    }

    // Fallback: try parsing raw JSON array without code block wrapper
    let trimmed = content.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let cleaned_lines = trimmed
            .lines()
            .filter(|line| line.trim() != "\"function\",")
            .collect::<Vec<_>>()
            .join("\n");

        if let Ok(calls) = serde_json::from_str::<Vec<ToolCall>>(&cleaned_lines) {
            return Some(calls);
        }
    }

    None
}

/// Helper to try parsing XML tool calls
fn try_parse_xml_tool_call(content: &str) -> Option<Vec<ToolCall>> {
    let mut tool_calls = Vec::new();

    for (i, cap) in XML_TOOL_CALL_REGEX.captures_iter(content).enumerate() {
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
            tool_calls.push(ToolCall {
                id: format!("call_{}", i),
                tool_type: "function".to_string(),
                function: crate::endpoints::chat::ChatFunctionCall {
                    name: function_name,
                    arguments: args_value,
                },
                index: None,
            });
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

    for (i, cap) in PIPE_TOOL_CALL_REGEX.captures_iter(content).enumerate() {
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
            tool_calls.push(ToolCall {
                id: format!("call_{}", i),
                tool_type: "function".to_string(),
                function: crate::endpoints::chat::ChatFunctionCall {
                    name: function_name,
                    arguments: args_value,
                },
                index: None,
            });
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Generates tool system message for embedding in messages.
fn tools_system_message(tools: &[OpenAiTool]) -> Result<ChatMessage, ChatError> {
    // This removes the wrapper that only adds the "type: function"
    let functions = tools
        .iter()
        .map(|tool| {
            let OpenAiTool::Function(function) = tool;
            function
        })
        .collect::<Vec<_>>();

    let system_message = format!(
        r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within the following JSON array:

```json
{}
```

# Tool Calls

When you need to call one or more tools, you must respond with a single JSON code block containing an array of tool call objects.
The JSON array must be enclosed in a ```json fenced code block.

Each object in the array represents a single tool call and must have the following properties:
- "id": A unique identifier for the tool call, e.g., "tool_call_0".
- "type": The type of the tool, which must be "function".
- "function": An object containing the function name and its arguments.

The "function" object must have the following properties:
- "name": The name of the function to call.
- "arguments": A JSON string of the arguments to pass to the function.

Example of a single tool call:

```json
[
  {{
    "id": "tool_call_0",
    "type": "function",
    "function": {{
      "name": "get_weather",
      "arguments": "{{"location": "Boston, MA"}}"
    }}
  }}
]
```

Example of multiple tool calls:

```json
[
  {{
    "id": "tool_call_0",
    "type": "function",
    "function": {{
      "name": "search_web",
      "arguments": "{{"query": "latest AI news"}}"
    }}
  }},
  {{
    "id": "tool_call_1",
    "type": "function",
    "function": {{
      "name": "summarize_text",
      "arguments": "{{"text": "A long text to be summarized..."}}"
    }}
  }}
]
```
"###,
        serde_json::to_string_pretty(&functions)?,
    );

    Ok(ChatMessage::system(system_message))
}

impl TryFrom<OpenAiChatRequest> for StraicoChatRequest {
    type Error = ChatError;

    fn try_from(request: OpenAiChatRequest) -> Result<Self, Self::Error> {
        let mut messages: Vec<ChatMessage> = request
            .chat_request
            .messages
            .into_iter()
            .map(ChatMessage::try_from)
            .collect::<Result<_, _>>()?;

        match request.tools {
            Some(tools) if !tools.is_empty() => {
                messages.insert(0, tools_system_message(&tools)?);
            }
            _ => {}
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
        Ok(match message {
            OpenAiChatMessage::System { content } => ChatMessage::System { content },
            OpenAiChatMessage::User { content } => ChatMessage::User { content },
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => ChatMessage::Assistant {
                content: if let Some(tool_calls) = tool_calls {
                    ChatContent::String(serde_json::to_string_pretty(&tool_calls)?)
                } else {
                    content.unwrap_or(ChatContent::String("".to_string()))
                },
            },
            OpenAiChatMessage::Tool { .. } => ChatMessage::User {
                content: ChatContent::String(serde_json::to_string_pretty(&message)?),
            },
        })
    }
}

impl TryFrom<ChatMessage> for OpenAiChatMessage {
    type Error = ChatError;

    fn try_from(message: ChatMessage) -> Result<Self, Self::Error> {
        match message {
            ChatMessage::System { content } => Ok(OpenAiChatMessage::System { content }),
            ChatMessage::User { content } => Ok(OpenAiChatMessage::User { content }),
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();

                let final_tool_calls = try_parse_json_tool_call(&content_str)
                    .or_else(|| try_parse_xml_tool_call(&content_str))
                    .or_else(|| try_parse_pipe_tool_call(&content_str));

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
}

impl TryFrom<StraicoChatResponse> for OpenAiChatResponse {
    type Error = ChatError;

    fn try_from(response: StraicoChatResponse) -> Result<Self, Self::Error> {
        let choices = response
            .response
            .choices
            .into_iter()
            .map(|choice| {
                let open_ai_message: OpenAiChatMessage = choice.message.try_into()?;
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
        // The LLM sends a JSON array of tool calls
        let tool_calls = vec![serde_json::json!({
            "id": "tool_call_0",
            "type": "function",
            "function": {
                "name": "write",
                "arguments": arguments.to_string() // arguments is a JSON string
            }
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
                assert_eq!(tool_calls[0].id, "tool_call_0");
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

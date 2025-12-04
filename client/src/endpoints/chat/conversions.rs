use super::{
    ChatContent, ChatError, ChatMessage, OpenAiChatMessage, ToolCall,
    request_types::{ChatRequest, OpenAiChatRequest, OpenAiTool, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};
use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;

static EXCESS_SLASH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\\{2,}([{}"\[\]\n])"#).unwrap());

/// Cleans over-escaped strings in tool call arguments
fn clean_tool_call_arguments(arguments: &str) -> String {
    // Fix regex literal over-escaping
    let cleaned = REGEX_LITERAL_SLASH_REGEX.replace_all(arguments, r#"\\"#);
    cleaned.to_string()
}

static REGEX_LITERAL_SLASH_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\\\{3,}"#).unwrap());

/// Generates tool XML for embedding in messages.
fn generate_tool_xml(tools: &[OpenAiTool], _model: &str) -> Result<String, ChatError> {
    let pre_tools = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;

    let post_tools = r###"
</tools>
# Tool Calls

When you need to call one or more tools, you must respond with a single JSON code block containing an array of tool call objects.
The JSON array must be enclosed in a ```json fenced code block.

Each object in the array represents a single tool call and must have the following properties:
- "id": A unique identifier for the tool call, e.g., "tool_call_0".
- "type": The type of the tool, which must be "function".
- "function": An object containing the function name and its arguments.

The "function" object must have the following properties:
- "name": The name of the function to call.
- "arguments": A JSON string of the arguments to pass to the function. Note that this must be a STRING containing a valid JSON object, not a nested JSON object.

Example of a single tool call:
```json
[
  {
    "id": "tool_call_0",
    "type": "function",
    "function": {
      "name": "get_weather",
      "arguments": "{\"location\": \"Boston, MA\"}"
    }
  }
]
```

Example of multiple tool calls:
```json
[
  {
    "id": "tool_call_0",
    "type": "function",
    "function": {
      "name": "search_web",
      "arguments": "{\"query\": \"latest AI news\"}"
    }
  },
  {
    "id": "tool_call_1",
    "type": "function",
    "function": {
      "name": "summarize_text",
      "arguments": "{\"text\": \"A long text to be summarized...\"}"
    }
  }
]
```
"###;

    let mut tools_message = String::new();
    tools_message.push_str(pre_tools);
    for tool in tools {
        let OpenAiTool::Function(function) = tool;
        tools_message.push_str(&serde_json::to_string_pretty(&function)?);
    }
    tools_message.push_str(post_tools);

    Ok(tools_message)
}

impl TryFrom<OpenAiChatRequest> for StraicoChatRequest {
    type Error = ChatError;

    fn try_from(request: OpenAiChatRequest) -> Result<Self, Self::Error> {
        let messages: Vec<ChatMessage> = request
            .chat_request
            .messages
            .into_iter()
            .map(ChatMessage::try_from)
            .collect::<Result<_, _>>()?;

        if let Some(tools) = request.tools {
            if !tools.is_empty() {
                let mut new_messages = messages;
                for tool in &tools {
                    let OpenAiTool::Function(_) = tool;
                }

                let tool_xml = generate_tool_xml(&tools, &request.chat_request.model)?;
                let system_message = ChatMessage::system(tool_xml);
                new_messages.insert(0, system_message);

                let mut builder = ChatRequest::builder()
                    .model(&request.chat_request.model)
                    .messages(new_messages);

                let max_tokens = request
                    .chat_request
                    .max_tokens
                    .or(request.max_completion_tokens);
                if let Some(tokens) = max_tokens {
                    builder = builder.max_tokens(tokens);
                }

                if let Some(temp) = request.chat_request.temperature {
                    builder = builder.temperature(temp);
                }

                return Ok(builder.build());
            }
        }

        let mut builder = ChatRequest::builder()
            .model(&request.chat_request.model)
            .messages(messages);

        let max_tokens = request
            .chat_request
            .max_tokens
            .or(request.max_completion_tokens);
        if let Some(tokens) = max_tokens {
            builder = builder.max_tokens(tokens);
        }

        if let Some(temp) = request.chat_request.temperature {
            builder = builder.temperature(temp);
        }

        Ok(builder.build())
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
            } => {
                if let Some(tool_calls) = tool_calls {
                    let tool_calls_str = serde_json::to_string_pretty(&tool_calls)?;
                    let new_content = format!("```json\n{}\n```", tool_calls_str);
                    ChatMessage::Assistant {
                        content: ChatContent::String(new_content),
                    }
                } else {
                    ChatMessage::Assistant {
                        content: content.unwrap_or(ChatContent::String("".to_string())),
                    }
                }
            }
            OpenAiChatMessage::Tool {
                content,
                tool_call_id,
            } => {
                let tool_output = format!(
                    "<tool_output tool_call_id=\"{}\">{}</tool_output>",
                    tool_call_id, content
                );
                ChatMessage::User {
                    content: ChatContent::String(tool_output),
                }
            }
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

                // Helper to try parsing and cleaning JSON
                let try_parse = |json_str: &str| -> Option<Vec<ToolCall>> {
                    let cleaned_lines = json_str
                        .lines()
                        .filter(|line| line.trim() != "\"function\",")
                        .collect::<Vec<_>>()
                        .join("\n");

                    if let Ok(calls) = serde_json::from_str::<Vec<ToolCall>>(&cleaned_lines) {
                        return Some(calls);
                    }

                    // Try fixing excess escaping
                    let cleaned_escapes = EXCESS_SLASH_REGEX.replace_all(&cleaned_lines, r#"\$1"#);
                    if let Ok(mut calls) = serde_json::from_str::<Vec<ToolCall>>(&cleaned_escapes) {
                        // Clean arguments
                        for tc in &mut calls {
                            tc.function.arguments =
                                clean_tool_call_arguments(&tc.function.arguments);
                        }
                        return Some(calls);
                    }

                    None
                };

                let mut final_tool_calls = None;

                // Strategy 1: Primary Heuristic
                // Look for ```json ... ]\n```
                let primary_regex = Regex::new(r"(?s)```json\s*(.*?)\]\n```").unwrap();
                if let Some(captures) = primary_regex.captures(&content_str) {
                    if let Some(match_) = captures.get(1) {
                        let raw_json = format!("{}]", match_.as_str().trim());
                        if let Some(calls) = try_parse(&raw_json) {
                            final_tool_calls = Some(calls);
                        }
                    }
                }

                // Strategy 2: Fallback
                if final_tool_calls.is_none() {
                    if let Some(start_match) = content_str.find("```json") {
                        let start_content_idx = start_match + 7; // length of ```json
                        let content_after_start = &content_str[start_content_idx..];

                        // Find all occurrences of ]\s*```
                        let end_regex = Regex::new(r"\]\s*```").unwrap();
                        for match_ in end_regex.find_iter(content_after_start) {
                            let end_idx = match_.start() + 1;
                            let raw_json = &content_after_start[..end_idx];

                            if let Some(calls) = try_parse(raw_json) {
                                final_tool_calls = Some(calls);
                                break;
                            }
                        }
                    }
                }

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
                let original_content = match &choice.message {
                    ChatMessage::Assistant { content } => content.to_string(),
                    ChatMessage::System { content } => content.to_string(),
                    ChatMessage::User { content } => content.to_string(),
                };
                let open_ai_message: OpenAiChatMessage = choice.message.try_into()?;
                let finish_reason = match &open_ai_message {
                    OpenAiChatMessage::Assistant {
                        tool_calls,
                        content,
                    } => {
                        if tool_calls.is_some() {
                            "tool_calls".to_string()
                        } else {
                            // Log when no tool call was identified from assistant message
                            debug!(
                                "No tool call identified in assistant message. Content: {}",
                                content
                                    .as_ref()
                                    .map(|c| c.to_string())
                                    .unwrap_or_else(|| original_content)
                            );
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
                arguments: "{{}}".to_string(),
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
                let expected_str = format!("```json\n{}\n```", expected_json);
                assert_eq!(content.to_string(), expected_str);
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
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::User { content } => {
                let expected_str = "<tool_output tool_call_id=\"tool1\">Tool output</tool_output>";
                assert_eq!(content.to_string(), expected_str);
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_tools() {
        let tool_calls_json = r#"[{"id":"tool_call_0","type":"function","function":{"name":"view","arguments": " { \"file_path\":\"client/Cargo.toml\" }"}}]"#;
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
                    " { \"file_path\":\"client/Cargo.toml\" }"
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
                assert!(tool_calls[0].function.arguments.contains("```bash"));
            }
            _ => panic!("Incorrect message type"),
        }
    }
}

use super::{
    ChatContent, ChatError, ChatMessage, OpenAiChatMessage, ToolCall,
    request_types::{ChatRequest, OpenAiChatRequest, OpenAiTool, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};
use once_cell::sync::Lazy;
use regex::Regex;

static TOOL_CALLS_JSON_FENCE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```json\s*(.*?)```").unwrap());

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
        tools_message.push_str(&serde_json::to_string_pretty(function)?);
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
                if let Some(captures) = TOOL_CALLS_JSON_FENCE_REGEX.captures(&content_str) {
                    if let Some(tool_calls_str_match) = captures.get(1) {
                        let tool_calls_str = tool_calls_str_match.as_str().trim();
                        if let Ok(mut tool_calls) =
                            serde_json::from_str::<Vec<ToolCall>>(tool_calls_str)
                        {
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
                choice.message.try_into().map(|message| ChatChoice {
                    index: choice.index,
                    message,
                    finish_reason: choice.finish_reason,
                    logprobs: None,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

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
    fn test_openai_to_chat_message_system() {
        let open_ai_msg = OpenAiChatMessage::System {
            content: ChatContent::String("System message".to_string()),
        };
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::System { content } => {
                assert_eq!(content.to_string(), "System message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_user() {
        let open_ai_msg = OpenAiChatMessage::User {
            content: ChatContent::String("User message".to_string()),
        };
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::User { content } => {
                assert_eq!(content.to_string(), "User message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_no_tools() {
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: Some(ChatContent::String("Assistant message".to_string())),
            tool_calls: None,
        };
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::Assistant { content } => {
                assert_eq!(content.to_string(), "Assistant message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_tools() {
        let tool_calls = vec![ToolCall {
            id: "tool1".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: "{}".to_string(),
            },
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
    fn test_chat_to_openai_message_system() {
        let chat_msg = ChatMessage::System {
            content: ChatContent::String("System message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::System { content } => {
                assert_eq!(content.to_string(), "System message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_user() {
        let chat_msg = ChatMessage::User {
            content: ChatContent::String("User message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::User { content } => {
                assert_eq!(content.to_string(), "User message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_no_tools() {
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String("Assistant message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.try_into().unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content.unwrap().to_string(), "Assistant message");
                assert!(tool_calls.is_none());
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_tools() {
        let tool_calls_json = r#"[{"id":"tool_call_0","type":"function","function":{"name":"view","arguments":"{\"file_path\":\"client/Cargo.toml\"}"}}]"#;
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
                    "{\"file_path\":\"client/Cargo.toml\"}"
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
    fn test_chat_to_openai_message_assistant_with_multiple_tools() {
        let tool_calls_json = r#"[
            {
                "id": "tool_call_0",
                "type": "function",
                "function": {
                    "name": "test_func",
                    "arguments": "{}"
                }
            },
            {
                "id": "tool_call_1",
                "type": "function",
                "function": {
                    "name": "test_func2",
                    "arguments": "{\"a\": 1}"
                }
            }
        ]"#;
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
                assert_eq!(tool_calls.len(), 2);

                assert_eq!(tool_calls[0].id, "tool_call_0");
                assert_eq!(tool_calls[0].function.name, "test_func");
                assert_eq!(tool_calls[0].function.arguments, "{}");

                assert_eq!(tool_calls[1].id, "tool_call_1");
                assert_eq!(tool_calls[1].function.name, "test_func2");
                assert_eq!(tool_calls[1].function.arguments, "{\"a\": 1}");
            }
            _ => panic!("Incorrect message type"),
        }
    }
}


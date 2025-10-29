use super::{
    ChatContent, ChatError, ChatFunctionCall, ChatMessage, OpenAiChatMessage, ToolCall,
    request_types::{ChatRequest, OpenAiChatRequest, OpenAiTool, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};
use once_cell::sync::Lazy;
use regex::Regex;

static TOOL_CALLS_WRAPPING_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<tool_calls>(.*)</tool_calls>").unwrap());
static TOOL_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<tool_call>(?s)(.*?)</tool_call>").unwrap());

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

Start with the opening tag <tool_calls>. For each tool call, return a json object with function name and arguments within <tool_call></tool_call> tags:
<tool_call>{\"name\": <function-name>, \"arguments\": <args-json-object>}</tool_call>.
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
                    let tool_calls_str = serde_json::to_string(&tool_calls)?;
                    let new_content = format!("<tool_calls>{}</tool_calls>", tool_calls_str);
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
                if let Some(captures) = TOOL_CALLS_WRAPPING_REGEX.captures(&content_str) {
                    if let Some(tool_calls_str) = captures.get(1) {
                        let mut tool_calls = Vec::new();
                        for (i, cap) in TOOL_CALL_REGEX
                            .captures_iter(tool_calls_str.as_str())
                            .enumerate()
                        {
                            if let Some(call_content) = cap.get(1) {
                                let function: ChatFunctionCall =
                                    serde_json::from_str(call_content.as_str())?;
                                tool_calls.push(ToolCall {
                                    id: format!("tool_call_{}", i),
                                    tool_type: "function".to_string(),
                                    function,
                                });
                            }
                        }

                        if !tool_calls.is_empty() {
                            return Ok(OpenAiChatMessage::Assistant {
                                content: None,
                                tool_calls: Some(tool_calls),
                            });
                        }
                    }
                }
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
    use crate::endpoints::chat::{ChatFunctionCall, ToolCall};

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
            tool_calls: Some(tool_calls),
        };
        let chat_msg: ChatMessage = open_ai_msg.try_into().unwrap();
        match chat_msg {
            ChatMessage::Assistant { content } => {
                let expected_str = "<tool_calls>[{\"id\":\"tool1\",\"type\":\"function\",\"function\":{\"name\":\"test_func\",\"arguments\":\"{}\"}}]</tool_calls>";
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
        let tool_call1 = r#"{"name":"test_func","arguments":"{}"}"#;
        let tool_call2 = r#"{"name":"test_func2","arguments":"{\"a\": 1}"}"#;
        let content_str = format!(
            "<tool_calls><tool_call>{}</tool_call><tool_call>{}</tool_call></tool_calls>",
            tool_call1, tool_call2
        );
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
                assert_eq!(tool_calls[0].tool_type, "function");
                assert_eq!(tool_calls[0].function.name, "test_func");
                assert_eq!(tool_calls[0].function.arguments, "{}");

                assert_eq!(tool_calls[1].id, "tool_call_1");
                assert_eq!(tool_calls[1].tool_type, "function");
                assert_eq!(tool_calls[1].function.name, "test_func2");
                assert_eq!(tool_calls[1].function.arguments, "{\"a\": 1}");
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_malformed_tools() {
        let content_str = "<tool_calls><tool_call>malformed json</tool_call></tool_calls>";
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg_result: Result<OpenAiChatMessage, _> = chat_msg.try_into();
        assert!(open_ai_msg_result.is_err());
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_multiline_tools() {
        let tool_call_str = r#"{
                "name": "test_func",
                "arguments": "{}"
            }"#;
        let content_str = format!(
            "<tool_calls><tool_call>{}</tool_call></tool_calls>",
            tool_call_str
        );
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
                assert_eq!(tool_calls[0].function.name, "test_func");
                assert_eq!(tool_calls[0].function.arguments, "{}");
            }
            _ => panic!("Incorrect message type"),
        }
    }
}

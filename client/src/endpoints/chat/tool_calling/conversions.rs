use super::error::ToolCallingError;

use super::types::{ModelProvider, ToolCall};
use crate::endpoints::chat::common_types::{ChatContent, ChatMessage, OpenAiChatMessage};

pub fn convert_assistant_with_tools_to_straico(
    content: Option<ChatContent>,
    tool_calls: &[ToolCall],
    provider: ModelProvider,
) -> Result<ChatMessage, ToolCallingError> {
    let mut final_content = content
        .map(|c: ChatContent| c.to_string())
        .unwrap_or_default();
    let formatted_tools = provider.format_tool_calls(tool_calls)?;
    if !final_content.is_empty() {
        final_content.push_str("\n\n");
    }
    final_content.push_str(&formatted_tools);

    Ok(ChatMessage::Assistant {
        content: ChatContent::String(final_content),
    })
}

pub fn convert_straico_assistant_to_openai(
    content: ChatContent,
    provider: ModelProvider,
) -> Result<OpenAiChatMessage, ToolCallingError> {
    let content_str = content.to_string();
    let mut tool_calls = provider.parse_tool_calls(&content_str);

    if let Some(ref mut tcs) = tool_calls
        && !tcs.is_empty()
    {
        // Assign indices if they are missing
        for (i, tc) in tcs.iter_mut().enumerate() {
            if tc.index.is_none() {
                tc.index = Some(i);
            }
        }

        return Ok(OpenAiChatMessage::Assistant {
            content: None,
            tool_calls: tool_calls.take(),
        });
    }

    Ok(OpenAiChatMessage::Assistant {
        content: Some(content),
        tool_calls: None,
    })
}

pub fn convert_tool_message_to_straico(
    message: &OpenAiChatMessage,
) -> Result<ChatMessage, ToolCallingError> {
    Ok(ChatMessage::User {
        content: ChatContent::String(serde_json::to_string_pretty(message)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoints::chat::tool_calling::types::ChatFunctionCall;

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
        let chat_msg =
            convert_assistant_with_tools_to_straico(None, &tool_calls, ModelProvider::Unknown)
                .unwrap();
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
    fn test_chat_to_openai_message_assistant_with_tools() {
        let tool_calls_json =
            r#"[{"name":"view","arguments": { "file_path":"client/Cargo.toml" }}]"#;
        let content_str = format!("<tool_calls>\n{}\n</tool_calls>", tool_calls_json);
        let content = ChatContent::String(content_str);

        let open_ai_msg =
            convert_straico_assistant_to_openai(content, ModelProvider::Unknown).unwrap();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let tool_calls = tool_calls.unwrap();
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].function.name, "view");
            }
            _ => panic!("Incorrect message type"),
        }
    }
}

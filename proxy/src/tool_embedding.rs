use crate::openai_types::{OpenAiChatMessage, OpenAiChatRequest, OpenAiContent, OpenAiTool};
use straico_client::chat::ToolCallsFormat;
use straico_client::endpoints::chat::ChatRequest;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolEmbeddingError {
    #[error("No user messages found for tool embedding")]
    NoUserMessages,
    #[error("Invalid tool definition: {0}")]
    InvalidTool(String),
    #[error("Content merging failed: {0}")]
    ContentMerging(String),
}

pub fn embed_tools_in_chat_request(
    mut openai_request: OpenAiChatRequest,
) -> Result<ChatRequest, ToolEmbeddingError> {
    let system_message_content = extract_system_message_content(&mut openai_request.messages);

    let tools = openai_request.tools.take();

    let mut preamble = String::new();
    if let Some(system_content) = system_message_content {
        preamble.push_str(&system_content);
    }

    if let Some(tools) = &tools {
        if !tools.is_empty() {
            for tool in tools {
                if tool.tool_type != "function" {
                    return Err(ToolEmbeddingError::InvalidTool(format!(
                        "Unsupported tool type: {}",
                        tool.tool_type
                    )));
                }
            }

            if !preamble.is_empty() {
                preamble.push_str("\n\n");
            }
            let tool_xml = generate_tool_xml(tools, &openai_request.model);
            preamble.push_str(&tool_xml);
        }
    }

    if !preamble.is_empty() {
        let first_user_message = openai_request
            .messages
            .iter_mut()
            .find(|msg| msg.role == "user");

        if first_user_message.is_none() {
            return Err(ToolEmbeddingError::NoUserMessages);
        }
        let first_user_message = first_user_message.unwrap();

        let new_content = match &first_user_message.content {
            OpenAiContent::String(text) => OpenAiContent::String(format!(
                "{preamble}

{text}"
            )),
            OpenAiContent::Array(objects) => {
                let original_text = objects
                    .iter()
                    .filter(|obj| obj.content_type == "text")
                    .map(|obj| obj.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                OpenAiContent::String(format!(
                    "{preamble}

{original_text}"
                ))
            }
            OpenAiContent::Null => {
                OpenAiContent::String(preamble) // Just use preamble if original content was null
            }
        };
        first_user_message.content = new_content;
    }

    openai_request
        .to_straico_request()
        .map_err(ToolEmbeddingError::ContentMerging)
}

fn extract_system_message_content(messages: &mut Vec<OpenAiChatMessage>) -> Option<String> {
    messages
        .iter()
        .position(|m| m.role == "system")
        .map(|pos| messages.remove(pos).content.to_string())
}

/// Generates tool XML for embedding in messages.
pub fn generate_tool_xml(tools: &[OpenAiTool], _model: &str) -> String {
    // Determine format based on model
    let format = ToolCallsFormat::default();

    let pre_tools = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;

    let post_tools = format!(
        "\n</tools>\n# Tool Calls\n\nStart with the opening tag {}. For each tool call, return a json object with function name and arguments within {}{} tags:\n{}{{\"name\": <function-name>{} \"arguments\": <args-json-object>}}{}. close the tool calls section with {}\n",
        format.tool_calls_begin,
        format.tool_call_begin,
        format.tool_call_end,
        format.tool_call_begin,
        format.tool_sep,
        format.tool_call_end,
        format.tool_calls_end
    );

    let mut tools_message = String::new();
    tools_message.push_str(pre_tools);
    for tool in tools {
        tools_message.push_str(&serde_json::to_string_pretty(&tool.function).unwrap());
    }
    tools_message.push_str(&post_tools);

    tools_message
}

// Default implementation for OpenAiChatRequest for tests
impl Default for OpenAiChatRequest {
    fn default() -> Self {
        OpenAiChatRequest {
            model: "default-model".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai_types::{OpenAiChatMessage, OpenAiContent, OpenAiFunction};

    #[test]
    fn test_embed_tools_in_chat_request() {
        let request = OpenAiChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: OpenAiContent::String("You are a helpful assistant.".to_string()),
                    tool_call_id: None,
                    name: None,
                    tool_calls: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: OpenAiContent::String("Hello".to_string()),
                    tool_call_id: None,
                    name: None,
                    tool_calls: None,
                },
            ],
            tools: Some(vec![OpenAiTool {
                tool_type: "function".to_string(),
                function: OpenAiFunction {
                    name: "test_function".to_string(),
                    description: Some("A test function".to_string()),
                    parameters: Some(serde_json::json!({ "type": "object", "properties": {} })),
                },
            }]),
            ..Default::default()
        };

        let result = embed_tools_in_chat_request(request).unwrap();
        let user_message_content = result.messages[0].content[0].text.clone();

        assert!(user_message_content.contains("You are a helpful assistant."));
        assert!(user_message_content.contains("<tools>"));
        assert!(user_message_content.contains("test_function"));
        assert!(user_message_content.contains("Hello"));
    }
}

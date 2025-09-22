use straico_client::chat::{Tool, ANTHROPIC_PROMPT_FORMAT, COMMAND_R_PROMPT_FORMAT, DEEPSEEK_PROMPT_FORMAT, LLAMA3_PROMPT_FORMAT, MISTRAL_PROMPT_FORMAT, PromptFormat, QWEN_PROMPT_FORMAT};
use crate::openai_types::{OpenAiChatMessage, OpenAiContent};
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

/// Embeds tool definitions into the first user message of a chat.
/// 
/// This function takes a mutable reference to chat messages and embeds
/// tool definitions as XML in the first user message's content.
/// 
/// # Arguments
/// * `messages` - Mutable reference to chat messages
/// * `tools` - Optional vector of tools to embed
/// * `model` - Model name for model-specific formatting
/// 
/// # Returns
/// * `Ok(())` if successful
/// * `Err(ToolEmbeddingError)` if embedding fails
pub fn embed_tools_in_messages(
    messages: &mut Vec<OpenAiChatMessage>,
    tools: Option<Vec<Tool>>,
    model: &str,
) -> Result<(), ToolEmbeddingError> {
    let tools = match tools {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()), // No tools to embed
    };

    let first_user_msg = find_first_user_message_mut(messages)
        .ok_or(ToolEmbeddingError::NoUserMessages)?;

    let tool_xml = generate_tool_xml(&tools, model);
    let merged_content = merge_tool_xml_with_content(&tool_xml, &first_user_msg.content)?;
    
    first_user_msg.content = merged_content;
    Ok(())
}

/// Generates tool XML for embedding in messages.
/// 
/// This function reuses the existing logic from client/src/chat.rs
/// to generate model-specific tool XML.
/// 
/// # Arguments
/// * `tools` - Slice of tools to embed
/// * `model` - Model name for model-specific formatting
/// 
/// # Returns
/// * String containing the formatted tool XML
pub fn generate_tool_xml(tools: &[Tool], model: &str) -> String {
    // Determine format based on model
    let format = get_prompt_format_for_model(model);
    
    let pre_tools = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;

    let post_tools = format!(
        "\n</tools>\n# Tool Calls\n\nStart with the opening tag {}. For each tool call, return a json object with function name and arguments within {}{} tags:\n{}{{\"name\": <function-name>{} \"arguments\": <args-json-object>}}{}. close the tool calls section with {}\n",
        format.tool_calls.tool_calls_begin,
        format.tool_calls.tool_call_begin,
        format.tool_calls.tool_call_end,
        format.tool_calls.tool_call_begin,
        format.tool_calls.tool_sep,
        format.tool_calls.tool_call_end,
        format.tool_calls.tool_calls_end
    );

    let mut tools_message = String::new();
    tools_message.push_str(pre_tools);
    for tool in tools {
        tools_message.push_str(&serde_json::to_string_pretty(tool).unwrap());
    }
    tools_message.push_str(&post_tools);
    
    tools_message
}

/// Finds the first user message in a message array.
/// 
/// # Arguments
/// * `messages` - Mutable slice of chat messages
/// 
/// # Returns
/// * `Some(&mut OpenAiChatMessage)` if a user message is found
/// * `None` if no user messages exist
fn find_first_user_message_mut(
    messages: &mut [OpenAiChatMessage]
) -> Option<&mut OpenAiChatMessage> {
    messages.iter_mut().find(|msg| msg.role == "user")
}

/// Merges tool XML with original content.
/// 
/// This function merges the generated tool XML with the original user content,
/// handling both string and array content formats.
/// 
/// # Arguments
/// * `tool_xml` - The tool XML to prepend
/// * `original_content` - The original user content
/// 
/// # Returns
/// * `Ok(OpenAiContent)` with merged content
/// * `Err(ToolEmbeddingError)` if merging fails
fn merge_tool_xml_with_content(
    tool_xml: &str,
    original_content: &OpenAiContent,
) -> Result<OpenAiContent, ToolEmbeddingError> {
    match original_content {
        OpenAiContent::String(text) => {
            let merged = format!("{}\n\n{}", tool_xml, text);
            Ok(OpenAiContent::String(merged))
        }
        OpenAiContent::Array(objects) => {
            let user_text = objects.iter()
                .filter(|obj| obj.content_type == "text")
                .map(|obj| obj.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            
            let merged = format!("{}\n\n{}", tool_xml, user_text);
            Ok(OpenAiContent::String(merged))
        }
    }
}

/// Gets the prompt format for a specific model.
/// 
/// This function reuses the model detection logic from chat.rs
/// to determine the appropriate prompt format.
/// 
/// # Arguments
/// * `model` - Model name
/// 
/// # Returns
/// * `PromptFormat` for the specified model
fn get_prompt_format_for_model(model: &str) -> PromptFormat<'static> {
    if model.to_lowercase().contains("anthropic") {
        ANTHROPIC_PROMPT_FORMAT
    } else if model.to_lowercase().contains("mistral") {
        MISTRAL_PROMPT_FORMAT
    } else if model.to_lowercase().contains("llama3") {
        LLAMA3_PROMPT_FORMAT
    } else if model.to_lowercase().contains("command") {
        COMMAND_R_PROMPT_FORMAT
    } else if model.to_lowercase().contains("qwen") {
        QWEN_PROMPT_FORMAT
    } else if model.to_lowercase().contains("deepseek") {
        DEEPSEEK_PROMPT_FORMAT
    } else {
        PromptFormat::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai_types::OpenAiContentObject;

    #[test]
    fn test_embed_tools_string_content() {
        let mut messages = vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Hello".to_string()),
            tool_call_id: None,
            name: None,
        }];
        
        let tools = vec![Tool::Function {
            name: "test_function".to_string(),
            description: Some("Test function".to_string()),
            parameters: None,
        }];

        embed_tools_in_messages(&mut messages, Some(tools), "test-model").unwrap();
        
        match &messages[0].content {
            OpenAiContent::String(content) => {
                assert!(content.contains("<tools>"));
                assert!(content.contains("test_function"));
                assert!(content.contains("Hello"));
            }
            _ => panic!("Expected string content"),
        }
    }

    #[test]
    fn test_no_tools_passthrough() {
        let mut messages = vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Hello".to_string()),
            tool_call_id: None,
            name: None,
        }];
        let original = messages.clone();

        // Test with None tools
        embed_tools_in_messages(&mut messages, None, "test-model").unwrap();
        assert_eq!(messages, original);
        
        // Test with empty tools vector
        embed_tools_in_messages(&mut messages, Some(vec![]), "test-model").unwrap();
        assert_eq!(messages, original);
    }

    #[test]
    fn test_no_user_messages_error() {
        let mut messages = vec![OpenAiChatMessage {
            role: "system".to_string(),
            content: OpenAiContent::String("System message".to_string()),
            tool_call_id: None,
            name: None,
        }];

        let tools = vec![Tool::Function {
            name: "test".to_string(),
            description: None,
            parameters: None,
        }];

        let result = embed_tools_in_messages(&mut messages, Some(tools), "test-model");
        assert!(matches!(result, Err(ToolEmbeddingError::NoUserMessages)));
    }

    #[test]
    fn test_embed_tools_array_content() {
        let mut messages = vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::Array(vec![
                OpenAiContentObject {
                    content_type: "text".to_string(),
                    text: "Hello".to_string(),
                },
                OpenAiContentObject {
                    content_type: "text".to_string(),
                    text: "World".to_string(),
                }
            ]),
            tool_call_id: None,
            name: None,
        }];
        
        let tools = vec![Tool::Function {
            name: "test_function".to_string(),
            description: Some("Test function".to_string()),
            parameters: None,
        }];

        embed_tools_in_messages(&mut messages, Some(tools), "test-model").unwrap();
        
        match &messages[0].content {
            OpenAiContent::String(content) => {
                assert!(content.contains("<tools>"));
                assert!(content.contains("test_function"));
                assert!(content.contains("Hello"));
                assert!(content.contains("World"));
            }
            _ => panic!("Expected string content"),
        }
    }

    #[test]
    fn test_generate_tool_xml() {
        let tools = vec![Tool::Function {
            name: "test_function".to_string(),
            description: Some("Test function".to_string()),
            parameters: None,
        }];

        let xml = generate_tool_xml(&tools, "anthropic-test");
        assert!(xml.contains("<tools>"));
        assert!(xml.contains("test_function"));
        assert!(xml.contains("</tools>"));
    }
}
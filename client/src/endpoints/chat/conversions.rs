use super::tool_calling;
use super::{
    ChatContent, ChatError, ChatMessage, OpenAiChatMessage,
    common_types::ModelProvider,
    request_types::{ChatRequest, OpenAiChatRequest, StraicoChatRequest},
    response_types::{ChatChoice, OpenAiChatResponse, StraicoChatResponse},
};

// Tool-related helper functions moved to tool_calling submodules

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
            if let Some(tool_calls) = tool_calls {
                tool_calling::convert_assistant_with_tools_to_straico(
                    content,
                    &tool_calls,
                    provider,
                )?
            } else {
                ChatMessage::Assistant {
                    content: content.unwrap_or(ChatContent::String(String::new())),
                }
            }
        }
        OpenAiChatMessage::Tool { .. } => tool_calling::convert_tool_message_to_straico(&message)?,
    })
}

impl TryFrom<OpenAiChatRequest> for StraicoChatRequest {
    type Error = ChatError;

    fn try_from(mut request: OpenAiChatRequest) -> Result<Self, Self::Error> {
        let provider = ModelProvider::from(request.chat_request.model.as_str());

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
            builder = builder.message(tool_calling::tools_system_message(&tools, provider)?);
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
        ChatMessage::Assistant { content } => Ok(
            tool_calling::convert_straico_assistant_to_openai(content, provider)?,
        ),
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
        let provider = ModelProvider::from(response.response.model.as_str());

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

// Integration tests for conversions are in tool_calling submodules

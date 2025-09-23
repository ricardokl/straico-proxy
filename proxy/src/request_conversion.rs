use crate::openai_types::{OpenAiChatRequest, OpenAiChatMessage, OpenAiContent};
use straico_client::endpoints::completion::{
    CompletionRequest as OpenAiLegacyRequest, Message,
};
use straico_client::chat::Chat;
use std::borrow::Cow;

/// Convert new chat request to legacy completion request format
pub fn convert_chat_to_legacy_request(chat_request: OpenAiChatRequest) -> OpenAiLegacyRequest<'static> {
    let messages: Vec<Message> = chat_request
        .messages
        .into_iter()
        .map(|msg| convert_openai_message_to_legacy(msg))
        .collect();

    let mut builder = OpenAiLegacyRequest::new()
        .models(Cow::Owned(chat_request.model))
        .message(Chat::new(messages).to_prompt(None, "gpt-3.5-turbo")); // model is not used here

    if let Some(max_tokens) = chat_request.max_tokens {
        builder = builder.max_tokens(max_tokens);
    }
    if let Some(temperature) = chat_request.temperature {
        builder = builder.temperature(temperature);
    }

    builder.build()
}

fn convert_openai_message_to_legacy(msg: OpenAiChatMessage) -> Message {
    let content_text = match msg.content {
        OpenAiContent::String(text) => text,
        OpenAiContent::Array(objects) => objects
            .into_iter()
            .filter(|obj| obj.content_type == "text")
            .map(|obj| obj.text)
            .collect::<Vec<_>>()
            .join(" "),
    };

    match msg.role.as_str() {
        "user" => Message::new_user(content_text),
        "assistant" => Message::new_assistant(Some(content_text), None),
        "system" => Message::new_system(content_text),
        _ => Message::new_user(content_text),
    }
}

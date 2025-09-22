use crate::{
    client::{StraicoClient, StraicoRequestBuilder, NoApiKey},
    endpoints::{ApiResponseData, chat::{ChatRequest, ChatResponse}},
    error::StraicoError,
};

/// Extension trait for StraicoClient to provide chat-specific functionality
pub trait ChatClientExt {
    /// Creates a request builder for the new chat completions endpoint
    fn chat_completions(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest>;
}

impl ChatClientExt for StraicoClient {
    /// Creates a request builder for the new chat completions endpoint
    ///
    /// This method provides access to the new `/v0/chat/completions` endpoint
    /// which supports structured message arrays and content objects.
    ///
    /// # Returns
    /// A `StraicoRequestBuilder` configured for making chat completion requests
    ///
    /// # Example
    /// ```rust
    /// use straico_client::{StraicoClient, endpoints::chat::ChatClientExt};
    /// 
    /// let client = StraicoClient::new();
    /// let response = client
    ///     .chat_completions()
    ///     .bearer_auth("your-api-key")
    ///     .json(chat_request)
    ///     .send()
    ///     .await?;
    /// ```
    fn chat_completions(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest> {
        self.chat()
    }
}

/// Extension trait for handling chat responses
pub trait ChatResponseExt {
    /// Extracts and parses the chat response from the API response data
    fn get_chat_response(self) -> Result<ChatResponse, StraicoError>;
}

impl ChatResponseExt for ApiResponseData {
    /// Extracts and parses the chat response from the API response data
    ///
    /// # Returns
    /// Result containing the parsed ChatResponse or an error
    ///
    /// # Errors
    /// Returns `StraicoError::Serde` if the response data cannot be parsed as a ChatResponse
    fn get_chat_response(self) -> Result<ChatResponse, StraicoError> {
        self.get_chat_response()
            .map_err(StraicoError::Serde)
    }
}

/// Convenience functions for creating chat requests
pub mod builders {
    use super::*;
    use crate::endpoints::chat::ChatMessage;

    /// Creates a simple chat request with a single user message
    ///
    /// # Arguments
    /// * `model` - The model identifier to use
    /// * `message` - The user message text
    ///
    /// # Returns
    /// A ChatRequest ready to be sent
    pub fn simple_chat_request<S: Into<String>, M: Into<String>>(
        model: S, 
        message: M
    ) -> ChatRequest {
        ChatRequest::builder()
            .model(model)
            .message(ChatMessage::user(message))
            .build()
    }

    /// Creates a chat request with system and user messages
    ///
    /// # Arguments
    /// * `model` - The model identifier to use
    /// * `system_message` - The system message text
    /// * `user_message` - The user message text
    ///
    /// # Returns
    /// A ChatRequest with both system and user messages
    pub fn system_user_chat_request<S: Into<String>, Sys: Into<String>, User: Into<String>>(
        model: S,
        system_message: Sys,
        user_message: User,
    ) -> ChatRequest {
        ChatRequest::builder()
            .model(model)
            .message(ChatMessage::system(system_message))
            .message(ChatMessage::user(user_message))
            .build()
    }

    /// Creates a chat request from a conversation history
    ///
    /// # Arguments
    /// * `model` - The model identifier to use
    /// * `messages` - Vector of chat messages forming the conversation
    ///
    /// # Returns
    /// A ChatRequest with the provided conversation history
    pub fn conversation_chat_request<S: Into<String>>(
        model: S,
        messages: Vec<ChatMessage>,
    ) -> ChatRequest {
        ChatRequest::builder()
            .model(model)
            .messages(messages)
            .build()
    }

    /// Creates a chat request with advanced parameters
    ///
    /// # Arguments
    /// * `model` - The model identifier to use
    /// * `messages` - Vector of chat messages
    /// * `temperature` - Optional temperature parameter (0.0 to 2.0)
    /// * `max_tokens` - Optional maximum number of tokens to generate
    ///
    /// # Returns
    /// A ChatRequest with the specified parameters
    pub fn advanced_chat_request<S: Into<String>>(
        model: S,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> ChatRequest {
        let mut builder = ChatRequest::builder()
            .model(model)
            .messages(messages);

        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        if let Some(tokens) = max_tokens {
            builder = builder.max_tokens(tokens);
        }

        builder.build()
    }
}

/// Convenience functions for working with chat responses
pub mod response_utils {
    use super::*;

    /// Extracts the first response content as a string
    ///
    /// # Arguments
    /// * `response` - The chat response to extract content from
    ///
    /// # Returns
    /// Option containing the first response content, or None if no content exists
    pub fn get_first_content(response: &ChatResponse) -> Option<String> {
        response.first_content()
    }

    /// Checks if the response contains tool calls
    ///
    /// # Arguments
    /// * `response` - The chat response to check
    ///
    /// # Returns
    /// True if the response contains tool calls, false otherwise
    pub fn has_tool_calls(response: &ChatResponse) -> bool {
        response.has_tool_calls()
    }

    /// Extracts all choice contents as strings
    ///
    /// # Arguments
    /// * `response` - The chat response to extract content from
    ///
    /// # Returns
    /// Vector of content strings from all choices
    pub fn get_all_contents(response: &ChatResponse) -> Vec<String> {
        response.choices.iter()
            .filter_map(|choice| choice.content_string())
            .collect()
    }

    /// Gets the finish reason for the first choice
    ///
    /// # Arguments
    /// * `response` - The chat response to check
    ///
    /// # Returns
    /// Option containing the finish reason, or None if no choices exist
    pub fn get_finish_reason(response: &ChatResponse) -> Option<&str> {
        response.first_choice()
            .map(|choice| choice.finish_reason.as_str())
    }

    /// Checks if the response finished due to reaching max tokens
    ///
    /// # Arguments
    /// * `response` - The chat response to check
    ///
    /// # Returns
    /// True if the response was truncated due to max tokens
    pub fn was_truncated(response: &ChatResponse) -> bool {
        get_finish_reason(response)
            .map(|reason| reason == "length")
            .unwrap_or(false)
    }

    /// Gets token usage information if available
    ///
    /// # Arguments
    /// * `response` - The chat response to extract usage from
    ///
    /// # Returns
    /// Option containing usage information
    pub fn get_usage(response: &ChatResponse) -> Option<&crate::endpoints::chat::ChatUsage> {
        response.usage.as_ref()
    }
}
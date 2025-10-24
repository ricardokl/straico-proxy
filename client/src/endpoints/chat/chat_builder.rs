use super::common_types::ChatMessage;
use super::ChatRequest;

/// Builder for constructing ChatRequest instances.
///
/// Provides a fluent interface for building chat requests with optional parameters.
#[derive(Debug, Clone, Default)]
pub struct ChatRequestBuilder {
    model: Option<String>,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl ChatRequestBuilder {
    /// Sets the model for the chat request.
    ///
    /// # Arguments
    /// * `model` - The model identifier to use
    ///
    /// # Returns
    /// Self for method chaining
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Adds a message to the chat request.
    ///
    /// # Arguments
    /// * `message` - The chat message to add
    ///
    /// # Returns
    /// Self for method chaining
    pub fn message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Adds multiple messages to the chat request.
    ///
    /// # Arguments
    /// * `messages` - Vector of chat messages to add
    ///
    /// # Returns
    /// Self for method chaining
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Sets the temperature parameter.
    ///
    /// # Arguments
    /// * `temperature` - Temperature value (0.0 to 2.0)
    ///
    /// # Returns
    /// Self for method chaining
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the max_tokens parameter.
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum number of tokens to generate
    ///
    /// # Returns
    /// Self for method chaining
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Builds the ChatRequest.
    ///
    /// # Returns
    /// The constructed ChatRequest
    ///
    /// # Panics
    /// Panics if model is not set
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model.expect("Model must be set"),
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}



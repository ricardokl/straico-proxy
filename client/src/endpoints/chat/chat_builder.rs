use super::ChatRequest;
use super::common_types::ChatMessage;

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

pub trait IntoOption<T> {
    fn into_option(self) -> Option<T>;
}

impl IntoOption<u32> for u32 {
    fn into_option(self) -> Option<u32> {
        Some(self)
    }
}

impl IntoOption<u32> for Option<u32> {
    fn into_option(self) -> Option<u32> {
        self
    }
}

impl IntoOption<f32> for f32 {
    fn into_option(self) -> Option<f32> {
        Some(self)
    }
}

impl IntoOption<f32> for Option<f32> {
    fn into_option(self) -> Option<f32> {
        self
    }
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
    pub fn messages<I>(mut self, messages: I) -> Self
    where
        I: IntoIterator<Item = ChatMessage>,
    {
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
    pub fn temperature<T: Into<Option<f32>>>(mut self, temperature: T) -> Self {
        self.temperature = temperature.into();
        self
    }

    /// Sets the max_tokens parameter.
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum number of tokens to generate
    ///
    /// # Returns
    /// Self for method chaining
    pub fn max_tokens<T: Into<Option<u32>>>(mut self, max_tokens: T) -> Self {
        self.max_tokens = max_tokens.into();
        self
    }

    /// Builds the ChatRequest.
    ///
    /// # Returns
    /// The constructed ChatRequest
    ///
    /// # Panics
    /// Panics if model is not set
    pub fn build(self) -> ChatRequest<ChatMessage> {
        ChatRequest {
            model: self.model.expect("Model must be set"),
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatContent;

    #[test]
    fn test_max_tokens_with_value() {
        let builder = ChatRequestBuilder::default().max_tokens(100);
        assert_eq!(builder.max_tokens, Some(100));
    }

    #[test]
    fn test_max_tokens_with_option() {
        let builder = ChatRequestBuilder::default().max_tokens(Some(100));
        assert_eq!(builder.max_tokens, Some(100));
    }

    #[test]
    fn test_max_tokens_with_none() {
        let builder = ChatRequestBuilder::default().max_tokens(None);
        assert_eq!(builder.max_tokens, None);
    }

    #[test]
    fn test_temperature_with_value() {
        let builder = ChatRequestBuilder::default().temperature(0.7);
        assert_eq!(builder.temperature, Some(0.7));
    }

    #[test]
    fn test_temperature_with_option() {
        let builder = ChatRequestBuilder::default().temperature(Some(0.7));
        assert_eq!(builder.temperature, Some(0.7));
    }

    #[test]
    fn test_temperature_with_none() {
        let builder = ChatRequestBuilder::default().temperature(None);
        assert_eq!(builder.temperature, None);
    }

    #[test]
    fn test_messages_consumes_iterator() {
        let msg = ChatMessage::User {
            content: ChatContent::String("Hello".to_string()),
        };
        let messages = vec![msg.clone()];
        
        // This should compile and run, consuming the vector
        let builder = ChatRequestBuilder::default().messages(messages);
        
        assert_eq!(builder.messages.len(), 1);
        // We can't easily check if it was cloned or moved without a non-Clone type, 
        // but ChatMessage is Clone. The fact that it accepts Vec<ChatMessage> 
        // (which is IntoIterator<Item=ChatMessage>) confirms the signature change.
    }
}

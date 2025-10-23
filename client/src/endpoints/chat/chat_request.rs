use serde::Serialize;

/// A request structure for the new Straico chat endpoint.
///
/// This struct represents a request to the `/v0/chat/completions` endpoint with support
/// for the new message format that uses content arrays instead of formatted prompts.
///
/// # Fields
/// * `model` - Single model identifier (unlike completion endpoint which supports multiple)
/// * `messages` - Array of chat messages with structured content
/// * `temperature` - Optional parameter controlling randomness in generation (0.0 to 2.0)
/// * `max_tokens` - Optional maximum number of tokens to generate
#[derive(Serialize, Debug, Clone, Default)]
pub struct ChatRequest {
    /// The language model to use for generating the chat completion
    pub model: String,
    /// Array of messages forming the conversation context
    pub messages: Vec<ChatMessage>,
    /// Optional parameter controlling randomness in generation (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Represents a single message in the chat conversation.
///
/// Each message variant has specific content requirements:
/// - System: mandatory content for system-level instructions
/// - User: mandatory content for user input
/// - Assistant: mandatory content for assistant responses (unlike OpenAI where it's optional)
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    /// System message with mandatory content
    System {
        /// The message content as structured content objects
        content: Vec<ContentObject>,
    },
    /// User message with mandatory content
    User {
        /// The message content as structured content objects
        content: Vec<ContentObject>,
    },
    /// Assistant message with mandatory content
    Assistant {
        /// The message content as structured content objects
        content: Vec<ContentObject>,
    },
}

/// Represents a single content object within a chat message.
///
/// This structure supports the new Straico chat format where content is represented
/// as an array of typed objects rather than a simple string.
///
/// # Fields
/// * `content_type` - The type of content (typically "text")
/// * `text` - The actual text content
#[derive(Serialize, Debug, Clone)]
pub struct ContentObject {
    /// The type of content object
    #[serde(rename = "type")]
    pub content_type: String,
    /// The text content
    pub text: String,
}

impl ChatRequest {
    /// Creates a new ChatRequest builder.
    ///
    /// # Returns
    /// A new `ChatRequestBuilder` instance for constructing the request
    pub fn builder() -> ChatRequestBuilder {
        ChatRequestBuilder::default()
    }
}

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

impl ChatMessage {
    /// Creates a system message with text content.
    ///
    /// # Arguments
    /// * `text` - The system message text
    ///
    /// # Returns
    /// A new ChatMessage with role "system"
    pub fn system<S: Into<String>>(text: S) -> Self {
        ChatMessage::System {
            content: vec![ContentObject::text(text)],
        }
    }

    /// Creates a user message with text content.
    ///
    /// # Arguments
    /// * `text` - The user message text
    ///
    /// # Returns
    /// A new ChatMessage with role "user"
    pub fn user<S: Into<String>>(text: S) -> Self {
        ChatMessage::User {
            content: vec![ContentObject::text(text)],
        }
    }

    /// Creates an assistant message with text content.
    ///
    /// # Arguments
    /// * `text` - The assistant message text
    ///
    /// # Returns
    /// A new ChatMessage with role "assistant"
    pub fn assistant<S: Into<String>>(text: S) -> Self {
        ChatMessage::Assistant {
            content: vec![ContentObject::text(text)],
        }
    }
}

impl ContentObject {
    /// Creates a new text content object.
    ///
    /// # Arguments
    /// * `text` - The text content
    ///
    /// # Returns
    /// A new ContentObject with type "text"
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self {
            content_type: "text".to_string(),
            text: text.into(),
        }
    }

    /// Creates a new content object with custom type.
    ///
    /// # Arguments
    /// * `content_type` - The type of content
    /// * `text` - The text content
    ///
    /// # Returns
    /// A new ContentObject with the specified type
    pub fn new<S: Into<String>, T: Into<String>>(content_type: S, text: T) -> Self {
        Self {
            content_type: content_type.into(),
            text: text.into(),
        }
    }
}

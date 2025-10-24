use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;

pub use crate::endpoints::chat::response_types::Message;
pub use crate::endpoints::chat::common_types::ToolCall;

/// Represents a chat conversation as a sequence of messages.
///
/// The `Chat` struct is a wrapper around a vector of `Message` values that represents
/// an entire chat conversation between a user, AI assistant, and optionally system messages
/// or tool outputs.
///
/// This struct implements `Deref` to provide direct access to the underlying vector
/// operations while maintaining type safety and encapsulation.
#[derive(Deserialize, Clone, Debug)]
pub struct Chat(pub Vec<Message>);

impl Chat {
    pub fn new(messages: Vec<Message>) -> Self {
        Self(messages)
    }
}

impl Deref for Chat {
    type Target = Vec<Message>;

    /// Implements `Deref` for `Chat` to provide direct access to the underlying vector.
    ///
    /// This method returns a reference to the inner `Vec<Message>` stored in the `Chat`
    /// struct, allowing direct access to vector operations while maintaining
    /// encapsulation.
    ///
    /// # Returns
    ///
    /// A reference to the underlying `Vec<Message>` that stores the chat messages.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents a tool/function that can be called by an AI assistant.
///
/// The `Tool` enum is used to define callable functions that an AI can use during
/// conversation. Each function represents a capability that can be invoked by the
/// assistant.
///
/// # Variants
///
/// * `Function` - Represents a callable function with the following fields:
///   * `name` - The name of the function that can be called
///   * `description` - Optional text describing the function's purpose and behavior
///   * `parameters` - Optional JSON schema defining the function's parameter structure
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase", content = "function")]
pub enum Tool {
    // From the request sent to the server
    // On the same level as 'model, or 'temperature'
    Function {
        /// Name of the function
        name: String,
        /// Optional description of what the function does
        description: Option<String>,
        /// Optional JSON schema of function parameters
        parameters: Option<Value>,
    },
}


use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common_types::{ChatMessage, OpenAiChatMessage};

/// A request structure for the Straico chat endpoint.
///
/// This struct represents a request to the `/v0/chat/completions` endpoint with support
/// for the new message format that uses content arrays instead of formatted prompts.
///
/// # Fields
/// * `model` - Single model identifier (unlike completion endpoint which supports multiple)
/// * `messages` - Array of chat messages with structured content
/// * `temperature` - Optional parameter controlling randomness in generation (0.0 to 2.0)
/// * `max_tokens` - Optional maximum number of tokens to generate
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ChatRequest<T> {
    /// The language model to use for generating the chat completion
    pub model: String,
    /// Array of messages forming the conversation context
    pub messages: Vec<T>,
    /// Optional parameter controlling randomness in generation (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ChatRequest<ChatMessage> {
    /// Creates a new ChatRequest builder.
    ///
    /// # Returns
    /// A new `ChatRequestBuilder` instance for constructing the request
    pub fn builder() -> super::ChatRequestBuilder {
        super::ChatRequestBuilder::default()
    }
}

/// Represents a function definition within a tool.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiFunction {
    /// The name of the function
    pub name: String,
    /// A description of what the function does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The parameters the function accepts, described as a JSON Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Represents a tool available to the model.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiTool {
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition
    pub function: OpenAiFunction,
}

/// Represents a tool choice option.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum OpenAiToolChoice {
    /// A string value like "none", "auto", or "required"
    String(String),
    /// An object specifying a specific tool to use
    Object(OpenAiNamedToolChoice),
}

/// Represents a named tool choice.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiNamedToolChoice {
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The specific function to use
    pub function: OpenAiFunction,
}

/// Represents a complete OpenAI chat request.
///
/// This structure handles incoming OpenAI-compatible requests that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatRequest<T> {
    #[serde(flatten)]
    pub chat_request: ChatRequest<T>,
    /// Optional maximum number of completion tokens (alias for max_tokens)
    #[serde(alias = "max_completion_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Optional tools/functions available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiTool>>,
    /// Optional tool choice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<OpenAiToolChoice>,
}

/// Generates tool XML for embedding in messages.
fn generate_tool_xml(tools: &[OpenAiTool], _model: &str) -> String {
    let pre_tools = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;

    let post_tools = "\n</tools>\n# Tool Calls\n\nStart with the opening tag <tool_calls>. For each tool call, return a json object with function name and arguments within <tool_call></tool_call> tags:\n<tool_call>{\"name\": <function-name>, \"arguments\": <args-json-object>}</tool_call>. close the tool calls section with </tool_calls>\n";

    let mut tools_message = String::new();
    tools_message.push_str(pre_tools);
    for tool in tools {
        tools_message.push_str(&serde_json::to_string_pretty(&tool.function).unwrap());
    }
    tools_message.push_str(post_tools);

    tools_message
}

#[derive(Debug)]
pub enum OpenAiConversionError {
    ToolEmbedding(String),
}

impl std::fmt::Display for OpenAiConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenAiConversionError::ToolEmbedding(msg) => write!(f, "Tool embedding error: {}", msg),
        }
    }
}

impl std::error::Error for OpenAiConversionError {}

impl OpenAiChatRequest<OpenAiChatMessage> {
    /// Converts OpenAI chat request to Straico ChatRequest format.
    ///
    /// This function now handles both regular chat requests and those with tools,
    /// embedding tool definitions into the user message content as needed.
    /// System messages are no longer specially handled and are passed through as-is.
    ///
    /// # Returns
    /// A `ChatRequest` with the message format converted for Straico.
    ///
    /// # Errors
    /// Returns an error if tool embedding fails (e.g., no user message to embed into).
    pub fn to_straico_request(&mut self) -> Result<ChatRequest<ChatMessage>, OpenAiConversionError> {
        let mut messages: Vec<ChatMessage> = self
            .chat_request
            .messages
            .drain(..)
            .map(|msg| msg.into())
            .collect();

        if let Some(tools) = self.tools.take() {
            if !tools.is_empty() {
                for tool in &tools {
                    if tool.tool_type != "function" {
                        return Err(OpenAiConversionError::ToolEmbedding(format!(
                            "Unsupported tool type: {}",
                            tool.tool_type
                        )));
                    }
                }

                let tool_xml = generate_tool_xml(&tools, &self.chat_request.model);
                let system_message = ChatMessage::system(tool_xml);
                messages.insert(0, system_message);
            }
        }

        let mut builder = ChatRequest::builder()
            .model(&self.chat_request.model)
            .messages(messages);

        let max_tokens = self.chat_request.max_tokens.or(self.max_completion_tokens);
        if let Some(tokens) = max_tokens {
            builder = builder.max_tokens(tokens);
        }

        if let Some(temp) = self.chat_request.temperature {
            builder = builder.temperature(temp);
        }

        Ok(builder.build())
    }
}

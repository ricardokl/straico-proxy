use crate::error::CustomError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use straico_client::{
    endpoints::chat::{ChatMessage, ChatRequest, ContentObject},
};

/// OpenAI-compatible content format that can be either a string or an array of content objects.
///
/// This enum handles the dual content format support required by the OpenAI API:
/// - String format: `"content": "Hello world"`
/// - Array format: `"content": [{"type": "text", "text": "Hello world"}]`
/// - Null format: `"content": null` (used when there's no content but other fields like tool_calls exist)
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum OpenAiContent {
    /// Simple string content format
    String(String),
    /// Array of structured content objects
    Array(Vec<OpenAiContentObject>),
    /// Null content (empty content)
    Null,
}

/// Represents a single content object in the OpenAI array format.
///
/// This structure matches the OpenAI API specification for content objects
/// within message content arrays.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiContentObject {
    /// The type of content (typically "text")
    #[serde(rename = "type")]
    pub content_type: String,
    /// The actual text content
    pub text: String,
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
    pub function: OpenAiFunctionName,
}

/// Represents a function name for tool choice.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiFunctionName {
    /// The name of the function
    pub name: String,
}

/// Represents a function call within a tool call.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiFunctionCall {
    /// The name of the function being called
    pub name: String,
    /// The arguments to pass to the function, as a JSON string
    pub arguments: String,
}

/// Represents a tool call made by the assistant.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiToolCall {
    /// The ID of the tool call
    pub id: String,
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function call details
    pub function: OpenAiFunctionCall,
}

/// Represents a chat message in OpenAI format.
///
/// This structure handles incoming OpenAI-style messages that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiChatMessage {
    /// The role of the message sender (system, user, assistant, tool)
    pub role: String,
    /// The message content in either string or array format
    pub content: OpenAiContent,
    /// Optional tool call ID for tool messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional name for function/tool messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional tool calls made by assistant messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// Represents a complete OpenAI chat request.
///
/// This structure handles incoming OpenAI-compatible requests that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatRequest {
    /// The model identifier to use for completion
    pub model: String,
    /// Array of chat messages in OpenAI format
    pub messages: Vec<OpenAiChatMessage>,
    /// Optional temperature parameter (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
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

use std::fmt;

impl OpenAiContent {
    /// Converts OpenAI content into a vector of `OpenAiContentObject`.
    ///
    /// This method normalizes the `OpenAiContent` enum into a consistent
    /// `Vec<OpenAiContentObject>` format.
    ///
    /// # Returns
    /// A `Vec<OpenAiContentObject>` containing the content.
    pub fn into_content_objects(self) -> Vec<OpenAiContentObject> {
        match self {
            OpenAiContent::String(text) => vec![OpenAiContentObject {
                content_type: "text".to_string(),
                text,
            }],
            OpenAiContent::Array(objects) => objects,
            OpenAiContent::Null => vec![],
        }
    }
}

impl fmt::Display for OpenAiContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text: String = match self {
            OpenAiContent::String(s) => s.clone(),
            OpenAiContent::Array(objects) => objects.iter().map(|obj| &obj.text).cloned().collect(),
            OpenAiContent::Null => "".to_string(),
        };
        write!(f, "{text}")
    }
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

impl OpenAiChatRequest {
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
    /// Returns a `CustomError` if tool embedding fails (e.g., no user message to embed into).
    pub fn to_straico_request(&mut self) -> Result<ChatRequest, CustomError> {
        let mut messages: Vec<ChatMessage> =
            self.messages.drain(..).map(|msg| msg.into()).collect();

        if let Some(tools) = self.tools.take() {
            if !tools.is_empty() {
                for tool in &tools {
                    if tool.tool_type != "function" {
                        return Err(CustomError::ToolEmbedding(format!(
                            "Unsupported tool type: {}",
                            tool.tool_type
                        )));
                    }
                }

                let tool_xml = generate_tool_xml(&tools, &self.model);
                let system_message = ChatMessage::system(tool_xml);
                messages.insert(0, system_message);
            }
        }

        let mut builder = ChatRequest::builder().model(&self.model).messages(messages);

        let max_tokens = self.max_tokens.or(self.max_completion_tokens);
        if let Some(tokens) = max_tokens {
            builder = builder.max_tokens(tokens);
        }

        if let Some(temp) = self.temperature {
            builder = builder.temperature(temp);
        }

        Ok(builder.build())
    }
}

impl From<OpenAiContentObject> for ContentObject {
    fn from(obj: OpenAiContentObject) -> Self {
        ContentObject::new(obj.content_type, obj.text)
    }
}

impl From<OpenAiChatMessage> for ChatMessage {
    fn from(msg: OpenAiChatMessage) -> Self {
        let content_objects = msg
            .content
            .into_content_objects()
            .into_iter()
            .map(|obj| obj.into())
            .collect();
        ChatMessage::new(msg.role, content_objects)
    }
}

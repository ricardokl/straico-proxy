use crate::error::CustomError;
use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use straico_client::endpoints::chat::{ChatMessage, ChatRequest, ContentObject};

fn deserialize_content_vector<'de, D>(
    deserializer: D,
) -> Result<Vec<OpenAiContentObject>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ContentHelper {
        String(String),
        Array(Vec<OpenAiContentObject>),
    }

    match ContentHelper::deserialize(deserializer)? {
        ContentHelper::String(s) => Ok(vec![OpenAiContentObject {
            content_type: "text".to_string(),
            text: s,
        }]),
        ContentHelper::Array(a) => Ok(a),
    }
}

fn serialize_content_vector<S>(
    content: &Vec<OpenAiContentObject>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if content.len() == 1 {
        if let Some(first) = content.first() {
            if first.content_type == "text" {
                return serializer.serialize_str(&first.text);
            }
        }
    }
    content.serialize(serializer)
}

fn deserialize_optional_content_vector<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<OpenAiContentObject>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ContentHelper {
        String(String),
        Array(Vec<OpenAiContentObject>),
    }

    match Option::<ContentHelper>::deserialize(deserializer)? {
        Some(ContentHelper::String(s)) => Ok(Some(vec![OpenAiContentObject {
            content_type: "text".to_string(),
            text: s,
        }])),
        Some(ContentHelper::Array(a)) => Ok(Some(a)),
        None => Ok(None),
    }
}

fn serialize_optional_content_vector<S>(
    content: &Option<Vec<OpenAiContentObject>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match content {
        Some(vec) => serialize_content_vector(vec, serializer),
        None => serializer.serialize_none(),
    }
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
    pub function: OpenAiFunction,
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
#[serde(tag = "role", rename_all = "lowercase")]
pub enum OpenAiChatMessage {
    System {
        #[serde(
            deserialize_with = "deserialize_content_vector",
            serialize_with = "serialize_content_vector"
        )]
        content: Vec<OpenAiContentObject>,
    },
    User {
        #[serde(
            deserialize_with = "deserialize_content_vector",
            serialize_with = "serialize_content_vector"
        )]
        content: Vec<OpenAiContentObject>,
    },
    Assistant {
        #[serde(
            deserialize_with = "deserialize_optional_content_vector",
            serialize_with = "serialize_optional_content_vector",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        content: Option<Vec<OpenAiContentObject>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<OpenAiToolCall>>,
    },
    Tool {
        tool_call_id: String,
        #[serde(
            deserialize_with = "deserialize_content_vector",
            serialize_with = "serialize_content_vector"
        )]
        content: Vec<OpenAiContentObject>,
    },
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

// A new struct for serializing tool output
#[derive(Serialize)]
struct ToolOutput {
    tool_call_id: String,
    output: String,
}

impl From<OpenAiChatMessage> for ChatMessage {
    fn from(msg: OpenAiChatMessage) -> Self {
        match msg {
            OpenAiChatMessage::System { content } => ChatMessage::new(
                "system".to_string(),
                content.into_iter().map(|c| c.into()).collect(),
            ),
            OpenAiChatMessage::User { content } => ChatMessage::new(
                "user".to_string(),
                content.into_iter().map(|c| c.into()).collect(),
            ),
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                let mut content_objects: Vec<ContentObject> = content
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c| c.into())
                    .collect();

                if let Some(tool_calls) = tool_calls {
                    if !tool_calls.is_empty() {
                        content_objects.push(ContentObject::text("<tool_calls>"));
                        let tool_calls_str =
                            serde_json::to_string(&tool_calls).unwrap_or_default();
                        content_objects.push(ContentObject::text(tool_calls_str));
                        content_objects.push(ContentObject::text("</tool_calls>"));
                    }
                }
                ChatMessage::new("assistant".to_string(), content_objects)
            }
            OpenAiChatMessage::Tool {
                tool_call_id,
                content,
            } => {
                let output = content
                    .first()
                    .map(|obj| obj.text.clone())
                    .unwrap_or_default();
                let tool_output = ToolOutput {
                    tool_call_id,
                    output,
                };
                let json_output = serde_json::to_string(&tool_output).unwrap_or_default();
                let new_content = format!("<tool_output>{}</tool_output>", json_output);
                ChatMessage::new("user".to_string(), vec![ContentObject::text(new_content)])
            }
        }
    }
}

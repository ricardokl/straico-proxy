use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Represents the details of a function call in the response.
///
/// # Fields
/// * `name` - The name of the function called
/// * `arguments` - The function arguments; internally `serde_json::Value`, serialized as JSON string
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ChatFunctionCall {
    /// The name of the function being called
    pub name: String,
    /// The arguments to pass to the function. Internally stored as a `serde_json::Value`,
    /// but serialized as a JSON string containing the JSON-encoded object.
    ///
    /// # Example
    /// - In memory: `serde_json::json!({"key": "value"})`
    /// - Serialized: `"{\"key\":\"value\"}"`
    #[serde(
        deserialize_with = "string_or_object_to_value_deserializer",
        serialize_with = "value_to_string_serializer"
    )]
    pub arguments: serde_json::Value,
}

pub fn string_or_object_to_value_deserializer<'de, D>(
    deserializer: D,
) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrObject {
        String(String),
        Object(serde_json::Value),
    }

    match StringOrObject::deserialize(deserializer)? {
        StringOrObject::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        StringOrObject::Object(v) => Ok(v),
    }
}

pub fn value_to_string_serializer<S>(
    value: &serde_json::Value,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

/// Represents a tool call made by the assistant.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ToolCall {
    /// The ID of the tool call
    pub id: String,
    /// The index of the tool call in the list of tool calls
    #[serde(default)]
    pub index: Option<usize>,
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function call details
    pub function: ChatFunctionCall,
}

/// High-level provider that produced or will consume a given model ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProvider {
    Anthropic,
    OpenAI,
    Zai,
    MoonshotAI,
    Qwen,
    Google,
    Unknown,
}

impl From<&str> for ModelProvider {
    fn from(value: &str) -> Self {
        // Model IDs are typically in the form "provider/model-name"
        let provider_prefix = value.split('/').next().unwrap_or("").to_lowercase();
        match provider_prefix.as_str() {
            "anthropic" => ModelProvider::Anthropic,
            "openai" => ModelProvider::OpenAI,
            // GLM models
            "z-ai" => ModelProvider::Zai,
            // Kimi models
            "moonshotai" => ModelProvider::MoonshotAI,
            "qwen" => ModelProvider::Qwen,
            "google" => ModelProvider::Google,
            _ => ModelProvider::Unknown,
        }
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
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "function")]
pub enum OpenAiTool {
    Function(OpenAiFunction),
}

/// Represents a tool choice option.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum OpenAiToolChoice {
    /// A string value like "none", "auto", or "required"
    String(String),
    /// An object specifying a specific tool to use
    Object(OpenAiTool),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provider_detection_anthropic() {
        assert_eq!(
            ModelProvider::from("anthropic/claude-3-opus"),
            ModelProvider::Anthropic
        );
    }

    #[test]
    fn test_provider_detection_openai() {
        assert_eq!(ModelProvider::from("openai/gpt-4"), ModelProvider::OpenAI);
    }

    #[test]
    fn test_provider_detection_zai() {
        assert_eq!(ModelProvider::from("z-ai/glm-4"), ModelProvider::Zai);
    }

    #[test]
    fn test_provider_detection_moonshotai() {
        assert_eq!(
            ModelProvider::from("moonshotai/moonshot-v1"),
            ModelProvider::MoonshotAI
        );
    }

    #[test]
    fn test_provider_detection_qwen() {
        assert_eq!(ModelProvider::from("qwen/qwen-max"), ModelProvider::Qwen);
    }

    #[test]
    fn test_provider_detection_google() {
        assert_eq!(
            ModelProvider::from("google/gemini-pro"),
            ModelProvider::Google
        );
    }

    #[test]
    fn test_chat_function_call_serialization() {
        let fc = ChatFunctionCall {
            name: "test_func".to_string(),
            arguments: json!({
                "arg1": "val1",
                "arg2": 42
            }),
        };

        let serialized = serde_json::to_string(&fc).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        // The arguments field should be a string, not an object
        assert!(parsed["arguments"].is_string());

        // The string should be a valid JSON representation of the original object
        let arguments_str = parsed["arguments"].as_str().unwrap();
        let arguments_json: serde_json::Value = serde_json::from_str(arguments_str).unwrap();
        assert_eq!(
            arguments_json,
            json!({
                "arg1": "val1",
                "arg2": 42
            })
        );
    }

    #[test]
    fn test_chat_function_call_deserialization_from_string() {
        let json_data = json!({
            "name": "test_func",
            "arguments": "{\"arg1\": \"val1\", \"arg2\": 42}"
        });

        let fc: ChatFunctionCall = serde_json::from_value(json_data).unwrap();
        assert_eq!(fc.name, "test_func");
        assert_eq!(
            fc.arguments,
            json!({
                "arg1": "val1",
                "arg2": 42
            })
        );
    }

    #[test]
    fn test_chat_function_call_deserialization_from_object() {
        let json_data = json!({
            "name": "test_func",
            "arguments": {
                "arg1": "val1",
                "arg2": 42
            }
        });

        let fc: ChatFunctionCall = serde_json::from_value(json_data).unwrap();
        assert_eq!(fc.name, "test_func");
        assert_eq!(
            fc.arguments,
            json!({
                "arg1": "val1",
                "arg2": 42
            })
        );
    }
}

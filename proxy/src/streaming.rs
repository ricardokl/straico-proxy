use serde::Serialize;
use serde_json::{json, Value};

use straico_client::endpoints::chat::common_types::{OpenAiChatMessage, ToolCall};
use straico_client::endpoints::chat::response_types::{ChatChoice, OpenAiChatResponse, Usage};

#[derive(Serialize, Debug, Clone)]
pub struct CompletionStream {
    pub choices: Vec<ChoiceStream>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    pub usage: Usage,
}

#[derive(Serialize, Debug, Clone)]
pub struct ChoiceStream {
    pub index: u8,
    pub delta: Delta,
    pub finish_reason: Option<Box<str>>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl From<OpenAiChatMessage> for Delta {
    fn from(value: OpenAiChatMessage) -> Self {
        match value {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                if let Some(tool_calls) = tool_calls {
                    Self {
                        role: Some("assistant".into()),
                        content: None,
                        tool_calls: Some(tool_calls),
                    }
                } else {
                    Self {
                        role: Some("assistant".into()),
                        content: content.map(|c| c.to_string().into()),
                        tool_calls: None,
                    }
                }
            }
            _ => Self::default(),
        }
    }
}

impl From<ChatChoice<OpenAiChatMessage>> for ChoiceStream {
    fn from(value: ChatChoice<OpenAiChatMessage>) -> Self {
        Self {
            index: value.index,
            delta: value.message.into(),
            finish_reason: Some(value.finish_reason.into()),
        }
    }
}

impl From<OpenAiChatResponse> for CompletionStream {
    fn from(value: OpenAiChatResponse) -> Self {
        Self {
            choices: value.choices.into_iter().map(Into::into).collect(),
            object: value.object.into(),
            id: value.id.into(),
            model: value.model.into(),
            created: value.created,
            usage: value.usage,
        }
    }
}

pub fn create_initial_chunk(model: &str, id: &str, created: u64) -> Value {
    json!({
        "choices": [{
            "delta": {
                "role": "assistant"
            },
            "index": 0
        }],
        "object": "chat.completion.chunk",
        "id": id,
        "model": model,
        "created": created
    })
}

pub fn create_heartbeat_chunk() -> Value {
    json!({
        "choices": [{
            "delta": {},
            "index": 0
        }],
        "object": "chat.completion.chunk"
    })
}

pub fn create_error_chunk(error: &str) -> Value {
    json!({
        "error": {
            "message": error
        }
    })
}

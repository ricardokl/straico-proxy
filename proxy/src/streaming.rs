use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use straico_client::endpoints::chat::common_types::{OpenAiChatMessage, ToolCall};
use straico_client::endpoints::chat::response_types::{ChatChoice, OpenAiChatResponse, Usage};
use straico_client::StraicoChatResponse;

use crate::CustomError;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(try_from = "StraicoChatResponse")]
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

impl TryFrom<StraicoChatResponse> for CompletionStream {
    type Error = CustomError;
    fn try_from(value: StraicoChatResponse) -> Result<Self, Self::Error> {
        Ok(OpenAiChatResponse::try_from(value).map(Into::into)?)
    }
}

impl TryFrom<CompletionStream> for Bytes {
    type Error = CustomError;
    fn try_from(value: CompletionStream) -> Result<Self, Self::Error> {
        let string = serde_json::to_string(&value)?;
        Ok(Bytes::from(format!("data: {}\n\n", string)))
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

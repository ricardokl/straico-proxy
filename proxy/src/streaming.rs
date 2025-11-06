use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use straico_client::endpoints::chat::common_types::{OpenAiChatMessage, ToolCall};
use straico_client::endpoints::chat::response_types::{ChatChoice, OpenAiChatResponse, Usage};
use straico_client::StraicoChatResponse;

use crate::CustomError;

/// Enum representing different types of SSE chunks
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum SseChunk {
    /// Data chunk containing a CompletionStream
    Data(CompletionStream),
    /// Done message (typically "[DONE]")
    Done(String),
}

/// Wrapper type that handles SSE formatting with efficient byte serialization
#[derive(Debug, Clone)]
pub struct SseFormattedChunk(SseChunk);

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

impl CompletionStream {
    /// Creates an initial SSE chunk with basic metadata and assistant role
    pub fn initial_chunk(model: &str, id: &str, created: u64) -> Self {
        Self {
            choices: vec![ChoiceStream {
                index: 0,
                delta: Delta {
                    role: Some("assistant".into()),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            object: "chat.completion.chunk".into(),
            id: id.into(),
            model: model.into(),
            created,
            usage: Usage::default(), // All zeros
        }
    }

    /// Creates a heartbeat SSE chunk with empty delta for keep-alive
    pub fn heartbeat_chunk() -> Self {
        Self {
            choices: vec![ChoiceStream {
                index: 0,
                delta: Delta::default(), // Empty delta
                finish_reason: None,
            }],
            object: "chat.completion.chunk".into(),
            id: "".into(), // Empty for heartbeat
            model: "".into(),
            created: 0,
            usage: Usage::default(),
        }
    }
}

impl From<CompletionStream> for SseFormattedChunk {
    fn from(stream: CompletionStream) -> Self {
        Self(SseChunk::Data(stream))
    }
}

impl From<String> for SseFormattedChunk {
    fn from(done_msg: String) -> Self {
        Self(SseChunk::Done(done_msg))
    }
}

impl TryFrom<SseFormattedChunk> for Bytes {
    type Error = CustomError;
    fn try_from(value: SseFormattedChunk) -> Result<Self, Self::Error> {
        let json_bytes = match value.0 {
            SseChunk::Data(stream) => serde_json::to_vec(&stream)?,
            SseChunk::Done(msg) => msg.into_bytes(),
        };

        // Prepend "data: " and append "\n\n"
        let mut sse_bytes = Vec::with_capacity(json_bytes.len() + 8); // "data: " (6) + "\n\n" (2)
        sse_bytes.extend_from_slice(b"data: ");
        sse_bytes.extend_from_slice(&json_bytes);
        sse_bytes.extend_from_slice(b"\n\n");

        Ok(Bytes::from(sse_bytes))
    }
}

// Keep the old implementation for backward compatibility during transition
// impl TryFrom<CompletionStream> for Bytes {
//     type Error = CustomError;
//     fn try_from(value: CompletionStream) -> Result<Self, Self::Error> {
//         SseFormattedChunk::from(value).try_into()
//     }
// }

pub fn create_error_chunk(error: &str) -> Value {
    json!({
        "error": {
            "message": error
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use straico_client::endpoints::chat::response_types::Usage;

    #[test]
    fn test_sse_formatted_chunk_data_serialization() {
        let stream = CompletionStream {
            choices: vec![ChoiceStream {
                index: 0,
                delta: Delta {
                    role: Some("assistant".into()),
                    content: Some("Hello".into()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            object: "chat.completion.chunk".into(),
            id: "test-id".into(),
            model: "test-model".into(),
            created: 1234567890,
            usage: Usage::default(),
        };

        let sse_chunk = SseFormattedChunk::from(stream);
        let bytes: Result<Bytes, CustomError> = sse_chunk.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert!(bytes_str.starts_with("data: "));
        assert!(bytes_str.ends_with("\n\n"));

        // Verify JSON structure
        let json_part = &bytes_str[6..bytes_str.len() - 2]; // Remove "data: " and "\n\n"
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(parsed["object"], "chat.completion.chunk");
        assert_eq!(parsed["id"], "test-id");
        assert_eq!(parsed["model"], "test-model");
        assert_eq!(parsed["created"], 1234567890);
    }

    #[test]
    fn test_sse_formatted_chunk_done_serialization() {
        let done_chunk = SseFormattedChunk::from("[DONE]".to_string());
        let bytes: Result<Bytes, CustomError> = done_chunk.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert_eq!(bytes_str, "data: [DONE]\n\n");
    }

    #[test]
    fn test_completion_stream_initial_chunk() {
        let chunk = CompletionStream::initial_chunk("gpt-4", "test-id", 1234567890);

        assert_eq!(chunk.object.as_ref(), "chat.completion.chunk");
        assert_eq!(chunk.id.as_ref(), "test-id");
        assert_eq!(chunk.model.as_ref(), "gpt-4");
        assert_eq!(chunk.created, 1234567890);
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].index, 0);
        assert_eq!(
            chunk.choices[0].delta.role.as_ref().unwrap().as_ref(),
            "assistant"
        );
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_completion_stream_heartbeat_chunk() {
        let chunk = CompletionStream::heartbeat_chunk();

        assert_eq!(chunk.object.as_ref(), "chat.completion.chunk");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].index, 0);
        assert!(chunk.choices[0].delta.role.is_none());
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_backward_compatibility_completion_stream_to_bytes() {
        let stream = CompletionStream::initial_chunk("test-model", "test-id", 1234567890);
        let bytes: Result<Bytes, CustomError> = stream.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert!(bytes_str.starts_with("data: "));
        assert!(bytes_str.ends_with("\n\n"));
    }

    #[test]
    fn test_sse_chunk_enum_serialization() {
        // Test Data variant
        let data_chunk = SseChunk::Data(CompletionStream::heartbeat_chunk());
        let json = serde_json::to_string(&data_chunk).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["object"], "chat.completion.chunk");

        // Test Done variant
        let done_chunk = SseChunk::Done("[DONE]".to_string());
        let json = serde_json::to_string(&done_chunk).unwrap();
        assert_eq!(json, "\"[DONE]\"");
    }

    #[test]
    fn test_byte_efficiency() {
        let stream = CompletionStream::initial_chunk("test", "id", 123);

        // Test new implementation
        let new_bytes: Bytes = SseFormattedChunk::from(stream.clone()).try_into().unwrap();

        // Test old-style implementation (for comparison)
        let old_style = format!("data: {}\n\n", serde_json::to_string(&stream).unwrap());
        let old_bytes = Bytes::from(old_style);

        // Both should produce identical output
        assert_eq!(new_bytes, old_bytes);
    }

    #[test]
    fn test_performance_comparison() {
        use std::time::Instant;

        let stream = CompletionStream::initial_chunk("test-model", "test-id", 1234567890);
        let iterations = 1000;

        // Benchmark new implementation
        let start = Instant::now();
        for _ in 0..iterations {
            let _: Bytes = SseFormattedChunk::from(stream.clone()).try_into().unwrap();
        }
        let new_duration = start.elapsed();

        // Benchmark old-style implementation
        let start = Instant::now();
        for _ in 0..iterations {
            let old_style = format!("data: {}\n\n", serde_json::to_string(&stream).unwrap());
            let _: Bytes = Bytes::from(old_style);
        }
        let old_duration = start.elapsed();

        println!("New implementation: {:?}", new_duration);
        println!("Old implementation: {:?}", old_duration);

        // New implementation should be faster or at least not significantly slower
        // This is more of a performance indicator than a strict test
        assert!(new_duration <= old_duration * 2); // Allow up to 2x slower as safety margin
    }
}

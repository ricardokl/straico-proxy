use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, clap::ValueEnum, Default)]
pub enum HeartbeatChar {
    /// Empty heartbeat (current behavior, no content)
    #[default]
    Empty,
    /// Zero-width space (\u200b)
    Zwsp,
    /// Zero-width non-joiner (\u200c)
    Zwnj,
    /// Word joiner (\u2060)
    Wj,
}

impl HeartbeatChar {
    pub fn as_str(&self) -> &str {
        match self {
            HeartbeatChar::Empty => "",
            HeartbeatChar::Zwsp => "\u{200b}",
            HeartbeatChar::Zwnj => "\u{200c}",
            HeartbeatChar::Wj => "\u{2060}",
        }
    }
}

use straico_client::endpoints::chat::common_types::{OpenAiChatMessage, ToolCall};
use straico_client::endpoints::chat::response_types::{ChatChoice, OpenAiChatResponse, Usage};
use straico_client::StraicoChatResponse;

use crate::ProxyError;

/// Enum representing different types of SSE chunks
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum SseChunk {
    /// Data chunk containing a CompletionStream
    Data(CompletionStream),
    /// Done message (typically "[DONE]")
    Done(String),
    /// Error chunk containing error information
    Error(Value),
}

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
                        role: None,
                        content: None,
                        tool_calls: Some(tool_calls),
                    }
                } else {
                    Self {
                        role: None,
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
            object: "chat.completion.chunk".into(),
            id: value.id.into(),
            model: value.model.into(),
            created: value.created,
            usage: value.usage,
        }
    }
}

impl TryFrom<StraicoChatResponse> for CompletionStream {
    type Error = ProxyError;
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

    /// Creates a heartbeat SSE chunk with configurable content for keep-alive
    pub fn heartbeat_chunk(heartbeat_char: &HeartbeatChar) -> Self {
        let content = heartbeat_char.as_str();
        let content_option = if content.is_empty() {
            None
        } else {
            Some(content.into())
        };

        Self {
            choices: vec![ChoiceStream {
                index: 0,
                delta: Delta {
                    content: content_option,
                    ..Default::default()
                },
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

impl From<CompletionStream> for SseChunk {
    fn from(stream: CompletionStream) -> Self {
        SseChunk::Data(stream)
    }
}

impl From<String> for SseChunk {
    fn from(done_msg: String) -> Self {
        SseChunk::Done(done_msg)
    }
}

impl From<Value> for SseChunk {
    fn from(error_value: Value) -> Self {
        SseChunk::Error(error_value)
    }
}

impl From<ProxyError> for SseChunk {
    fn from(error: ProxyError) -> Self {
        SseChunk::Error(error.to_streaming_chunk())
    }
}

impl TryFrom<SseChunk> for Bytes {
    type Error = ProxyError;
    fn try_from(value: SseChunk) -> Result<Self, Self::Error> {
        let json_bytes = match value {
            SseChunk::Data(stream) => serde_json::to_vec(&stream)?,
            SseChunk::Done(msg) => msg.into_bytes(),
            SseChunk::Error(error_value) => serde_json::to_vec(&error_value)?,
        };

        // Prepend "data: " and append "\n\n"
        let mut sse_bytes = Vec::with_capacity(json_bytes.len() + 8); // "data: " (6) + "\n\n" (2)
        sse_bytes.extend_from_slice(b"data: ");
        sse_bytes.extend_from_slice(&json_bytes);
        sse_bytes.extend_from_slice(b"\n\n");

        Ok(Bytes::from(sse_bytes))
    }
}

pub fn create_error_chunk(error: &str) -> Value {
    json!({
        "error": {
            "message": error,
            "type": "server_error",
            "code": "streaming_error"
        }
    })
}

/// Creates an error chunk with proper OpenAI-compatible error format
pub fn create_error_chunk_with_type(
    error: &str,
    error_type: &str,
    error_code: Option<&str>,
) -> Value {
    json!({
        "error": {
            "message": error,
            "type": error_type,
            "code": error_code
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use straico_client::endpoints::chat::response_types::Usage;

    #[test]
    fn test_sse_chunk_data_serialization() {
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

        let sse_chunk = SseChunk::from(stream);
        let bytes: Result<Bytes, ProxyError> = sse_chunk.try_into();
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
    fn test_sse_chunk_done_serialization() {
        let done_chunk = SseChunk::from("[DONE]".to_string());
        let bytes: Result<Bytes, ProxyError> = done_chunk.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert_eq!(bytes_str, "data: [DONE]\n\n");
    }

    #[test]
    fn test_sse_chunk_error_serialization() {
        let error_chunk = SseChunk::from(create_error_chunk("Test error message"));
        let bytes: Result<Bytes, ProxyError> = error_chunk.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert!(bytes_str.starts_with("data: "));
        assert!(bytes_str.ends_with("\n\n"));
        assert!(bytes_str.contains("Test error message"));

        // Verify JSON structure includes new fields
        let json_part = &bytes_str[6..bytes_str.len() - 2]; // Remove "data: " and "\n\n"
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(parsed["error"]["message"], "Test error message");
        assert_eq!(parsed["error"]["type"], "server_error");
        assert_eq!(parsed["error"]["code"], "streaming_error");
    }

    #[test]
    fn test_create_error_chunk_with_type() {
        let error_chunk = create_error_chunk_with_type(
            "Custom error message",
            "invalid_request_error",
            Some("invalid_parameter"),
        );

        assert_eq!(error_chunk["error"]["message"], "Custom error message");
        assert_eq!(error_chunk["error"]["type"], "invalid_request_error");
        assert_eq!(error_chunk["error"]["code"], "invalid_parameter");
    }

    #[test]
    fn test_custom_error_to_sse_chunk() {
        let custom_error = ProxyError::InvalidParameter {
            parameter: "temperature".to_string(),
            reason: "Invalid parameter".to_string(),
        };
        let error_chunk = SseChunk::from(custom_error);
        let bytes: Result<Bytes, ProxyError> = error_chunk.try_into();
        assert!(bytes.is_ok());

        let bytes_str = String::from_utf8(bytes.unwrap().to_vec()).unwrap();
        assert!(bytes_str.contains("Invalid parameter"));
    }

    #[test]
    fn test_streaming_error_flow() {
        // Simulate the streaming error handling flow
        use futures::{stream, StreamExt};

        // Create a stream that produces an error
        let error_stream = stream::iter(vec![
            Ok(CompletionStream::initial_chunk(
                "test-model",
                "test-id",
                123,
            )),
            Err(ProxyError::InvalidParameter {
                parameter: "model".to_string(),
                reason: "Simulated API error".to_string(),
            }),
        ])
        .map(|result| match result {
            Ok(stream) => SseChunk::from(stream).try_into(),
            Err(error) => SseChunk::from(error).try_into(),
        });

        // Collect the stream results
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let results: Vec<Result<Bytes, ProxyError>> =
            runtime.block_on(async { error_stream.collect().await });

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok()); // Initial chunk should succeed
        assert!(results[1].is_ok()); // Error chunk should also succeed (converted to bytes)

        // Verify the error chunk contains the error message
        let error_bytes = results[1].as_ref().unwrap();
        let error_str = String::from_utf8_lossy(error_bytes);
        assert!(error_str.contains("Simulated API error"));
        assert!(error_str.starts_with("data: "));
        assert!(error_str.ends_with("\n\n"));
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
        // Test Empty variant
        let chunk = CompletionStream::heartbeat_chunk(&HeartbeatChar::Empty);
        assert!(chunk.choices[0].delta.content.is_none());

        // Test Zwsp variant
        let chunk = CompletionStream::heartbeat_chunk(&HeartbeatChar::Zwsp);
        assert_eq!(
            chunk.choices[0].delta.content.as_ref().unwrap().as_ref(),
            "\u{200b}"
        );

        // Test Zwnj variant
        let chunk = CompletionStream::heartbeat_chunk(&HeartbeatChar::Zwnj);
        assert_eq!(
            chunk.choices[0].delta.content.as_ref().unwrap().as_ref(),
            "\u{200c}"
        );

        // Test Wj variant
        let chunk = CompletionStream::heartbeat_chunk(&HeartbeatChar::Wj);
        assert_eq!(
            chunk.choices[0].delta.content.as_ref().unwrap().as_ref(),
            "\u{2060}"
        );
    }

    #[test]
    fn test_sse_chunk_enum_serialization() {
        // Test Data variant
        let data_chunk = SseChunk::Data(CompletionStream::heartbeat_chunk(&HeartbeatChar::Empty));
        let json = serde_json::to_string(&data_chunk).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["object"], "chat.completion.chunk");

        // Test Done variant
        let done_chunk = SseChunk::Done("[DONE]".to_string());
        let json = serde_json::to_string(&done_chunk).unwrap();
        assert_eq!(json, "\"[DONE]\"");

        // Test Error variant
        let error_chunk = SseChunk::Error(create_error_chunk("Test error"));
        let json = serde_json::to_string(&error_chunk).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"]["message"], "Test error");
    }

    #[test]
    fn test_byte_efficiency() {
        let stream = CompletionStream::initial_chunk("test", "id", 123);

        // Test new implementation
        let new_bytes: Bytes = SseChunk::from(stream.clone()).try_into().unwrap();

        // Test old-style implementation (for comparison)
        let old_style = format!("data: {}\n\n", serde_json::to_string(&stream).unwrap());
        let old_bytes = Bytes::from(old_style);

        // Both should produce identical output
        assert_eq!(new_bytes, old_bytes);
    }
}

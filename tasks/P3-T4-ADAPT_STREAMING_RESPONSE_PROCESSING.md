# P3-T4: Adapt Streaming Response Processing

## Objective
Adapt the existing streaming response processing logic to work with the new chat endpoint response format while maintaining OpenAI compatibility.

## Background
The current streaming response processing works with the prompt endpoint format. We need to adapt this to handle the new chat endpoint response format while preserving the same SSE output format for OpenAI compatibility.

## Tasks

### 1. Define New Chat Stream Response Types
**File**: `client/src/endpoints/chat/chat_response.rs`

**Streaming Response Structures**:
```rust
#[derive(Deserialize, Debug, Clone)]
pub struct ChatStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatStreamChoice>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatStreamChoice {
    pub index: u32,
    pub delta: ChatStreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

### 2. Implement Conversion from Chat to Completion Format
**File**: `proxy/src/streaming_conversion.rs` (new file)

**Conversion Logic**:
```rust
use straico_client::endpoints::chat::{ChatStreamChunk, ChatStreamChoice, ChatStreamDelta};
use crate::streaming::{CompletionStream, ChoiceStream, Delta};

impl From<ChatStreamChunk> for CompletionStream {
    fn from(chat_chunk: ChatStreamChunk) -> Self {
        CompletionStream {
            choices: chat_chunk.choices.into_iter().map(Into::into).collect(),
            object: "chat.completion.chunk".into(),
            id: chat_chunk.id.into(),
            model: chat_chunk.model.into(),
            created: chat_chunk.created,
            #[cfg(not(test))]
            usage: Usage::default(), // Will be updated in final chunk
            #[cfg(test)]
            usage: (),
        }
    }
}

impl From<ChatStreamChoice> for ChoiceStream {
    fn from(chat_choice: ChatStreamChoice) -> Self {
        ChoiceStream {
            index: chat_choice.index as u8,
            delta: chat_choice.delta.into(),
            finish_reason: chat_choice.finish_reason.map(Into::into),
        }
    }
}

impl From<ChatStreamDelta> for Delta {
    fn from(chat_delta: ChatStreamDelta) -> Self {
        Delta {
            role: chat_delta.role.map(Into::into),
            content: chat_delta.content.map(Into::into),
            tool_calls: chat_delta.tool_calls,
        }
    }
}
```

### 3. Implement SSE Chunk Parsing
**File**: `proxy/src/sse_parser.rs` (new file)

**SSE Parsing Logic**:
```rust
use bytes::Bytes;
use serde_json;

pub fn parse_sse_chunk(chunk: &Bytes) -> Result<ChatStreamChunk, SseParseError> {
    let chunk_str = std::str::from_utf8(chunk)
        .map_err(|e| SseParseError::InvalidUtf8(e.to_string()))?;

    // Parse SSE format: "data: {json}\n\n"
    for line in chunk_str.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..]; // Remove "data: " prefix
            
            if json_str == "[DONE]" {
                return Err(SseParseError::StreamComplete);
            }

            let chat_chunk: ChatStreamChunk = serde_json::from_str(json_str)
                .map_err(|e| SseParseError::JsonParse(e.to_string()))?;
            
            return Ok(chat_chunk);
        }
    }

    Err(SseParseError::NoDataFound)
}

#[derive(Debug, thiserror::Error)]
pub enum SseParseError {
    #[error("Invalid UTF-8 in chunk: {0}")]
    InvalidUtf8(String),
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    #[error("No data found in SSE chunk")]
    NoDataFound,
    #[error("Stream complete")]
    StreamComplete,
}
```

### 4. Adapt Streaming Iterator Logic
**File**: `proxy/src/streaming_conversion.rs`

**Enhanced Iterator Support**:
```rust
// Extend existing streaming logic to handle chat format
impl From<ChatStreamChunk> for Vec<CompletionStream> {
    fn from(chat_chunk: ChatStreamChunk) -> Self {
        // Convert chat chunk to completion format
        let completion_stream: CompletionStream = chat_chunk.into();
        
        // Use existing iterator logic to break into individual chunks
        completion_stream.into_iter().collect()
    }
}

pub fn process_chat_stream_chunk(
    chat_chunk: ChatStreamChunk,
) -> impl Iterator<Item = CompletionStream> {
    // Convert to completion format and iterate
    let completion_stream: CompletionStream = chat_chunk.into();
    completion_stream.into_iter()
}
```

### 5. Implement Tool Call Handling in Streams
**File**: `proxy/src/streaming_conversion.rs`

**Tool Call Processing**:
```rust
impl ChatStreamDelta {
    pub fn process_tool_calls(&mut self, model: &str) -> Result<(), ToolProcessingError> {
        if let Some(content) = &self.content {
            // Check if content contains tool call XML
            if content.contains("<tool_call>") {
                // Parse tool calls from content
                let tool_calls = parse_tool_calls_from_content(content, model)?;
                
                // Move tool calls to tool_calls field
                self.tool_calls = Some(tool_calls);
                
                // Clear content since it contained tool calls
                self.content = None;
            }
        }
        Ok(())
    }
}

fn parse_tool_calls_from_content(
    content: &str,
    model: &str,
) -> Result<Vec<ToolCall>, ToolProcessingError> {
    // Reuse existing tool parsing logic from completion_response.rs
    // Adapt for streaming context where tool calls might be partial
    
    let format = get_prompt_format_for_model(model);
    let pattern = format!(
        r"{}(.*?){}",
        regex::escape(&format.tool_calls.tool_call_begin),
        regex::escape(&format.tool_calls.tool_call_end)
    );
    
    let re = regex::Regex::new(&pattern)?;
    let tool_calls = re
        .find_iter(content)
        .map(|m| parse_single_tool_call(m.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(tool_calls)
}
```

### 6. Implement Heartbeat and Error Handling
**File**: `proxy/src/chat_streaming.rs` (extend existing)

**Enhanced Stream Processing**:
```rust
impl ChatStreamingHandler {
    fn create_enhanced_sse_stream(
        &self,
        chat_stream: impl Stream<Item = Result<ChatStreamChunk, StraicoError>>,
        model: String,
        stream_id: String,
    ) -> impl Stream<Item = Result<web::Bytes, CustomError>> {
        let heartbeat_interval = tokio::time::interval(self.config.heartbeat_interval);
        
        let response_stream = stream::unfold(
            (chat_stream, heartbeat_interval, false, true),
            |(mut stream, mut hb, finished, mut first_tick)| async move {
                if finished {
                    return None;
                }

                if first_tick {
                    hb.tick().await; // Consume immediate first tick
                    first_tick = false;
                }

                tokio::select! {
                    biased;

                    chunk_result = stream.next() => {
                        match chunk_result {
                            Some(Ok(chat_chunk)) => {
                                // Process tool calls if present
                                let mut processed_chunk = chat_chunk;
                                for choice in &mut processed_chunk.choices {
                                    if let Err(e) = choice.delta.process_tool_calls(&model) {
                                        log::warn!("Tool call processing error: {}", e);
                                    }
                                }

                                // Convert to completion format
                                let completion_chunks = process_chat_stream_chunk(processed_chunk);
                                
                                // Return first chunk (more will come in subsequent iterations)
                                if let Some(chunk) = completion_chunks.into_iter().next() {
                                    let json = serde_json::to_string(&chunk).unwrap();
                                    let bytes = web::Bytes::from(format!("data: {}\n\n", json));
                                    Some((Ok(bytes), (stream, hb, false, first_tick)))
                                } else {
                                    // Continue to next chunk if this one was empty
                                    Some((Ok(web::Bytes::new()), (stream, hb, false, first_tick)))
                                }
                            }
                            Some(Err(e)) => {
                                let error_chunk = create_error_chunk(&e.to_string());
                                let json = serde_json::to_string(&error_chunk).unwrap();
                                let bytes = web::Bytes::from(format!("data: {}\n\n", json));
                                Some((Ok(bytes), (stream, hb, true, first_tick)))
                            }
                            None => {
                                // Stream ended
                                Some((Ok(web::Bytes::from("data: [DONE]\n\n")), (stream, hb, true, first_tick)))
                            }
                        }
                    },
                    _ = hb.tick() => {
                        // Send heartbeat
                        let hb_chunk = create_heartbeat_chunk();
                        let json = serde_json::to_string(&hb_chunk).unwrap();
                        Some((Ok(web::Bytes::from(format!("data: {}\n\n", json))), (stream, hb, false, first_tick)))
                    }
                }
            },
        );

        // Add initial chunk
        let initial_chunk = create_initial_chunk(&model, &stream_id);
        let initial_stream = stream::once(async move {
            Ok(web::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::to_string(&initial_chunk).unwrap()
            )))
        });

        initial_stream.chain(response_stream)
    }
}
```

### 7. Add Comprehensive Error Handling
**File**: `proxy/src/streaming_errors.rs` (new file)

**Error Types**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StreamingProcessingError {
    #[error("SSE parsing failed: {0}")]
    SseParsing(#[from] SseParseError),
    #[error("Tool processing failed: {0}")]
    ToolProcessing(#[from] ToolProcessingError),
    #[error("Stream conversion failed: {0}")]
    StreamConversion(String),
    #[error("Chunk processing failed: {0}")]
    ChunkProcessing(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ToolProcessingError {
    #[error("Regex compilation failed: {0}")]
    RegexError(#[from] regex::Error),
    #[error("JSON parsing failed: {0}")]
    JsonParsing(#[from] serde_json::Error),
    #[error("Invalid tool call format: {0}")]
    InvalidFormat(String),
}
```

## Deliverables

1. **New Modules**:
   - `proxy/src/streaming_conversion.rs` - Chat to completion conversion
   - `proxy/src/sse_parser.rs` - SSE chunk parsing
   - `proxy/src/streaming_errors.rs` - Error handling

2. **Enhanced Streaming**:
   - Chat stream response types
   - Conversion logic to maintain compatibility
   - Tool call processing in streams
   - Enhanced error handling

3. **Integration**:
   - Updated streaming handler
   - Heartbeat and error handling
   - OpenAI format preservation

## Success Criteria

- [ ] Chat stream chunks convert to completion format correctly
- [ ] SSE parsing works with new format
- [ ] Tool calls processed correctly in streams
- [ ] Heartbeat mechanism maintained
- [ ] Error handling comprehensive
- [ ] OpenAI compatibility preserved
- [ ] Performance acceptable

## Time Estimate
**Duration**: 4-5 hours

## Dependencies
- **P3-T3**: Implement Streaming Request Handling

## Next Task
**P3-T5**: Update Error Handling and Fallbacks

## Notes
- Maintain exact SSE format compatibility with existing implementation
- Focus on robust tool call processing in streaming context
- Ensure error handling doesn't break streams
- Test with various chunk sizes and formats
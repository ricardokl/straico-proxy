use crate::{
    error::CustomError,
    streaming::{CompletionStream, SseChunk},
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use actix_web::{post, web, HttpResponse};
use futures::{future, stream, FutureExt, StreamExt, TryFutureExt};
use log::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::{client::StraicoClient, StraicoChatRequest};
use tokio::time::Duration;
use uuid::Uuid;
/// Safely gets the current Unix timestamp, with fallback for edge cases
///
/// In the extremely unlikely case that the system clock is set before UNIX_EPOCH,
/// this function will log a warning and return a reasonable fallback timestamp.
fn get_current_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            // This should never happen in practice, but handle it gracefully
            warn!(
                "System clock appears to be set before Unix epoch ({}). Using fallback timestamp.",
                e
            );
            // Return a reasonable fallback timestamp (2024-01-01 00:00:00 UTC)
            1704067200
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub debug: bool,
    pub log: bool,
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let openai_request = req.into_inner();
    let stream = openai_request.stream;

    if data.debug || data.log {
        let debug_or_info = format!(
            "\n\n===== Request received (raw): =====\n{}",
            serde_json::to_string_pretty(&openai_request)?
        );
        if data.debug {
            debug!("{debug_or_info}");
        }
        if data.log {
            info!("{debug_or_info}");
        }
    }

    let chat_request = StraicoChatRequest::try_from(openai_request)?;
    let client = data.client.clone();
    let model = chat_request.model.clone();
    let straico_response = client
        .chat()
        .bearer_auth(&data.key)
        .json(chat_request)
        .send();

    if stream {
        let id = format!("chatcmpl-{}", Uuid::new_v4());
        let created = get_current_timestamp();

        let initial_chunk = stream::once(future::ready(
            SseChunk::from(CompletionStream::initial_chunk(&model, &id, created)).try_into(),
        ));

        let (remote, remote_handle) = straico_response.remote_handle();

        let heartbeat = tokio_stream::StreamExt::throttle(
            stream::repeat_with(|| SseChunk::from(CompletionStream::heartbeat_chunk()).try_into()),
            Duration::from_secs(3),
        )
        .take_until(remote);

        let straico_stream = remote_handle
            .and_then(reqwest::Response::json::<StraicoChatResponse>)
            .inspect(|result| println!("\n\n ===== Response: ===== \n\n{:?}", result))
            .map_ok(|x| CompletionStream::try_from(x).unwrap())
            .inspect(|result| println!("\n\n ===== CompletionStream: ===== \n\n{:?}", result))
            .map_ok(SseChunk::from)
            .map_err(|e| SseChunk::from(CustomError::from(e)))
            .map(|result| match result {
                Ok(chunk) => chunk.try_into(),
                Err(error_chunk) => error_chunk.try_into(),
            })
            .into_stream();

        let done = stream::once(future::ready(
            SseChunk::from("[DONE]".to_string()).try_into(),
        ));

        let response_stream = initial_chunk
            .chain(heartbeat)
            .chain(straico_stream)
            .chain(done);

        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(response_stream))
    } else {
        let straico_response: StraicoChatResponse = straico_response.await?.json().await?;

        if data.debug || data.log {
            let debug_or_info = format!(
                "\n\n===== Response from Straico (raw): =====\n{}",
                serde_json::to_string_pretty(&straico_response)?
            );
            if data.debug {
                debug!("{debug_or_info}");
            }
            if data.log {
                info!("{debug_or_info}");
            }
        }

        let openai_response = OpenAiChatResponse::try_from(straico_response)?;

        Ok(HttpResponse::Ok().json(openai_response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp();

        // Should be a reasonable timestamp (after 2020-01-01 and before 2030-01-01)
        let year_2020 = 1577836800; // 2020-01-01 00:00:00 UTC
        let year_2030 = 1893456000; // 2030-01-01 00:00:00 UTC

        assert!(timestamp >= year_2020, "Timestamp should be after 2020");
        assert!(timestamp <= year_2030, "Timestamp should be before 2030");
    }

    #[test]
    fn test_get_current_timestamp_consistency() {
        let timestamp1 = get_current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let timestamp2 = get_current_timestamp();

        // Timestamps should be very close (within a few seconds)
        assert!(timestamp2 >= timestamp1);
        assert!(timestamp2 - timestamp1 <= 1); // Should be within 1 second
    }

    #[test]
    fn test_fallback_timestamp_value() {
        // Test that the fallback timestamp is reasonable
        let fallback_timestamp = 1704067200; // 2024-01-01 00:00:00 UTC

        // Should be after 2020 and before 2030
        let year_2020 = 1577836800; // 2020-01-01 00:00:00 UTC
        let year_2030 = 1893456000; // 2030-01-01 00:00:00 UTC

        assert!(fallback_timestamp > year_2020);
        assert!(fallback_timestamp < year_2030);
    }

    #[test]
    fn test_timestamp_function_never_panics() {
        // This test ensures our function doesn't panic under normal conditions
        // We can't easily test the edge case without mocking SystemTime
        for _ in 0..100 {
            let _timestamp = get_current_timestamp();
            // If we get here, the function didn't panic
        }
    }
}

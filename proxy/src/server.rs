use crate::{
    error::CustomError,
    streaming::{CompletionStream, SseChunk},
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use actix_web::{post, web, HttpResponse};
use bytes::Bytes;
use futures::{future, stream, FutureExt, StreamExt, TryFutureExt};
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::{client::StraicoClient, StraicoChatRequest};
use tokio::time::Duration;
use uuid::Uuid;
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
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

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
            .and_then(reqwest::Response::json::<CompletionStream>)
            .map_ok(SseChunk::from)
            .map_ok_or_else(|x| Err(x.into()), Bytes::try_from)
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

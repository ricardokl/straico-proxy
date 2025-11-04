use crate::{
    error::CustomError,
    streaming::{
        create_error_chunk, create_heartbeat_chunk, create_initial_chunk, CompletionStream,
    },
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{post, web, HttpResponse};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    future::{self},
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use log::{debug, error, info};
use straico_client::{client::StraicoClient, StraicoChatRequest};
use tokio::time::{Duration};
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

        let initial_chunk = stream::once(future::ready(Ok(Bytes::from(format!(
            "data: {}\n\n",
            serde_json::to_string(&create_initial_chunk(&model, &id, created)).unwrap()
        )))));

        let (remote, remote_handle) = straico_response.remote_handle();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let _ = remote.await;
            let _ = tx.send(());
        });

        let heartbeat = stream::unfold((), |()| async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            Some((
                Ok(Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::to_string(&create_heartbeat_chunk()).unwrap()
                ))),
                (),
            ))
        })
        .take_until(rx);

        let final_stream = stream::once(async move {
            let response = remote_handle.await;
            let result = handle_response(response, data.debug, data.log).await;
            let bytes = match result {
                Ok(straico_response) => {
                    let openai_response = OpenAiChatResponse::try_from(straico_response).unwrap();
                    let completion_chunk = CompletionStream::from(openai_response);
                    let json = serde_json::to_string(&completion_chunk).unwrap();
                    let response_bytes = format!("data: {json}\n\n");
                    Bytes::from(response_bytes)
                }
                Err(e) => {
                    error!("Error handling Straico response: {}", e);
                    let error_chunk = create_error_chunk(&e.to_string());
                    let json = serde_json::to_string(&error_chunk).unwrap();
                    Bytes::from(format!("data: {json}\n\n"))
                }
            };
            Ok::<_, CustomError>(bytes)
        });

        let done = stream::once(future::ready(Ok(Bytes::from("data: [DONE]\n\n"))));

        let response_stream: BoxStream<Result<Bytes, CustomError>> =
            Box::pin(initial_chunk.chain(heartbeat).chain(final_stream).chain(done));

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

async fn handle_response(
    response: Result<reqwest::Response, reqwest::Error>,
    debug: bool,
    log: bool,
) -> Result<StraicoChatResponse, CustomError> {
    let response_bytes = match response {
        Ok(resp) => resp.bytes().await?,
        Err(e) => return Err(CustomError::from(e)),
    };

    if debug || log {
        let response_json = serde_json::from_slice::<serde_json::Value>(&response_bytes)
            .and_then(|json| serde_json::to_string_pretty(&json))
            .unwrap_or_else(|_| String::from_utf8_lossy(&response_bytes).to_string());
        if debug {
            debug!("\n\n===== Response from Straico (raw): =====\n{response_json}");
        }
        if log {
            info!("\n\n===== Response from Straico (raw): =====\n{response_json}");
        }
    }

    let straico_response = serde_json::from_slice::<StraicoChatResponse>(&response_bytes)?;
    Ok(straico_response)
}

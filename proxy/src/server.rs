use crate::{
    error::CustomError,
    streaming::{
        create_error_chunk, create_heartbeat_chunk, create_initial_chunk, CompletionStream,
    },
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use actix_web::{post, web, HttpResponse};
use bytes::Bytes;
use futures_util::stream::StreamExt;
use log::{debug, error, info};
use straico_client::{client::StraicoClient, StraicoChatRequest};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::ReceiverStream;
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
        let request_json = serde_json::to_string_pretty(&openai_request).unwrap();
        if data.debug {
            debug!("\n\n===== Request received (raw): =====\n{request_json}");
        }
        if data.log {
            info!("\n\n===== Request received (raw): =====\n{request_json}");
        }
    }
    if stream {
        let (tx, rx) = mpsc::channel(10);
        let app_state = data.clone();
        let id = format!("chatcmpl-{}", Uuid::new_v4());
        let model = openai_request.chat_request.model.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(3));
            let chat_request = match StraicoChatRequest::try_from(openai_request) {
                Ok(req) => req,
                Err(e) => {
                    error!("Error converting request: {}", e);
                    let error_chunk = create_error_chunk(&e.to_string());
                    let json = serde_json::to_string(&error_chunk).unwrap();
                    let _ = tx.send(Bytes::from(format!("data: {json}\n\n"))).await;
                    let _ = tx.send(Bytes::from("data: [DONE]\n\n")).await;
                    return;
                }
            };
            let initial_chunk = create_initial_chunk(&model, &id);
            let json = serde_json::to_string(&initial_chunk).unwrap();
            if tx.send(Bytes::from(format!("data: {json}\n\n"))).await.is_err() {
                return;
            }

            let client = app_state.client.clone();
            let straico_response_future = client
                .chat()
                .bearer_auth(&app_state.key)
                .json(chat_request)
                .send();
            let mut straico_response_future = Box::pin(straico_response_future);

            let response = loop {
                tokio::select! {
                    res = &mut straico_response_future => {
                        break res;
                    },
                    _ = ticker.tick() => {
                        let heartbeat = create_heartbeat_chunk();
                        let json = serde_json::to_string(&heartbeat).unwrap();
                        if tx.send(Bytes::from(format!("data: {json}\n\n"))).await.is_err() {
                            return;
                        }
                    }
                }
            };

            let tx_clone = tx.clone();
            let handle_response = async move {
                let response_bytes = match response {
                    Ok(resp) => resp.bytes().await?,
                    Err(e) => return Err(CustomError::from(e)),
                };

                if app_state.debug || app_state.log {
                    let response_json = serde_json::from_slice::<serde_json::Value>(&response_bytes)
                        .and_then(|json| serde_json::to_string_pretty(&json))
                        .unwrap_or_else(|_| {
                            String::from_utf8_lossy(&response_bytes).to_string()
                        });
                    if app_state.debug {
                        debug!("\n\n===== Response from Straico (raw): =====\n{response_json}");
                    }
                    if app_state.log {
                        info!("\n\n===== Response from Straico (raw): =====\n{response_json}");
                    }
                }

                let straico_response = serde_json::from_slice::<StraicoChatResponse>(&response_bytes)
                    .map_err(CustomError::SerdeJson)?;
                let openai_response = OpenAiChatResponse::try_from(straico_response)?;

                let stream_iterator = CompletionStream::from(openai_response).into_iter();
                for chunk in stream_iterator {
                    let json = serde_json::to_string(&chunk).unwrap();
                    if tx_clone
                        .send(Bytes::from(format!("data: {json}\n\n")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok::<(), CustomError>(())
            };

            let tx_clone2 = tx.clone();
            if let Err(e) = handle_response.await {
                error!("Error handling Straico response: {}", e);
                let error_chunk = create_error_chunk(&e.to_string());
                let json = serde_json::to_string(&error_chunk).unwrap();
                let _ = tx_clone2.send(Bytes::from(format!("data: {json}\n\n"))).await;
            }

            let _ = tx.send(Bytes::from("data: [DONE]\n\n")).await;
        });

        let stream = ReceiverStream::new(rx).map(Ok::<_, CustomError>);
        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        let chat_request = straico_client::StraicoChatRequest::try_from(openai_request)?;
        let client = data.client.clone();
        let straico_response = client
            .chat()
            .bearer_auth(&data.key)
            .json(chat_request)
            .send()
            .await?;

        let response_bytes = straico_response.bytes().await?;

        if data.debug || data.log {
            let response_json = serde_json::from_slice::<serde_json::Value>(&response_bytes)
                .and_then(|json| serde_json::to_string_pretty(&json))
                .unwrap_or_else(|_| String::from_utf8_lossy(&response_bytes).to_string());

            if data.debug {
                debug!("\n\n===== Response from Straico (raw): =====\n{response_json}");
            }
            if data.log {
                info!("\n\n===== Response from Straico (raw): =====\n{response_json}");
            }
        }

        let response = serde_json::from_slice::<StraicoChatResponse>(&response_bytes)
            .map_err(CustomError::SerdeJson)?;
        let openai_response = OpenAiChatResponse::try_from(response)?;

        Ok(HttpResponse::Ok().json(openai_response))
    }
}

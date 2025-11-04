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
        let (tx, rx) = mpsc::channel(10);
        let id = format!("chatcmpl-{}", Uuid::new_v4());

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(3));
            let initial_chunk = create_initial_chunk(&model, &id);
            let json = serde_json::to_string(&initial_chunk).unwrap();
            if tx
                .send(Bytes::from(format!("data: {json}\n\n")))
                .await
                .is_err()
            {
                return;
            }

            let mut straico_response_future = Box::pin(straico_response);

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

                if data.debug || data.log {
                    let response_json =
                        serde_json::from_slice::<serde_json::Value>(&response_bytes)
                            .and_then(|json| serde_json::to_string_pretty(&json))
                            .unwrap_or_else(|_| {
                                String::from_utf8_lossy(&response_bytes).to_string()
                            });
                    if data.debug {
                        debug!("\n\n===== Response from Straico (raw): =====\n{response_json}");
                    }
                    if data.log {
                        info!("\n\n===== Response from Straico (raw): =====\n{response_json}");
                    }
                }

                let straico_response =
                    serde_json::from_slice::<StraicoChatResponse>(&response_bytes)?;
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
                let _ = tx_clone2
                    .send(Bytes::from(format!("data: {json}\n\n")))
                    .await;
            }

            let _ = tx.send(Bytes::from("data: [DONE]\n\n")).await;
        });

        let stream = ReceiverStream::new(rx).map(Ok::<_, CustomError>);
        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(stream))
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

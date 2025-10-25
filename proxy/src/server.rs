use crate::{
    error::CustomError,
    streaming::CompletionStream,
    types::{
        ChatChoice, OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse,
    },
};
use actix_web::{post, web, HttpResponse};
use bytes::Bytes;
use futures_util::stream::{self, StreamExt};
use log::{debug, info};
use straico_client::client::StraicoClient;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub debug: bool,
    pub log: bool,
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest<OpenAiChatMessage>>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let mut openai_request = req.into_inner();

    if data.debug || data.log {
        let request_json = serde_json::to_string_pretty(&openai_request).unwrap();
        if data.debug {
            debug!("\n\n===== Request received (raw): =====\n{request_json}");
        }
        if data.log {
            info!("\n\n===== Request received (raw): =====\n{request_json}");
        }
    }

    let chat_request = openai_request.to_straico_request()?;

    let client = data.client.clone();
    let straico_response = client
        .chat()
        .bearer_auth(&data.key)
        .json(chat_request.clone())
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

    if openai_request.stream {
        let stream_iterator = CompletionStream::from(response).into_iter();
        let stream = stream::iter(stream_iterator)
            .map(|chunk| {
                let json = serde_json::to_string(&chunk).unwrap();
                Ok::<_, CustomError>(Bytes::from(format!("data: {json}\n\n")))
            })
            .chain(stream::once(async { Ok(Bytes::from("data: [DONE]\n\n")) }));

        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        let openai_response = OpenAiChatResponse {
            id: response.response.id,
            object: response.response.object,
            created: response.response.created,
            model: response.response.model,
            choices: response
                .response
                .choices
                .into_iter()
                .map(|choice| ChatChoice {
                    index: choice.index,
                    message: OpenAiChatMessage::from(choice.message),
                    finish_reason: choice.finish_reason,
                    logprobs: None,
                })
                .collect(),
            usage: response.response.usage,
        };
        Ok(HttpResponse::Ok().json(openai_response))
    }
}



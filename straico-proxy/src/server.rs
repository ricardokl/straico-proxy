use crate::{
    error::CustomError,
    streaming::{create_heartbeat_chunk, create_initial_chunk, CompletionStream},
    AppState,
};
use actix_web::{
    post, web, Either, HttpResponse, HttpResponseBuilder,
    http::StatusCode,
};
use anyhow::anyhow;
use futures::{stream, StreamExt};
use log::{debug, error};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;
use straico_client::{
    chat::{Chat, Tool},
    endpoints::completion::{
        completion_request::CompletionRequest, completion_response::Completion,
    },
};
use tokio::sync::mpsc;

/// Represents a chat completion request in the OpenAI API format
///
/// This struct maps incoming API requests to the internal completion request format,
/// providing compatibility with OpenAI-style chat completions.
///
/// # Fields
/// * `model` - The model identifier to use for completion
/// * `messages` - The chat history and prompt messages
/// * `max_tokens` - Optional maximum number of tokens to generate
/// * `temperature` - Optional temperature parameter for controlling randomness
/// * `_stream` - Optional streaming parameter (currently unused)
/// * `tools` - Optional list of tools available to the model
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(into = "CompletionRequest")]
struct OpenAiRequest<'a> {
    /// The model identifier to use for completion (e.g. "gpt-3.5-turbo")
    model: Cow<'a, str>,
    /// The conversation history and prompt messages
    messages: Chat,
    /// Maximum number of tokens to generate in the completion response
    #[serde(alias = "max_completion_tokens")]
    max_tokens: Option<u32>,
    /// Controls randomness in the response generation (0.0 to 1.0)
    temperature: Option<f32>,
    /// Whether to stream the response
    //#[serde(default = "default_streaming")]
    #[serde(default)]
    stream: bool,
    /// List of tools/functions available to the model during completion
    tools: Option<Vec<Tool>>,
}

//fn default_streaming() -> bool {
//    true
//}

impl<'a> From<OpenAiRequest<'a>> for CompletionRequest<'a> {
    /// Converts an OpenAI-style chat completion request into a CompletionRequest
    ///
    /// Takes an OpenAiRequest which contains chat messages, model selection, and optional
    /// parameters like max_tokens and temperature, and converts it into a CompletionRequest.
    /// The conversion process handles optional fields by conditionally building the request
    /// based on which parameters are present.
    ///
    /// # Arguments
    /// * `value` - The OpenAiRequest to convert containing messages and parameters
    ///
    /// # Returns
    /// A CompletionRequest configured with the specified messages and parameters
    fn from(value: OpenAiRequest<'a>) -> Self {
        let builder = CompletionRequest::new()
            .models(value.model.clone())
            .message(value.messages.to_prompt(value.tools, &value.model));
        match (value.max_tokens, value.temperature) {
            (Some(x), Some(y)) => builder.max_tokens(x).temperature(y).build(),
            (Some(x), None) => builder.max_tokens(x).build(),
            (None, Some(y)) => builder.temperature(y).build(),
            (None, None) => builder.build(),
        }
    }
}

/// Handles OpenAI-style chat completion API requests
///
/// This endpoint processes chat completion requests in the OpenAI API format, forwards them to the
/// underlying completion service, and returns the generated response. It supports debug logging of
/// requests and responses when enabled.
///
/// # Arguments
/// * `req` - The incoming chat completion request in OpenAI format
/// * `data` - Shared application state containing client and configuration
///
/// # Returns
/// * `Result<impl Responder, Error>` - The completion response or error
#[post("/v1/chat/completions")]
async fn openai_completion<'a>(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<Either<web::Json<Completion>, HttpResponse>, CustomError> {
    let req_inner = req.into_inner();
    if data.print_request_raw {
        debug!("\n\n===== Request recieved (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    let req_inner_oa: OpenAiRequest = serde_json::from_value(req_inner.clone())?;

    if data.print_request_converted {
        let converted_request: CompletionRequest = req_inner_oa.clone().into();
        debug!("\n\n===== Request recieved (converted): =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&converted_request).unwrap()
        );
    }

    if req_inner_oa.stream {
        let stream_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        let stream_id = format!("chatcmpl-{}", stream_id);

        let (tx, rx) = mpsc::channel(4);

        let app_state = data.clone();
        let req_for_background = req_inner_oa.clone();
        let req_inner_for_err = req_inner.clone();
        tokio::spawn(async move {
            let client = app_state.client.clone();
            let res = client
                .completion()
                .bearer_auth(&app_state.key)
                .json(req_for_background)
                .send()
                .await;

            match res {
                Ok(response) => match response.get_completion() {
                    Ok(completion_response) => {
                        match completion_response.parse() {
                            Ok(parsed_response) => {
                                if app_state.print_response_converted {
                                    debug!("\n\n===== Received response (converted): =====");
                                    debug!(
                                        "\n{}",
                                        serde_json::to_string_pretty(&parsed_response).unwrap()
                                    );
                                }
                                let completion_stream = CompletionStream::from(parsed_response);
                                for chunk in completion_stream {
                                    if tx.send(Ok(chunk)).await.is_err() {
                                        error!("Failed to send chunk to stream");
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse completion response: {}", e);
                                let _ = tx
                                    .send(Err(CustomError::ResponseParse(req_inner_for_err)))
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(CustomError::Anyhow(anyhow!(e.to_string())))).await;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(e.into())).await;
                }
            }
        });

        let stream = create_streaming_response(rx, req_inner_oa.model.to_string(), stream_id);

        Ok(Either::Right(
            HttpResponseBuilder::new(StatusCode::OK)
                .content_type("text/event-stream")
                .append_header(("Cache-Control", "no-cache"))
                .append_header(("Connection", "keep-alive"))
                .streaming(stream),
        ))
    } else {
        // Non-streaming logic remains the same
        let client = data.client.clone();
        let response = client
            .completion()
            .bearer_auth(&data.key)
            .json(req_inner_oa)
            .send()
            .await?
            .get_completion()?;

        if data.print_response_raw {
            debug!("\n\n===== Received response (raw): =====");
            debug!("\n{}", serde_json::to_string_pretty(&response).unwrap());
        }

        let parsed_response = response
            .parse()
            .map_err(|_| CustomError::ResponseParse(req_inner))?;

        if data.print_response_converted {
            debug!("\n\n===== Received response (converted): =====");
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&parsed_response).unwrap()
            );
        }
        Ok(Either::Left(web::Json(parsed_response)))
    }
}

fn create_streaming_response(
    rx: mpsc::Receiver<Result<CompletionStream, CustomError>>,
    model: String,
    id: String,
) -> impl futures::Stream<Item = Result<web::Bytes, CustomError>> {
    let initial_chunk = create_initial_chunk(&model, &id);
    let initial_stream = stream::once(async move {
        Ok(web::Bytes::from(format!(
            "data: {}\n\n",
            serde_json::to_string(&initial_chunk).unwrap()
        )))
    });

    let heartbeat_interval = tokio::time::interval(Duration::from_secs(15));

    let response_stream = stream::unfold(
        (rx, heartbeat_interval, false, true),
        |(mut rx, mut hb, finished, mut first_tick)| async move {
            if finished {
                return None;
            }

            if first_tick {
                hb.tick().await; // Consume the immediate first tick
                first_tick = false;
            }

            tokio::select! {
                biased;

                res = rx.recv() => {
                    match res {
                        Some(Ok(chunk)) => {
                            let json = serde_json::to_string(&chunk).unwrap();
                            let bytes = web::Bytes::from(format!("data: {}\n\n", json));
                            Some((Ok(bytes), (rx, hb, false, first_tick)))
                        }
                        Some(Err(e)) => {
                            let json = serde_json::to_string(&e.to_streaming_chunk()).unwrap();
                            let bytes = web::Bytes::from(format!("data: {}\n\n", json));
                            Some((Ok(bytes), (rx, hb, true, first_tick)))
                        }
                        None => {
                             // Channel closed, we are done
                            Some((Ok(web::Bytes::from("data: [DONE]\n\n")), (rx, hb, true, first_tick)))
                        }
                    }
                },
                _ = hb.tick() => {
                    let hb_chunk = create_heartbeat_chunk();
                    let json = serde_json::to_string(&hb_chunk).unwrap();
                    Some((Ok(web::Bytes::from(format!("data: {}\n\n", json))), (rx, hb, false, first_tick)))
                }
            }
        },
    );

    initial_stream.chain(response_stream)
}

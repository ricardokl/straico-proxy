use crate::AppState;
use crate::streaming::CompletionStream;
use actix_web::{
    Either, Error, HttpResponse, HttpResponseBuilder, error::ErrorInternalServerError,
    http::StatusCode, post, web,
};
use futures::{stream, StreamExt};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use straico_client::{
    chat::{Chat, Tool},
    endpoints::completion::{
        completion_request::CompletionRequest, completion_response::Completion,
    },
};

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
) -> Result<Either<web::Json<Completion>, HttpResponse>, Error> {
    let req_inner = req.into_inner();
    if data.print_request_raw {
        debug!("\n\n===== Request recieved (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    let client = data.client.clone();
    let req_inner_oa: OpenAiRequest = serde_json::from_value(req_inner)?;

    if data.print_request_converted {
        let converted_request: CompletionRequest = req_inner_oa.clone().into();
        debug!("\n\n===== Request recieved (converted): =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&converted_request).unwrap()
        );
    }

    let stream = req_inner_oa.stream;
    let response = client
        .completion()
        .bearer_auth(&data.key)
        .json(req_inner_oa)
        .send()
        .await
        .map_err(ErrorInternalServerError)?
        .get_completion()
        .map_err(ErrorInternalServerError)?;

    if data.print_response_raw {
        debug!("\n\n===== Received response (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&response).unwrap());
    }

    let parsed_response = response.parse().map_err(ErrorInternalServerError)?;

    if data.print_response_converted {
        debug!("\n\n===== Received response (converted): =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&parsed_response).unwrap()
        );
    }

    if stream {
        let completion_stream = CompletionStream::from(parsed_response);
        let stream = stream::iter(completion_stream).map(|chunk| {
            match serde_json::to_string(&chunk) {
                Ok(json) => {
                    Ok::<_, actix_web::Error>(web::Bytes::from(format!("data: {}\n\n", json)))
                }
                Err(e) => {
                    error!("Error serializing chunk: {}", e);
                    Ok(web::Bytes::from(format!("data: {{}}\n\n"))) // Send empty object as fallback
                }
            }
        });
        let end_stream =
            stream::once(async { Ok::<_, actix_web::Error>(web::Bytes::from("data: [DONE]\n\n")) });
        let final_stream = stream.chain(end_stream);
        Ok(Either::Right(
            HttpResponseBuilder::new(StatusCode::OK)
                .content_type("text/event-stream")
                .append_header(("Cache-Control", "no-cache"))
                .append_header(("Connection", "keep-alive"))
                .streaming(final_stream),
        ))
    } else {
        Ok(Either::Left(web::Json(parsed_response)))
    }
}

use crate::AppState;
use actix_web::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{Either, Error, HttpResponse, error::ErrorInternalServerError, post, web};
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::iter::Iterator;
use straico_client::chat::{Chat, Tool};
use straico_client::endpoints::completion::completion_request::CompletionRequest;
use straico_client::endpoints::completion::completion_response::{
    Choice, Completion, Message, ToolCall, Usage,
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
    #[serde(default = "default_streaming")]
    stream: bool,
    /// List of tools/functions available to the model during completion
    tools: Option<Vec<Tool>>,
}

fn default_streaming() -> bool {
    true
}

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

#[derive(Serialize, Debug)]
pub struct CompletionStream {
    choices: Vec<ChoiceStream>,
    object: Box<str>,
    id: Box<str>,
    model: Box<str>,
    created: u64,
    usage: Usage,
}
#[derive(Serialize, Debug)]
pub struct ChoiceStream {
    index: u8,
    delta: Delta,
    finish_reason: Option<Box<str>>,
}

#[derive(Serialize, Debug)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
}

pub struct DeltaIterator<T, I, U> {
    role: T,
    content: Option<I>,
    tool_calls: Option<U>,
}

pub struct ChoiceStreamIterator<T, I, U> {
    index: u8,
    delta: DeltaIterator<T, I, U>,
    finish_reason: Option<Box<str>>,
}

pub struct CompletionStreamIterator<T, I, U> {
    choices: Vec<ChoiceStreamIterator<T, I, U>>,
    object: Box<str>,
    id: Box<str>,
    model: Box<str>,
    created: u64,
    usage: Usage,
}

impl IntoIterator for Delta {
    type Item = Delta;
    type IntoIter = DeltaIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        DeltaIterator {
            role: vec![self.role.unwrap()].into_iter(),
            content: self.content.map(|c| {
                c.split_inclusive(' ')
                    .map(Box::from)
                    .collect::<Vec<Box<_>>>()
                    .into_iter()
            }),
            tool_calls: self.tool_calls.map(|t| vec![t].into_iter()),
        }
    }
}

impl IntoIterator for ChoiceStream {
    type Item = ChoiceStream;
    type IntoIter = ChoiceStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        ChoiceStreamIterator {
            index: self.index,
            delta: self.delta.into_iter(),
            finish_reason: self.finish_reason,
        }
    }
}

impl IntoIterator for CompletionStream {
    type Item = CompletionStream;
    type IntoIter = CompletionStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        CompletionStreamIterator {
            choices: self.choices.into_iter().map(|x| x.into_iter()).collect(),
            object: self.object,
            id: self.id,
            model: self.model,
            created: self.created,
            usage: self.usage,
        }
    }
}

impl<I, T, U> Iterator for DeltaIterator<I, T, U>
where
    I: Iterator<Item = Box<str>>,
    T: Iterator<Item = Box<str>>,
    U: Iterator<Item = Vec<ToolCall>>,
{
    type Item = Delta;

    fn next(&mut self) -> Option<Self::Item> {
        let delta = Delta {
            role: self.role.next(),
            //content: match &mut self.content {
            //    Some(c) => c.next(),
            //    None => None,
            //},
            content: self.content.as_mut().and_then(Iterator::next),
            //.map(Iterator::next).flatten(),
            tool_calls: self.tool_calls.as_mut().and_then(Iterator::next),
            //.map(Iterator::next).flatten(),
            //tool_calls: match &mut self.tool_calls {
            //    Some(t) => t.next(),
            //    None => None,
            //},
        };
        if delta.content.is_none() && delta.tool_calls.is_none() {
            None
        } else {
            Some(delta)
        }
    }
}

impl Iterator
    for ChoiceStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >
{
    type Item = ChoiceStream;

    fn next(&mut self) -> Option<Self::Item> {
        let choice = ChoiceStream {
            index: self.index,
            delta: self.delta.next()?,
            finish_reason: self.finish_reason.clone(),
        };
        Some(choice)
    }
}

impl Iterator
    for CompletionStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >
{
    type Item = CompletionStream;

    fn next(&mut self) -> Option<Self::Item> {
        let completion = CompletionStream {
            choices: self
                .choices
                .iter_mut()
                .map(|x| x.next())
                .collect::<Option<Vec<ChoiceStream>>>()?,
            object: self.object.clone(),
            id: self.id.clone(),
            model: self.model.clone(),
            created: self.created,
            usage: self.usage.clone(),
        };
        Some(completion)
    }
}

impl From<Message> for Delta {
    fn from(value: Message) -> Self {
        //let content_to_string = |content: Content| match content {
        //    Content::Text(text) => Some(text),
        //    Content::TextArray(texts) => Some(Box::from(texts.into_iter().map(|t| t.text).collect::<Vec<_>>().join(" "))),
        //};

        match value {
            Message::User { content } => Delta {
                role: Some("user".into()),
                content: Some(content.to_string().into_boxed_str()),
                tool_calls: None,
            },
            Message::Assistant {
                content,
                tool_calls,
            } => Delta {
                role: Some("assistant".into()),
                content: content.map(|c| c.to_string().into_boxed_str()),
                tool_calls,
            },
            Message::System { content } => Delta {
                role: Some("system".into()),
                content: Some(content.to_string().into_boxed_str()),
                tool_calls: None,
            },
            Message::Tool { content } => Delta {
                role: Some("function".into()),
                content: Some(content.to_string().into_boxed_str()),
                tool_calls: None,
            },
        }
    }
}

impl From<Choice> for ChoiceStream {
    fn from(value: Choice) -> Self {
        Self {
            index: value.index,
            delta: value.message.into(),
            finish_reason: Some(value.finish_reason),
        }
    }
}

impl From<Completion> for CompletionStream {
    fn from(value: Completion) -> Self {
        Self {
            choices: value.choices.into_iter().map(Into::into).collect(),
            object: value.object,
            id: value.id,
            model: value.model,
            created: value.created,
            usage: value.usage,
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
    if data.debug {
        eprintln!("\n\n===== Request recieved: =====");
        eprintln!("\n{}", serde_json::to_string_pretty(&req_inner)?);
    }
    let client = data.client.clone();

    let req_inner_oa: OpenAiRequest = serde_json::from_value(req_inner)?;
    let stream = req_inner_oa.stream;
    let response = client
        .completion()
        .bearer_auth(&data.key)
        // .json(req_inner)
        .json(req_inner_oa)
        .send()
        .await
        .map_err(ErrorInternalServerError)?
        .get_completion()
        .map_err(ErrorInternalServerError)?;

    if data.debug {
        eprintln!("\n\n===== Received response: =====");
        eprintln!("\n{}", serde_json::to_string_pretty(&response)?);
    }

    let parsed_response = response.parse().map_err(ErrorInternalServerError)?;

    if stream {
        let i = CompletionStream::from(parsed_response);
        let stream = stream::iter(i).map(|chunk| {
            let json = serde_json::to_string(&chunk).unwrap();
            Ok::<_, actix_web::Error>(web::Bytes::from(format!("data: {}\n\n", json)))
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

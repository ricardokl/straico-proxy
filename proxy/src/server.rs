use crate::{
    error::CustomError, types::OpenAiChatRequest, streaming::CompletionStream,
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
    req: web::Json<OpenAiChatRequest>,
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

    let response =
        serde_json::from_slice::<straico_client::endpoints::chat::chat_response::ChatResponse>(
            &response_bytes,
        )
        .map_err(|e| CustomError::SerdeJson(e))?;

    if openai_request.stream {
        let stream_iterator = CompletionStream::from(response).into_iter();
        let stream = stream::iter(stream_iterator)
            .map(|chunk| {
                let json = serde_json::to_string(&chunk).unwrap();
                Ok::<_, CustomError>(Bytes::from(format!("data: {json}\n\n")))
            })
            .chain(stream::once(async {
                Ok(Bytes::from("data: [DONE]\n\n"))
            }));

        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        Ok(HttpResponse::Ok().json(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ChatContent, OpenAiChatMessage, OpenAiToolCall, ChatFunctionCall
    };
    use actix_web::{test, web, App};
    use straico_client::endpoints::chat::chat_response::{
        ChatChoice, ChatResponse, ChatResponseContent, Message,
    };
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[actix_rt::test]
    async fn test_openai_chat_completion_streaming() {
        // Arrange
        let server = MockServer::start().await;
        let mock_response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-3.5-turbo-0125".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            choices: vec![ChatChoice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(ChatResponseContent::Text("Hello there!".to_string())),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Default::default(),
            price: Default::default(),
            words: Default::default(),
        };
        Mock::given(method("POST"))
            .and(path("/v0/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        let client = StraicoClient::with_base_url(server.uri());

        let app_state = web::Data::new(AppState {
            client,
            key: "test_key".to_string(),
            debug: false,
            log: false,
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(openai_chat_completion),
        )
        .await;
        let req_payload = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![OpenAiChatMessage {
                role: "user".to_string(),
                content: Some(ChatContent::String("Hello".to_string())),
                tool_call_id: None,
                name: None,
                tool_calls: None,
            }],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: true,
            tools: None,
            tool_choice: None,
        };
        let req = test::TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(&req_payload)
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let mut lines = body_str.lines().filter(|&l| !l.is_empty());

        let first_line = lines.next().unwrap();
        assert!(first_line.starts_with("data: "));
        let first_chunk_json: serde_json::Value =
            serde_json::from_str(&first_line["data: ".len()..]).unwrap();
        assert_eq!(
            first_chunk_json["choices"][0]["delta"]["role"], "assistant"
        );
        assert!(first_chunk_json["choices"][0]["delta"]["content"].is_null());

        let second_line = lines.next().unwrap();
        assert!(second_line.starts_with("data: "));
        let second_chunk_json: serde_json::Value =
            serde_json::from_str(&second_line["data: ".len()..]).unwrap();
        assert!(second_chunk_json["choices"][0]["delta"]["role"].is_null());
        assert_eq!(
            second_chunk_json["choices"][0]["delta"]["content"], "Hello there!"
        );

        assert_eq!(lines.next().unwrap(), "data: [DONE]");
    }

    #[actix_rt::test]
    async fn test_openai_chat_completion_with_tool_calls() {
        // Arrange
        let server = MockServer::start().await;
        let mock_response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-3.5-turbo-0125".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            choices: vec![ChatChoice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(ChatResponseContent::Text("Hello there!".to_string())),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Default::default(),
            price: Default::default(),
            words: Default::default(),
        };
        Mock::given(method("POST"))
            .and(path("/v0/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        let client = StraicoClient::with_base_url(server.uri());

        let app_state = web::Data::new(AppState {
            client,
            key: "test_key".to_string(),
            debug: false,
            log: false,
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(openai_chat_completion),
        )
        .await;
        let req_payload = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![OpenAiChatMessage {
                role: "assistant".to_string(),
                content: None,
                tool_call_id: None,
                name: None,
                tool_calls: Some(vec![OpenAiToolCall {
                    id: "call_123".to_string(),
                    tool_type: "function".to_string(),
                    function: ChatFunctionCall {
                        name: "get_weather".to_string(),
                        arguments: "{\"location\": \"London\"}".to_string(),
                    },
                }]),
            }],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        };
        let req = test::TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(&req_payload)
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert!(resp.status().is_success());
        let received_requests = server.received_requests().await.unwrap();
        let received_request = received_requests.get(0).unwrap();
        let received_body: serde_json::Value =
            serde_json::from_slice(&received_request.body).unwrap();

        let expected_tool_calls = serde_json::to_string(&req_payload.messages[0]
            .tool_calls
            .clone()
            .unwrap())
        .unwrap();

        assert_eq!(
            received_body["messages"][0]["content"][0]["text"],
            "<tool_calls>"
        );
        assert_eq!(
            received_body["messages"][0]["content"][1]["text"],
            expected_tool_calls
        );
        assert_eq!(
            received_body["messages"][0]["content"][2]["text"],
            "</tool_calls>"
        );
    }

    #[actix_rt::test]
    async fn test_openai_chat_completion_with_tool_role() {
        // Arrange
        let server = MockServer::start().await;
        let mock_response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-3.5-turbo-0125".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            choices: vec![ChatChoice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(ChatResponseContent::Text("Hello there!".to_string())),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Default::default(),
            price: Default::default(),
            words: Default::default(),
        };
        Mock::given(method("POST"))
            .and(path("/v0/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        let client = StraicoClient::with_base_url(server.uri());

        let app_state = web::Data::new(AppState {
            client,
            key: "test_key".to_string(),
            debug: false,
            log: false,
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(openai_chat_completion),
        )
        .await;
        let req_payload = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![OpenAiChatMessage {
                role: "tool".to_string(),
                content: Some(ChatContent::String("{\"result\": \"success\"}".to_string())),
                tool_call_id: Some("call_123".to_string()),
                name: None,
                tool_calls: None,
            }],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        };
        let req = test::TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(&req_payload)
            .to_request();

        // Act
        let resp = test::call_service(&app, req).await;

        // Assert
        assert!(resp.status().is_success());
        let received_requests = server.received_requests().await.unwrap();
        let received_request = received_requests.get(0).unwrap();
        let received_body: serde_json::Value =
            serde_json::from_slice(&received_request.body).unwrap();

        assert_eq!(received_body["messages"][0]["role"], "user");
        let expected_json = serde_json::json!({
            "tool_call_id": "call_123",
            "output": "{\"result\": \"success\"}"
        });

        let actual_content_str = received_body["messages"][0]["content"][0]["text"]
            .as_str()
            .unwrap();
        let actual_json_str = actual_content_str
            .strip_prefix("<tool_output>")
            .unwrap()
            .strip_suffix("</tool_output>")
            .unwrap();
        let actual_json: serde_json::Value = serde_json::from_str(actual_json_str).unwrap();

        assert_eq!(actual_json, expected_json);
    }
}

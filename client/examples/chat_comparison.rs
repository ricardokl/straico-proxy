use straico_client::{
    chat::{Message,},
    endpoints::chat::{ChatClientExt, ChatMessage, ChatRequest, ChatResponseExt},
    StraicoClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StraicoClient::new();
    let api_key =
        std::env::var("STRAICO_API_KEY").expect("STRAICO_API_KEY environment variable must be set");

    let user_question = "What are the main advantages of Rust programming language?";

    println!("=== Chat Endpoint Example ===\n");
    println!("Question: {}\n", user_question);

    let chat_request = ChatRequest::builder()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a helpful programming expert."))
        .message(ChatMessage::user(user_question))
        .temperature(0.7)
        .max_tokens(200)
        .build();

    let chat_response = client
        .chat()
        .bearer_auth(&api_key)
        .json(chat_request.clone())
        .send()
        .await?;

    let chat_data = chat_response.get_chat_response()?;

    if let Some(content) = chat_data.first_content() {
        println!("Response: {}", content);
    }

    if let Some(usage) = &chat_data.usage {
        println!("Tokens: {} total\n", usage.total_tokens);
    }

    println!("\n=== Example completed! ===");
    Ok(())
}

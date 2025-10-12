use straico_client::{
    StraicoClient,
    endpoints::chat::{
        ChatMessage, ChatRequest, ContentObject,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StraicoClient::new();
    let api_key =
        std::env::var("STRAICO_API_KEY").expect("STRAICO_API_KEY environment variable must be set");

    println!("=== Straico Chat Endpoint Examples ===\n");

    println!("1. Simple Chat Request:");
    let simple_request =
    ChatRequest::builder()
    .model("gpt-3.5-turbo")
    .message(ChatMessage::user("Hello! Can you explain what Rust is?"))
    .build();

    let response = client
        .clone()
        .chat()
        .bearer_auth(&api_key)
        .json(simple_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = chat_response.first_content() {
        println!("Response: {content}\n");
    }

    println!("2. System + User Message:");
    let system_user_request = ChatRequest::builder()
    .model("gpt-3.5-turbo")
    .message(ChatMessage::system("You are a helpful programming tutor. Explain concepts clearly and provide examples."))
    .message(ChatMessage::user("What are the main benefits of Rust's ownership system?"))
    .build();

    let response = client
        .clone()
        .chat()
        .bearer_auth(&api_key)
        .json(system_user_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = chat_response.first_content() {
        println!("Response: {content}\n");
    }

    println!("3. Multi-turn Conversation:");
    let conversation_messages = vec![
        ChatMessage::system("You are a helpful assistant that provides concise answers."),
        ChatMessage::user("What is the capital of France?"),
        ChatMessage::assistant("The capital of France is Paris."),
        ChatMessage::user("What's the population of that city?"),
    ];

    let conversation_request = ChatRequest::builder()
    .model("gpt-3.5-turbo")
    .messages(conversation_messages)
    .build();

    let response = client
        .clone()
        .chat()
        .bearer_auth(&api_key)
        .json(conversation_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = chat_response.first_content() {
        println!("Response: {content}\n");
    }

    println!("4. Advanced Request with Parameters:");
    let advanced_messages = vec![
        ChatMessage::system("You are a creative writing assistant."),
        ChatMessage::user("Write a short poem about programming."),
    ];

    let advanced_request =
    ChatRequest::builder()
    .model("gpt-3.5-turbo")
    .messages(advanced_messages)
    .temperature(0.8)
    .max_tokens(150)
    .build();

    let response = client
        .clone()
        .chat()
        .bearer_auth(&api_key)
        .json(advanced_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = chat_response.first_content() {
        println!("Response: {content}\n");
    }

    println!("5. Direct Builder Pattern:");
    let builder_request = ChatRequest::builder()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a helpful assistant."))
        .message(ChatMessage::user(
            "Explain quantum computing in simple terms.",
        ))
        .temperature(0.3)
        .max_tokens(200)
        .build();

    let response = client
        .clone()
        .chat()
        .bearer_auth(&api_key)
        .json(builder_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;

    println!("Response analysis:");
    println!(
        "- Content: {}",
        chat_response.first_content().unwrap_or("No content".to_string())
    );
    println!(
        "- Finish reason: {}",
        chat_response.first_choice().map(|c| c.finish_reason.as_str()).unwrap_or("Unknown")
    );
    println!("- Was truncated: {}", chat_response.first_choice().map(|c| c.finish_reason == "length").unwrap_or(false));
    println!("- Has tool calls: {}", chat_response.has_tool_calls());

    if let Some(usage) = &chat_response.usage {
        println!(
            "- Token usage: {} prompt + {} completion = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    println!("\n6. Structured Content Objects:");
    let structured_message = ChatMessage::new(
        "user",
        vec![
            ContentObject::text("Please analyze this text: "),
            ContentObject::text("Rust is a systems programming language."),
        ],
    );

    let structured_request = ChatRequest::builder()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a text analysis expert."))
        .message(structured_message)
        .build();

    let response = client
        .chat()
        .bearer_auth(&api_key)
        .json(structured_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = chat_response.first_content() {
        println!("Analysis: {content}\n");
    }

    println!("=== All examples completed successfully! ===");
    Ok(())
}

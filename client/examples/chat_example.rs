use straico_client::{
    StraicoClient,
    endpoints::chat::{
        ChatRequest, ChatMessage, ContentObject,
        ChatClientExt, ChatResponseExt,
        builders::*,
        response_utils::*,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client
    let client = StraicoClient::new();
    let api_key = std::env::var("STRAICO_API_KEY")
        .expect("STRAICO_API_KEY environment variable must be set");

    println!("=== Straico Chat Endpoint Examples ===\n");

    // Example 1: Simple chat request
    println!("1. Simple Chat Request:");
    let simple_request = simple_chat_request(
        "gpt-3.5-turbo",
        "Hello! Can you explain what Rust is?"
    );

    let response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(simple_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = get_first_content(&chat_response) {
        println!("Response: {}\n", content);
    }

    // Example 2: System + User message
    println!("2. System + User Message:");
    let system_user_request = system_user_chat_request(
        "gpt-3.5-turbo",
        "You are a helpful programming tutor. Explain concepts clearly and provide examples.",
        "What are the main benefits of Rust's ownership system?"
    );

    let response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(system_user_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = get_first_content(&chat_response) {
        println!("Response: {}\n", content);
    }

    // Example 3: Conversation with multiple messages
    println!("3. Multi-turn Conversation:");
    let conversation_messages = vec![
        ChatMessage::system("You are a helpful assistant that provides concise answers."),
        ChatMessage::user("What is the capital of France?"),
        ChatMessage::assistant("The capital of France is Paris."),
        ChatMessage::user("What's the population of that city?"),
    ];

    let conversation_request = conversation_chat_request(
        "gpt-3.5-turbo",
        conversation_messages
    );

    let response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(conversation_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = get_first_content(&chat_response) {
        println!("Response: {}\n", content);
    }

    // Example 4: Advanced request with parameters
    println!("4. Advanced Request with Parameters:");
    let advanced_messages = vec![
        ChatMessage::system("You are a creative writing assistant."),
        ChatMessage::user("Write a short poem about programming."),
    ];

    let advanced_request = advanced_chat_request(
        "gpt-3.5-turbo",
        advanced_messages,
        Some(0.8), // Higher temperature for creativity
        Some(150), // Limit tokens
    );

    let response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(advanced_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = get_first_content(&chat_response) {
        println!("Response: {}\n", content);
    }

    // Example 5: Using builder pattern directly
    println!("5. Direct Builder Pattern:");
    let builder_request = ChatRequest::new()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a helpful assistant."))
        .message(ChatMessage::user("Explain quantum computing in simple terms."))
        .temperature(0.3)
        .max_tokens(200)
        .build();

    let response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(builder_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    
    // Demonstrate response utilities
    println!("Response analysis:");
    println!("- Content: {}", get_first_content(&chat_response).unwrap_or("No content".to_string()));
    println!("- Finish reason: {}", get_finish_reason(&chat_response).unwrap_or("Unknown"));
    println!("- Was truncated: {}", was_truncated(&chat_response));
    println!("- Has tool calls: {}", has_tool_calls(&chat_response));
    
    if let Some(usage) = get_usage(&chat_response) {
        println!("- Token usage: {} prompt + {} completion = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    // Example 6: Structured content objects
    println!("\n6. Structured Content Objects:");
    let structured_message = ChatMessage::new(
        "user",
        vec![
            ContentObject::text("Please analyze this text: "),
            ContentObject::text("Rust is a systems programming language."),
        ]
    );

    let structured_request = ChatRequest::new()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a text analysis expert."))
        .message(structured_message)
        .build();

    let response = client
        .chat_completions()
        .bearer_auth(&api_key)
        .json(structured_request)
        .send()
        .await?;

    let chat_response = response.get_chat_response()?;
    if let Some(content) = get_first_content(&chat_response) {
        println!("Analysis: {}\n", content);
    }

    println!("=== All examples completed successfully! ===");
    Ok(())
}
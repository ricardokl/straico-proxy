use straico_client::{
    StraicoClient,
    endpoints::{
        completion::{CompletionRequest, completion_response::Completion},
        chat::{ChatRequest, ChatMessage, ChatClientExt, ChatResponseExt},
    },
    chat::{Chat, Message},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StraicoClient::new();
    let api_key = std::env::var("STRAICO_API_KEY")
        .expect("STRAICO_API_KEY environment variable must be set");

    let user_question = "What are the main advantages of Rust programming language?";
    
    println!("=== Comparison: Completion vs Chat Endpoints ===\n");
    println!("Question: {}\n", user_question);

    // Example 1: Using the old completion endpoint
    println!("1. OLD COMPLETION ENDPOINT:");
    println!("---------------------------");
    
    let old_messages = Chat(vec![
        Message::System { 
            content: "You are a helpful programming expert.".into() 
        },
        Message::User { 
            content: user_question.into() 
        },
    ]);

    let completion_request = CompletionRequest::new()
        .models("gpt-3.5-turbo")
        .message(old_messages.to_prompt(None, "gpt-3.5-turbo"))
        .temperature(0.7)
        .max_tokens(200)
        .build();

    let completion_response = client
        .clone()
        .completion()
        .bearer_auth(&api_key)
        .json(completion_request)
        .send()
        .await?;

    let completion_data = completion_response.get_completion()?;
    let parsed_completion = completion_data.parse()?;
    
    println!("Response: {}", parsed_completion.choices[0].message.content);
    println!("Tokens: {} total\n", parsed_completion.usage.total_tokens);

    // Example 2: Using the new chat endpoint
    println!("2. NEW CHAT ENDPOINT:");
    println!("---------------------");
    
    let chat_request = ChatRequest::new()
        .model("gpt-3.5-turbo")
        .message(ChatMessage::system("You are a helpful programming expert."))
        .message(ChatMessage::user(user_question))
        .temperature(0.7)
        .max_tokens(200)
        .build();

    let chat_response = client
        .clone()
        .chat_completions()
        .bearer_auth(&api_key)
        .json(chat_request)
        .send()
        .await?;

    let chat_data = chat_response.get_chat_response()?;
    
    if let Some(content) = chat_data.first_content() {
        println!("Response: {}", content);
    }
    
    if let Some(usage) = &chat_data.usage {
        println!("Tokens: {} total\n", usage.total_tokens);
    }

    // Example 3: Demonstrate content format differences
    println!("3. CONTENT FORMAT COMPARISON:");
    println!("-----------------------------");
    
    // Old format: Single prompt string
    println!("Old format (prompt string):");
    let old_prompt = old_messages.to_prompt(None, "gpt-3.5-turbo");
    println!("Prompt: {}\n", old_prompt.message);
    
    // New format: Structured messages
    println!("New format (structured messages):");
    for (i, message) in chat_request.messages.iter().enumerate() {
        println!("Message {}: role='{}', content={:?}", 
            i + 1, message.role, message.content);
    }
    
    println!("\n4. KEY DIFFERENCES:");
    println!("-------------------");
    println!("• Old endpoint: Single prompt string, model-specific formatting");
    println!("• New endpoint: Structured message arrays, consistent format");
    println!("• Old endpoint: Manual prompt construction with XML tool embedding");
    println!("• New endpoint: Native message structure with content objects");
    println!("• Old endpoint: Response parsing required for tool calls");
    println!("• New endpoint: Structured response with native tool call support");
    
    println!("\n=== Comparison completed! ===");
    Ok(())
}
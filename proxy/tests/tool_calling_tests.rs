use straico_proxy::openai_types::{
    OpenAiChatRequest, OpenAiContent, OpenAiToolChoice
};

#[test]
fn test_parse_example1_request() {
    let example_request = r#"
    {
      "model": "llama-3.3-70b-versatile",
      "messages": [
        {
          "role": "system",
          "content": "You are a weather assistant. Use the get_weather function to retrieve weather information for a given location."
        },
        {
          "role": "user",
          "content": "What's the weather like in New York today?"
        }
      ],
      "tools": [
        {
          "type": "function",
          "function": {
            "name": "get_weather",
            "description": "Get the current weather for a location",
            "parameters": {
              "type": "object",
              "properties": {
                "location": {
                  "type": "string",
                  "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                  "type": "string",
                  "enum": ["celsius", "fahrenheit"],
                  "description": "The unit of temperature to use. Defaults to fahrenheit."
                }
              },
              "required": ["location"]
            }
          }
        }
      ],
      "tool_choice": "auto",
      "max_completion_tokens": 4096
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(example_request).unwrap();

    // Verify basic request properties
    assert_eq!(request.model, "llama-3.3-70b-versatile");
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.max_completion_tokens, Some(4096));

    // Verify tools
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool_type, "function");
    assert_eq!(tools[0].function.name, "get_weather");
    assert_eq!(tools[0].function.description, Some("Get the current weather for a location".to_string()));

    // Verify tool choice
    assert!(request.tool_choice.is_some());
    match request.tool_choice.unwrap() {
        OpenAiToolChoice::String(choice) => {
            assert_eq!(choice, "auto");
        }
        _ => panic!("Expected string tool choice"),
    }

    // Verify messages
    let system_msg = &request.messages[0];
    assert_eq!(system_msg.role, "system");
    match &system_msg.content {
        OpenAiContent::String(text) => {
            assert_eq!(text, "You are a weather assistant. Use the get_weather function to retrieve weather information for a given location.");
        }
        _ => panic!("Expected string content"),
    }

    let user_msg = &request.messages[1];
    assert_eq!(user_msg.role, "user");
    match &user_msg.content {
        OpenAiContent::String(text) => {
            assert_eq!(text, "What's the weather like in New York today?");
        }
        _ => panic!("Expected string content"),
    }
}

#[test]
fn test_parse_example2_request() {
    let example_request = r#"
    {
      "model": "google/gemini-2.0-flash-001",
      "messages": [
        {
          "role": "user",
          "content": "What are the titles of some James Joyce books?"
        }
      ],
      "tools": [
        {
          "type": "function",
          "function": {
            "name": "search_gutenberg_books",
            "description": "Search for books in the Project Gutenberg library",
            "parameters": {
              "type": "object",
              "properties": {
                "search_terms": {
                  "type": "array",
                  "items": {"type": "string"},
                  "description": "List of search terms to find books"
                }
              },
              "required": ["search_terms"]
            }
          }
        }
      ]
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(example_request).unwrap();

    // Verify basic request properties
    assert_eq!(request.model, "google/gemini-2.0-flash-001");
    assert_eq!(request.messages.len(), 1);

    // Verify tools
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool_type, "function");
    assert_eq!(tools[0].function.name, "search_gutenberg_books");
    assert_eq!(tools[0].function.description, Some("Search for books in the Project Gutenberg library".to_string()));

    // Verify message
    let user_msg = &request.messages[0];
    assert_eq!(user_msg.role, "user");
    match &user_msg.content {
        OpenAiContent::String(text) => {
            assert_eq!(text, "What are the titles of some James Joyce books?");
        }
        _ => panic!("Expected string content"),
    }
}

#[test]
fn test_parse_example2_request_with_tool_results() {
    let example_request = r#"
    {
      "model": "google/gemini-2.0-flash-001",
      "messages": [
        {
          "role": "user",
          "content": "What are the titles of some James Joyce books?"
        },
        {
          "role": "assistant",
          "content": null,
          "tool_calls": [
            {
              "id": "call_abc123",
              "type": "function",
              "function": {
                "name": "search_gutenberg_books",
                "arguments": "{\"search_terms\": [\"James\", \"Joyce\"]}"
              }
            }
          ]
        },
        {
          "role": "tool",
          "tool_call_id": "call_abc123",
          "content": "[{\"id\": 4300, \"title\": \"Ulysses\", \"authors\": [{\"name\": \"Joyce, James\"}]}]"
        }
      ],
      "tools": [
        {
          "type": "function",
          "function": {
            "name": "search_gutenberg_books",
            "description": "Search for books in the Project Gutenberg library",
            "parameters": {
              "type": "object",
              "properties": {
                "search_terms": {
                  "type": "array",
                  "items": {"type": "string"},
                  "description": "List of search terms to find books"
                }
              },
              "required": ["search_terms"]
            }
          }
        }
      ]
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(example_request).unwrap();

    // Verify basic request properties
    assert_eq!(request.model, "google/gemini-2.0-flash-001");
    assert_eq!(request.messages.len(), 3);

    // Verify tools
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool_type, "function");
    assert_eq!(tools[0].function.name, "search_gutenberg_books");

    // Check first message (user)
    let user_msg = &request.messages[0];
    assert_eq!(user_msg.role, "user");
    match &user_msg.content {
        OpenAiContent::String(text) => {
            assert_eq!(text, "What are the titles of some James Joyce books?");
        }
        _ => panic!("Expected string content"),
    }

    // Check second message (assistant with tool calls)
    let assistant_msg = &request.messages[1];
    assert_eq!(assistant_msg.role, "assistant");
    match &assistant_msg.content {
        OpenAiContent::String(text) => {
            assert!(text.is_empty()); // null content becomes empty string
        }
        OpenAiContent::Array(objects) => {
            assert!(objects.is_empty()); // null content becomes empty array
        }
        &OpenAiContent::Null => {
            // null content is preserved as null
        }
    }

    // Verify tool calls exist
    assert!(assistant_msg.tool_calls.is_some());
    let tool_calls = assistant_msg.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    
    let tool_call = &tool_calls[0];
    assert_eq!(tool_call.id, "call_abc123");
    assert_eq!(tool_call.tool_type, "function");
    assert_eq!(tool_call.function.name, "search_gutenberg_books");
    assert_eq!(tool_call.function.arguments, "{\"search_terms\": [\"James\", \"Joyce\"]}");

    // Check third message (tool response)
    let tool_msg = &request.messages[2];
    assert_eq!(tool_msg.role, "tool");
    assert_eq!(tool_msg.tool_call_id, Some("call_abc123".to_string()));
    match &tool_msg.content {
        OpenAiContent::String(text) => {
            assert_eq!(text, "[{\"id\": 4300, \"title\": \"Ulysses\", \"authors\": [{\"name\": \"Joyce, James\"}]}]");
        }
        _ => panic!("Expected string content"),
    }
}

#[test]
fn test_tool_call_serialization_roundtrip() {
    let original_request = r#"
    {
      "model": "llama-3.3-70b-versatile",
      "messages": [
        {
          "role": "user",
          "content": "What's the weather like in New York today?"
        },
        {
          "role": "assistant",
          "content": null,
          "tool_calls": [
            {
              "id": "call_d5wg",
              "type": "function",
              "function": {
                "name": "get_weather",
                "arguments": "{\"location\": \"New York, NY\"}"
              }
            }
          ]
        }
      ],
      "tools": [
        {
          "type": "function",
          "function": {
            "name": "get_weather",
            "description": "Get the current weather for a location",
            "parameters": {
              "type": "object",
              "properties": {
                "location": {
                  "type": "string",
                  "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                  "type": "string",
                  "enum": ["celsius", "fahrenheit"],
                  "description": "The unit of temperature to use. Defaults to fahrenheit."
                }
              },
              "required": ["location"]
            }
          }
        }
      ],
      "tool_choice": "auto"
    }
    "#;

    // Parse the request
    let request: OpenAiChatRequest = serde_json::from_str(original_request).unwrap();

    // Serialize it back
    let serialized = serde_json::to_string_pretty(&request).unwrap();

    // Parse it again to verify round-trip
    let parsed_again: OpenAiChatRequest = serde_json::from_str(&serialized).unwrap();

    // Check that the roundtrip preserved the data
    assert_eq!(request.model, parsed_again.model);
    assert_eq!(request.messages.len(), parsed_again.messages.len());
    assert_eq!(request.tools.as_ref().map(|t| t.len()), parsed_again.tools.as_ref().map(|t| t.len()));
}

#[test]
fn test_tool_choice_variants() {
    // Test "none" string variant
    let request_with_none_choice = r#"
    {
      "model": "gpt-4",
      "messages": [
        {
          "role": "user",
          "content": "Hello"
        }
      ],
      "tool_choice": "none"
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(request_with_none_choice).unwrap();
    assert!(request.tool_choice.is_some());
    match request.tool_choice.unwrap() {
        OpenAiToolChoice::String(choice) => {
            assert_eq!(choice, "none");
        }
        _ => panic!("Expected string tool choice"),
    }

    // Test "required" string variant
    let request_with_required_choice = r#"
    {
      "model": "gpt-4",
      "messages": [
        {
          "role": "user",
          "content": "Hello"
        }
      ],
      "tool_choice": "required"
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(request_with_required_choice).unwrap();
    assert!(request.tool_choice.is_some());
    match request.tool_choice.unwrap() {
        OpenAiToolChoice::String(choice) => {
            assert_eq!(choice, "required");
        }
        _ => panic!("Expected string tool choice"),
    }
}

#[test]
fn test_content_null_handling() {
    // Test that null content is handled properly
    let request_with_null_content = r#"
    {
      "model": "gpt-4",
      "messages": [
        {
          "role": "assistant",
          "content": null,
          "tool_calls": [
            {
              "id": "call_123",
              "type": "function",
              "function": {
                "name": "test_function",
                "arguments": "{}"
              }
            }
          ]
        }
      ]
    }
    "#;

    let request: OpenAiChatRequest = serde_json::from_str(request_with_null_content).unwrap();
    let msg = &request.messages[0];

    // Verify the message has no content but has tool calls
    match &msg.content {
        OpenAiContent::String(s) => assert_eq!(s, ""), // null should become empty string
        OpenAiContent::Array(arr) => assert!(arr.is_empty()), // or empty array
        &OpenAiContent::Null => {
            // null content is preserved as null
        }
    }
    
    assert!(msg.tool_calls.is_some());
    let tool_calls = msg.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_123");
}
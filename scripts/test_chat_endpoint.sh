#!/bin/bash

# Manual testing script for new chat endpoint
# Usage: ./scripts/test_chat_endpoint.sh [host] [port]

HOST=${1:-localhost}
PORT=${2:-8000}
BASE_URL="http://${HOST}:${PORT}"

echo "=== Straico Proxy Chat Endpoint Testing ==="
echo "Testing against: ${BASE_URL}"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to test an endpoint
test_endpoint() {
    local test_name="$1"
    local data="$2"
    local expected_status="$3"
    
    echo -e "${YELLOW}Testing: ${test_name}${NC}"
    echo "Request: $data"
    
    response=$(curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "$data")
    
    # Extract status code (last line)
    status_code=$(echo "$response" | tail -n1)
    # Extract response body (all but last line)
    response_body=$(echo "$response" | head -n -1)
    
    echo "Status: $status_code"
    echo "Response: $response_body"
    
    if [[ "$status_code" == "$expected_status" ]]; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL (expected $expected_status, got $status_code)${NC}"
    fi
    echo "----------------------------------------"
}

echo "1. Testing basic string content..."
test_endpoint "Basic String Content" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "user", "content": "Hello, how are you?"}
    ]
}' "500"

echo ""
echo "2. Testing array content format..."
test_endpoint "Array Content Format" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {
            "role": "user", 
            "content": [
                {"type": "text", "text": "What is Rust programming language?"}
            ]
        }
    ]
}' "500"

echo ""
echo "3. Testing conversation with system message..."
test_endpoint "System + User Messages" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "system", "content": "You are a helpful programming assistant."},
        {"role": "user", "content": "Explain variables in Rust"}
    ],
    "temperature": 0.7,
    "max_tokens": 150
}' "500"

echo ""
echo "4. Testing multi-turn conversation..."
test_endpoint "Multi-turn Conversation" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What is 2+2?"},
        {"role": "assistant", "content": "2+2 equals 4."},
        {"role": "user", "content": "What about 3+3?"}
    ],
    "temperature": 0.5
}' "500"

echo ""
echo "5. Testing with all parameters..."
test_endpoint "Full Parameters" '{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Write a haiku about programming"}
    ],
    "temperature": 0.9,
    "max_tokens": 100,
    "stream": false
}' "500"

echo ""
echo "6. Testing mixed content formats..."
test_endpoint "Mixed Content Formats" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "user", "content": "String format message"},
        {"role": "assistant", "content": "Assistant response"},
        {
            "role": "user", 
            "content": [
                {"type": "text", "text": "Array format message"}
            ]
        }
    ]
}' "500"

echo ""
echo "7. Testing unicode and emoji content..."
test_endpoint "Unicode Content" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "user", "content": "Hello 👋 世界 🌍 How are you? 😊"}
    ]
}' "500"

echo ""
echo "8. Testing error cases..."

echo ""
echo "8a. Empty messages array..."
test_endpoint "Empty Messages" '{
    "model": "gpt-3.5-turbo",
    "messages": []
}' "400"

echo ""
echo "8b. Missing model..."
test_endpoint "Missing Model" '{
    "messages": [
        {"role": "user", "content": "Hello"}
    ]
}' "400"

echo ""
echo "8c. Invalid temperature..."
test_endpoint "Invalid Temperature" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {"role": "user", "content": "Hello"}
    ],
    "temperature": 3.0
}' "400"

echo ""
echo "8d. Invalid content type..."
test_endpoint "Invalid Content Type" '{
    "model": "gpt-3.5-turbo",
    "messages": [
        {
            "role": "user", 
            "content": [
                {"type": "image", "text": "This should fail"}
            ]
        }
    ]
}' "400"

echo ""
echo "9. Testing large content..."
large_content=$(python3 -c "print('Large content test. ' * 100)")
test_endpoint "Large Content" "{
    \"model\": \"gpt-3.5-turbo\",
    \"messages\": [
        {\"role\": \"user\", \"content\": \"$large_content\"}
    ]
}" "500"

echo ""
echo "10. Testing endpoint routing..."
echo "Testing legacy completion endpoint..."
curl -s -w "\nStatus: %{http_code}\n" -X POST "${BASE_URL}/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Legacy endpoint test"}
        ]
    }' | head -n 3

echo ""
echo "=== Testing Complete ==="
echo ""
echo "Note: Status 500 is expected for most tests due to invalid API key."
echo "Status 400 indicates request validation is working correctly."
echo "Status 404 would indicate routing problems."
echo ""
echo "To test with a real API key, set STRAICO_API_KEY environment variable"
echo "and restart the proxy server."
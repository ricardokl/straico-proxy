# Straico Proxy

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/ricardokl/straico-proxy)](https://github.com/ricardo-jorge/straico-proxy/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/ricardokl/straico-proxy/release.yml)](https://github.com/ricardo-jorge/straico-proxy/actions)

A proxy server that enables tool calling and streaming for Straico API, with format conversions to bridge compatibility gaps. "Mileage may vary" as conversions between different API formats are involved.

## Features

- **Tool calling support** for Straico API
- **Streaming simulation** with heartbeat chunks until response arrives
- **Format conversions** between OpenAI and Straico API formats
- Simple configuration through environment variables

## Straico API

The proxy can convert between OpenAI format and Straico's API format. Straico's chat endpoint:

- https://api.straico.com/v2/chat/completions

### Available Parameters

- model: String
- temperature: number
- max_tokens: number
- messages: Object with role and content fields

## Installation

### Pre-compiled Binary (Recommended)

Download the latest release from GitHub for your platform:

#### Linux (GNU)

##### Linux x86_64
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-linux-gnu.tar.gz | tar -xz
```

##### Linux ARM64
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-linux-gnu.tar.gz | tar -xz
```

#### macOS

##### macOS x86_64 (Intel)
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-apple-darwin.tar.gz | tar -xz
```

##### macOS ARM64 (Apple Silicon)
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-apple-darwin.tar.gz | tar -xz
```

#### Windows

##### Windows x86_64
```powershell
Invoke-WebRequest -Uri "https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-pc-windows-msvc.zip" -OutFile "straico-proxy.zip"
Expand-Archive -Path "straico-proxy.zip" -DestinationPath "."
```

##### Windows ARM64
```powershell
Invoke-WebRequest -Uri "https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-pc-windows-msvc.zip" -OutFile "straico-proxy.zip"
Expand-Archive -Path "straico-proxy.zip" -DestinationPath "."
```

#### Manual Download

Or download manually for your platform from:
https://github.com/ricardokl/straico-proxy/releases/latest

### From Source (Alternative)

If you prefer to compile from source or need a specific version:

```bash
cargo install --path ./proxy/
```

## Usage

Start the proxy server for Straico API with tool calling and streaming:

```bash
straico-proxy
```

## API Keys

Set your Straico API key via environment variable:

- `STRAICO_API_KEY` - For Straico requests

## Example Request

### Basic Chat

```bash
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama-3.1-70b",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'
```

### Tool Calling

```bash
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama-3.3-70b-versatile",
    "messages": [
      {
        "role": "system",
        "content": "You are a weather assistant. Use the get_weather function to retrieve weather information for a given location."
      },
      {
        "role": "user",
        "content": "What\'s the weather like in New York today?"
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
  }'
```
## License

MIT

# Straico Proxy

A proxy server that enables tool calling and streaming for Straico API, with format conversions to bridge compatibility gaps. "Mileage may vary" as conversions between different API formats are involved.

Router functionality is included as an extra feature for multi-provider support.

## Features

- **Tool calling support** for Straico API
- **Streaming capabilities** for real-time responses
- **Format conversions** between OpenAI and Straico API formats
- **Multi-provider routing** (extra feature) for SambaNova, Cerebras, Groq
- Simple configuration through environment variables

## Installation

```bash
cargo install --path .
```

## Usage

### Basic Mode (Default)

Start the proxy server for Straico API with tool calling and streaming:

```bash
straico-proxy
```

### Router Mode (Extra Feature)

Enable router mode to route requests to different providers based on model prefix:

```bash
straico-proxy --router
```

## Supported Providers

The router supports the following providers:

- **straico** - Routes to Straico API (with response conversion)
- **sambanova** - Routes to SambaNova API  
- **cerebras** - Routes to Cerebras API
- **groq** - Routes to Groq API

## Model Format

When using router mode, prefix your model name with the provider:

```
<provider>/<model-name>
```

Examples:
- `straico/gpt-4`
- `groq/llama-3.1-70b`
- `sambanova/Meta-Llama-3.1-8B-Instruct`
- `cerebras/llama3.1-8b`

## API Keys

Each provider requires its own API key set via environment variables:

- `STRAICO_API_KEY` - For Straico requests
- `SAMBANOVA_API_KEY` - For SambaNova requests
- `CEREBRAS_API_KEY` - For Cerebras requests
- `GROQ_API_KEY` - For Groq requests

## Example Request

```bash
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "groq/llama-3.1-70b",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'
```

## Straico API

The proxy can convert between OpenAI format and Straico's API format. Straico's chat endpoint:

- https://api.straico.com/v2/chat/completions

### Available Parameters

- model: String
- temperature: number
- max_tokens: number
- messages: Object with role and content fields

## Development

```bash
cargo build
cargo test
```

## License

MIT

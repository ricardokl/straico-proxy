# Straico Proxy

A proxy server that enables tool calling and streaming for Straico API, with format conversions to bridge compatibility gaps. "Mileage may vary" as conversions between different API formats are involved.

Router functionality is included as an extra feature for multi-provider support.

## Features

- **Tool calling support** for Straico API
- **Streaming capabilities** for real-time responses
- **Format conversions** between OpenAI and Straico API formats
- **Multi-provider routing** (extra feature) for SambaNova, Cerebras, Groq
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

#### Linux
```bash
# Linux x86_64
curl -L https://github.com/ricardo-jorge/straico-proxy/releases/latest/download/straico-proxy-linux-x86_64.tar.gz | tar xz
sudo mv straico-proxy /usr/local/bin/

# Linux ARM64
curl -L https://github.com/ricardo-jorge/straico-proxy/releases/latest/download/straico-proxy-linux-arm64.tar.gz | tar xz
sudo mv straico-proxy /usr/local/bin/
```

#### macOS
```bash
# macOS x86_64 (Intel)
curl -L https://github.com/ricardo-jorge/straico-proxy/releases/latest/download/straico-proxy-darwin-x86_64.tar.gz | tar xz
sudo mv straico-proxy /usr/local/bin/

# macOS ARM64 (Apple Silicon)
curl -L https://github.com/ricardo-jorge/straico-proxy/releases/latest/download/straico-proxy-darwin-arm64.tar.gz | tar xz
sudo mv straico-proxy /usr/local/bin/
```

#### Windows
```powershell
# Windows x86_64
Invoke-WebRequest -Uri "https://github.com/ricardo-jorge/straico-proxy/releases/latest/download/straico-proxy-windows-x86_64.zip" -OutFile "straico-proxy.zip"
Expand-Archive -Path "straico-proxy.zip" -DestinationPath "."
# Add to PATH or move straico-proxy.exe to a directory in your PATH
```

#### Manual Download

Or download manually for your platform from:
https://github.com/ricardo-jorge/straico-proxy/releases/latest

### From Source (Alternative)

If you prefer to compile from source or need a specific version:

```bash
cargo install --path ./proxy/
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
## License

MIT

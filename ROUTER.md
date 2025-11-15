# Router Mode

The proxy now supports a router mode that can route requests to different AI providers based on the model prefix.

## Usage

Enable router mode with the `--router` flag:

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

## How It Works

1. The router parses the provider prefix from the model name
2. Retrieves the appropriate API key from environment variables
3. Routes the request to the correct provider endpoint
4. For Straico: Converts between OpenAI and Straico formats (streaming and non-streaming)
5. For other providers: Passes through the request directly (stripping the provider prefix from model name)

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

The router will automatically route this to Groq's API using the `GROQ_API_KEY`.

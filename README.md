# Straico Proxy

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/ricardokl/straico-proxy)](https://github.com/ricardo-jorge/straico-proxy/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/ricardokl/straico-proxy/release.yml)](https://github.com/ricardo-jorge/straico-proxy/actions)

A proxy server that enables tool calling and streaming for Straico API, with format conversions to bridge compatibility gaps. "Mileage may vary" as conversions between different API formats are involved.

<details open>
<summary><h2>🚀 Quickstart (x86_64)</h2></summary>

Follow these 2 steps for your platform to get started. For more detailed instructions (ARM64, source build, PATH config), see the [detailed sections below](#-advanced-installation).

<details>
<summary><b>🐧 Linux</b></summary>

1. Download the proxy:
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-linux-gnu.tar.gz | tar -xz
```

2. Run the proxy:
```bash
./straico-proxy
```
</details>

<details>
<summary><b>🍎 macOS</b></summary>

1. Download the proxy:
```bash
curl -L https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-apple-darwin.tar.gz | tar -xz
```

2. Run the proxy:
```bash
./straico-proxy
```
</details>

<details>
<summary><b>🪟 Windows</b></summary>

1. Download the proxy:
```powershell
Invoke-WebRequest -Uri "https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-pc-windows-msvc.zip" -OutFile "straico-proxy.zip"
Expand-Archive -Path "straico-proxy.zip" -DestinationPath "."
```

2. Run the proxy:
```powershell
.\straico-proxy.exe
```
</details>

</details>

<details>
<summary><h2>✨ Features</h2></summary>

- **Tool calling emulation** for Straico API
- **Streaming simulation** with heartbeat chunks until response arrives
- **Format conversions** between OpenAI and Straico API formats
- **HTTPS support** with auto-generated self-signed certificates or custom certificates
- Simple configuration through environment variables
</details>

<details>
<summary><h2>🔌 Straico API</h2></summary>

The proxy can convert between OpenAI format and Straico's API format. Straico's chat endpoint:

- `https://api.straico.com/v2/chat/completions`

### Available Parameters

- model: String
- temperature: number
- max_tokens: number
- messages: Object with role and content fields
</details>

<details>
<summary><h2>📦 Advanced Installation</h2></summary>

### Pre-compiled Binaries

Download for your specific architecture:

#### Linux (GNU)
- [Linux x86_64](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-linux-gnu.tar.gz)
- [Linux ARM64](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-linux-gnu.tar.gz)

#### macOS
- [macOS x86_64 (Intel)](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-apple-darwin.tar.gz)
- [macOS ARM64 (Apple Silicon)](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-apple-darwin.tar.gz)

#### Windows
- [Windows x86_64](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-x86_64-pc-windows-msvc.zip)
- [Windows ARM64](https://github.com/ricardokl/straico-proxy/releases/latest/download/straico-proxy-aarch64-pc-windows-msvc.zip)

### From Source

```bash
cargo install --path ./proxy/
```
</details>

<details>
<summary><h2>🛠️ Usage</h2></summary>

### HTTP Mode (Default)
```bash
straico-proxy
```

### HTTPS Mode with Auto-Generated Certificate
```bash
straico-proxy --https
```

### HTTPS Mode with Custom Certificates
```bash
straico-proxy --https --cert /path/to/cert.pem --key /path/to/key.pem
```

### Additional Options
```bash
straico-proxy --help
```

Common options:
- `--host <HOST>` - Host address to bind to (default: 127.0.0.1)
- `--port <PORT>` - Port to listen on (default: 8000)
- `--https` - Enable HTTPS mode
- `--cert <PATH>` - Path to TLS certificate file (PEM format)
- `--key <PATH>` - Path to TLS private key file (PEM format)
- `--router` - Enable multi-provider routing mode
- `--log-level <LEVEL>` - Set log level (trace, debug, info, warn, error)
</details>

<details>
<summary><h2>🌐 Global Path Configuration</h2></summary>

To run `straico-proxy` from anywhere, add the binary to your system PATH.

### Linux/macOS
Move the binary to `/usr/local/bin`:
```bash
sudo mv straico-proxy /usr/local/bin/
```

### Windows
1. Create a folder for your binaries (e.g., `C:\bin`).
2. Move `straico-proxy.exe` there.
3. Add `C:\bin` to your Environment Variables under "Path".
</details>

<details>
<summary><h2>🔑 API Keys</h2></summary>

Set your Straico API key via environment variable:

- `STRAICO_API_KEY` - For Straico requests
</details>

<details>
<summary><h2>📝 Example Request</h2></summary>

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
      ...
    ],
    "tools": [...],
    "tool_choice": "auto",
    "max_completion_tokens": 4096
  }'
```
</details>

## License

MIT


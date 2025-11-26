# New chat endpoint

- https://api.straico.com/v2/chat/completions

## Available parameters

- model: String
- temperature: number
- max_tokens: number
- messages: Object (many key-value pairs)
    - role: String
    - content: Array (vector of objects)
        - type: String (Only "text" will be supported here)
        - text: String
        - <only "text" keys will be supported here>

Example:

```json
{
    "model": "meta-llama/llama-4-maverick",
    "messages": [
        {
            "role": "user",
            "content": [{"type": "text", "text": "solve for x => 10x -4x = 0"}]
        },
        {
            "role": "assistant",
            "content": "To solve for x, we need to simplify the equation.\n\nThe given equation is:\n10x - 4x + 2 = 0\n\nFirst, combine the like terms:\n(10x - 4x) + 2 = 0\n6x + 2 = 0\n\nNext, isolate x by subtracting 2 from both sides:\n6x = -2\n\nNow, divide both sides by 6:\nx = -2/6\n\nSimplify the fraction:\nx = -1/3\n\nSo, the solution is x = -1/3."
        },
        {
            "role": "user",
            "content": [{"type": "text", "text": "How much is x times 2"}]
        }
    ]
}
```

## Non-available parameters

- stream
- tool_calls
- tools
- function_call
- functions
- structured_output
- json_mode

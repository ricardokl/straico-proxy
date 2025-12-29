use super::super::common_types::ChatMessage;
use super::types::{ModelProvider, OpenAiTool};
// Note: We use the re-exported error here to match what's expected in the main module
// once we update the re-exports. For now, we use the local ToolCallingError where appropriate.
use super::error::ToolCallingError;

/// Shared preamble for all providers, standardizing the function definitions section.
pub fn build_tools_preamble(
    functions: &[&super::types::OpenAiFunction],
) -> Result<String, ToolCallingError> {
    Ok(format!(
        "You are provided with function signatures within <tools></tools> XML tags:\n<tools>\n{}\n</tools>",
        serde_json::to_string_pretty(&functions)?
    ))
}

/// Returns tool calling format instructions for the Zai provider.
///
/// Uses XML tags with function name and arg_key/arg_value pairs.
pub fn zai_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_call>{function_name}
<arg_key>{parameter_name}</arg_key>
<arg_value>{parameter_value}</arg_value>
</tool_call>

Each tool call must be wrapped in <tool_call> tags with the function name immediately after the opening tag. Parameters are specified using <arg_key> and <arg_value> pairs.

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_call>get_weather
<arg_key>location</arg_key>
<arg_value>Boston, MA</arg_value>
</tool_call>

Example of multiple tool calls:

<tool_call>search_web
<arg_key>query</arg_key>
<arg_value>latest AI news</arg_value>
</tool_call>
<tool_call>summarize_text
<arg_key>text</arg_key>
<arg_value>A long text to be summarized...</arg_value>
</tool_call>"#.to_string()
}

/// Returns tool calling format instructions for the Qwen provider.
///
/// Uses JSON objects wrapped in XML tool_call tags.
pub fn qwen_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_call>
{"name": "function_name", "arguments": {"arg_name": "arg_value"}}
</tool_call>

Each tool call must be a JSON object containing "name" and "arguments" fields, wrapped in <tool_call> XML tags.

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_call>
{"name": "get_weather", "arguments": {"location": "Boston, MA"}}
</tool_call>

Example of multiple tool calls:

<tool_call>
{"name": "search_web", "arguments": {"query": "latest AI news"}}
</tool_call>
<tool_call>
{"name": "summarize_text", "arguments": {"text": "A long text to be summarized..."}}
</tool_call>"#.to_string()
}

/// Returns tool calling format instructions for the MoonshotAI provider.
///
/// Uses special delimiters like <|tool_calls_section_begin|>.
pub fn moonshot_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<|tool_calls_section_begin|><|tool_call_begin|>{function_name}<|tool_call_argument_begin|>{arguments}<|tool_call_end|><|tool_calls_section_end|>

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<|tool_calls_section_begin|><|tool_call_begin|>get_weather<|tool_call_argument_begin|>{"location": "Boston, MA"}<|tool_call_end|><|tool_calls_section_end|>

Example of multiple tool calls:

<|tool_calls_section_begin|><|tool_call_begin|>search_web<|tool_call_argument_begin|>{"query": "latest AI news"}<|tool_call_end|><|tool_call_begin|>summarize_text<|tool_call_argument_begin|>{"text": "A long text to be summarized..."}<|tool_call_end|><|tool_calls_section_end|>"#.to_string()
}

/// Returns default JSON-based tool calling format instructions.
///
/// Uses a JSON array wrapped in <tool_calls> XML tags.
pub fn json_calling_instructions() -> String {
    r#"# Tool Call Format

⚠️ CRITICAL: You MUST use the following exact wrapper syntax. This is not optional.

<tool_calls>
[
  {
    "name": "function_name",
    "arguments": {"arg_name": "arg_value"}
  }
]
</tool_calls>

A JSON array inside <tool_calls> tags where each object contains:
- "name": The function name (string)
- "arguments": The function arguments (JSON object)

❌ DO NOT respond with tool calls in any other format. DO NOT omit the wrapper.

## Examples

Example of a single tool call:

<tool_calls>
[
  {
    "name": "get_weather",
    "arguments": {"location": "Boston, MA"}
  }
]
</tool_calls>

Example of multiple tool calls:

<tool_calls>
[
  {
    "name": "search_web",
    "arguments": {"query": "latest AI news"}
  },
  {
    "name": "summarize_text",
    "arguments": {"text": "A long text to be summarized..."}
  }
]
</tool_calls>"#
        .to_string()
}

pub fn build_tool_system_message(
    provider: ModelProvider,
    functions: &[&super::types::OpenAiFunction],
) -> Result<String, ToolCallingError> {
    let preamble = build_tools_preamble(functions)?;
    let calling_instructions = match provider {
        ModelProvider::Zai => zai_calling_instructions(),
        ModelProvider::Qwen => qwen_calling_instructions(),
        ModelProvider::MoonshotAI => moonshot_calling_instructions(),
        _ => json_calling_instructions(),
    };

    Ok(format!(
        r###"# Tools

You may call one or more functions to assist with the user query.

{}

{}
"###,
        preamble, calling_instructions
    ))
}

/// Generates a system message for tool calling based on the provided tools and model provider.
///
/// Converts OpenAI tool definitions into a provider-specific system message that instructs
/// the model on how to format tool calls.
///
/// # Arguments
/// * `tools` - Slice of OpenAI tool definitions (currently only Function tools are supported)
/// * `provider` - The model provider to generate format instructions for
///
/// # Returns
/// A `ChatMessage::system` containing the formatted tool instructions
///
/// # Errors
/// Returns `ToolCallingError` if:
/// - Function serialization fails
/// - Non-Function tool variants are encountered (after fixing the panic issue)
pub fn tools_system_message(
    tools: &[OpenAiTool],
    provider: ModelProvider,
) -> Result<ChatMessage, ToolCallingError> {
    let functions = tools
        .iter()
        .map(|tool| match tool {
            OpenAiTool::Function(function) => function,
        })
        .collect::<Vec<_>>();

    let system_message = build_tool_system_message(provider, &functions)?;

    Ok(ChatMessage::system(system_message))
}

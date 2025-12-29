mod conversions;
mod error;
mod formatters;
mod parsers;
mod system_messages;
mod templates;
mod types;

pub use conversions::{
    convert_assistant_with_tools_to_straico, convert_straico_assistant_to_openai,
    convert_tool_message_to_straico,
};
pub use error::ToolCallingError;
pub use formatters::format_tool_calls;
pub use parsers::parse_tool_calls;
pub use system_messages::{build_tool_system_message, tools_system_message};
pub use types::{
    ChatFunctionCall, ModelProvider, OpenAiFunction, OpenAiTool, OpenAiToolChoice, ToolCall,
    string_or_object_to_value_deserializer, value_to_string_serializer,
};

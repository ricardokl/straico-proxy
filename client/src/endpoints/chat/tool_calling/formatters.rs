use super::error::ToolCallingError;
use super::types::{ModelProvider, ToolCall};

pub fn format_moonshot_tool_calls(tool_calls: &[ToolCall]) -> Result<String, ToolCallingError> {
    let mut formatted = String::from("<|tool_calls_section_begin|>");
    for tool_call in tool_calls {
        let args = match tool_call.function.arguments.as_str() {
            Some(s) => s.to_string(),
            None => serde_json::to_string(&tool_call.function.arguments)?,
        };

        let name = if tool_call.function.name.is_empty() {
            &tool_call.id
        } else {
            &tool_call.function.name
        };

        formatted.push_str(&format!(
            "<|tool_call_begin|>{}<|tool_call_argument_begin|>{}<|tool_call_end|>",
            name, args
        ));
    }
    formatted.push_str("<|tool_calls_section_end|>");
    Ok(formatted)
}

pub fn format_qwen_tool_calls(tool_calls: &[ToolCall]) -> Result<String, ToolCallingError> {
    let mut formatted = String::new();
    for tool_call in tool_calls {
        let name = if tool_call.function.name.is_empty() {
            &tool_call.id
        } else {
            &tool_call.function.name
        };

        let call_obj = serde_json::json!({
            "name": name,
            "arguments": tool_call.function.arguments
        });
        formatted.push_str(&format!(
            "<tool_call>\n{}\n</tool_call>\n",
            serde_json::to_string(&call_obj)?
        ));
    }
    Ok(formatted.trim().to_string())
}

pub fn format_zai_tool_calls(tool_calls: &[ToolCall]) -> Result<String, ToolCallingError> {
    let mut formatted = String::new();
    for tool_call in tool_calls {
        let name = if tool_call.function.name.is_empty() {
            &tool_call.id
        } else {
            &tool_call.function.name
        };

        formatted.push_str(&format!("<tool_call>{}\n", name));
        if let Some(obj) = tool_call.function.arguments.as_object() {
            for (k, v) in obj {
                let val_str = if v.is_string() {
                    v.as_str().unwrap().to_string()
                } else {
                    v.to_string()
                };
                formatted.push_str(&format!(
                    "<arg_key>{}</arg_key>\n<arg_value>{}</arg_value>\n",
                    k, val_str
                ));
            }
        }
        formatted.push_str("</tool_call>\n");
    }
    Ok(formatted.trim().to_string())
}

pub fn format_json_tool_calls(tool_calls: &[ToolCall]) -> Result<String, ToolCallingError> {
    let simplified: Vec<_> = tool_calls
        .iter()
        .map(|tc| {
            serde_json::json!({
                "name": if tc.function.name.is_empty() { &tc.id } else { &tc.function.name },
                "arguments": tc.function.arguments
            })
        })
        .collect();
    Ok(format!(
        "<tool_calls>\n{}\n</tool_calls>",
        serde_json::to_string_pretty(&simplified)?
    ))
}

pub fn format_tool_calls(
    tool_calls: &[ToolCall],
    provider: ModelProvider,
) -> Result<String, ToolCallingError> {
    match provider {
        ModelProvider::MoonshotAI => format_moonshot_tool_calls(tool_calls),
        ModelProvider::Qwen => format_qwen_tool_calls(tool_calls),
        ModelProvider::Zai => format_zai_tool_calls(tool_calls),
        _ => format_json_tool_calls(tool_calls),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoints::chat::tool_calling::types::ChatFunctionCall;

    #[test]
    fn test_qwen_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg": "val"}),
            },
            index: None,
        }];
        let formatted = format_tool_calls(&tool_calls, ModelProvider::Qwen).unwrap();
        assert!(formatted.contains("<tool_call>"));
        assert!(formatted.contains("\"name\":\"test_func\""));
        assert!(formatted.contains("\"arguments\":{\"arg\":\"val\"}"));
    }

    #[test]
    fn test_zai_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_456".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg1": "val1", "arg2": 2}),
            },
            index: None,
        }];
        let formatted = format_tool_calls(&tool_calls, ModelProvider::Zai).unwrap();
        assert!(formatted.contains("<tool_call>test_func"));
        assert!(formatted.contains("<arg_key>arg1</arg_key>"));
        assert!(formatted.contains("<arg_value>val1</arg_value>"));
    }

    #[test]
    fn test_moonshot_tool_call_formatting() {
        let tool_calls = vec![ToolCall {
            id: "call_12345".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: serde_json::json!({"arg": "val"}),
            },
            index: None,
        }];
        let formatted = format_tool_calls(&tool_calls, ModelProvider::MoonshotAI).unwrap();
        assert!(formatted.contains("<|tool_call_begin|>test_func<|tool_call_argument_begin|>"));
        assert!(formatted.contains("{\"arg\":\"val\"}"));
    }
}

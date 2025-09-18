use crate::endpoints::completion::completion_request::Prompt;
use crate::endpoints::completion::completion_response::{Message, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::ops::Deref;

/// Represents a chat conversation as a sequence of messages.
///
/// The `Chat` struct is a wrapper around a vector of `Message` values that represents
/// an entire chat conversation between a user, AI assistant, and optionally system messages
/// or tool outputs.
///
/// This struct implements `Deref` to provide direct access to the underlying vector
/// operations while maintaining type safety and encapsulation.
#[derive(Deserialize, Clone, Debug)]
pub struct Chat(Vec<Message>);

impl Deref for Chat {
    type Target = Vec<Message>;

    /// Implements `Deref` for `Chat` to provide direct access to the underlying vector.
    ///
    /// This method returns a reference to the inner `Vec<Message>` stored in the `Chat`
    /// struct, allowing direct access to vector operations while maintaining
    /// encapsulation.
    ///
    /// # Returns
    ///
    /// A reference to the underlying `Vec<Message>` that stores the chat messages.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents a tool/function that can be called by an AI assistant.
///
/// The `Tool` enum is used to define callable functions that an AI can use during
/// conversation. Each function represents a capability that can be invoked by the
/// assistant.
///
/// # Variants
///
/// * `Function` - Represents a callable function with the following fields:
///   * `name` - The name of the function that can be called
///   * `description` - Optional text describing the function's purpose and behavior
///   * `parameters` - Optional JSON schema defining the function's parameter structure
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase", content = "function")]
pub enum Tool {
    // From the request sent to the server
    // On the same level as 'model, or 'temperature'
    Function {
        /// Name of the function
        name: String,
        /// Optional description of what the function does
        description: Option<String>,
        /// Optional JSON schema of function parameters
        parameters: Option<Value>,
    },
}

/// Defines the format for structuring chat prompts for different language models.
///
/// The `PromptFormat` struct specifies how different parts of a chat conversation should be
/// formatted when creating prompts for language models. It controls the wrapping text and
/// markers around system messages, user messages, and assistant responses.
///
/// # Fields
///
/// * `begin` - Text to insert at the very start of the prompt
/// * `system_pre` - Text to insert before system messages
/// * `system_post` - Text to insert after system messages
/// * `user_pre` - Text to insert before user messages
/// * `user_post` - Text to insert after user messages
/// * `assistant_pre` - Text to insert before assistant responses
/// * `assistant_post` - Text to insert after assistant responses
/// * `end` - Text to append at the very end of the prompt
pub struct PromptFormat<'a> {
    /// Text to insert at the very beginning of the prompt
    begin: &'a str,
    /// Text to insert before system messages
    system_pre: &'a str,
    /// Text to insert after system messages
    system_post: &'a str,
    /// Text to insert before user messages
    user_pre: &'a str,
    /// Text to insert after user messages
    user_post: &'a str,
    /// Text to insert before assistant responses
    assistant_pre: &'a str,
    /// Text to insert after assistant responses
    assistant_post: &'a str,
    /// Text to append at the very end of the prompt
    #[allow(dead_code)]
    end: &'a str,
    pub tool_calls: ToolCallsFormat<'a>,
    pub tool_output: ToolOutputFormat<'a>,
}

pub struct ToolCallsFormat<'a> {
    pub tool_calls_begin: &'a str,
    pub tool_call_begin: &'a str,
    pub tool_sep: &'a str,
    pub tool_call_end: &'a str,
    pub tool_calls_end: &'a str,
}

pub struct ToolOutputFormat<'a> {
    pub tool_outputs_begin: &'a str,
    pub tool_output_begin: &'a str,
    pub tool_output_end: &'a str,
    pub tool_outputs_end: &'a str,
    pub end: &'a str,
}

impl Default for ToolCallsFormat<'_> {
    fn default() -> Self {
        ToolCallsFormat {
            tool_calls_begin: "<tool_calls>",
            tool_call_begin: "<tool_call>",
            tool_sep: ",",
            tool_call_end: "</tool_call>",
            tool_calls_end: "</tool_calls>",
        }
    }
}

impl Default for ToolOutputFormat<'_> {
    fn default() -> Self {
        ToolOutputFormat {
            tool_outputs_begin: "<user>",
            tool_output_begin: "<tool_response>",
            tool_output_end: "</tool_response>",
            tool_outputs_end: "</user>",
            end: "",
        }
    }
}

impl Default for PromptFormat<'_> {
    /// Returns a default prompt format suitable for basic chat interactions.
    ///
    /// This implementation provides a simple instruction/response style format with:
    /// - No special prefix/suffix for system messages
    /// - "### Instruction:" prefix for user messages
    /// - "### Response:" prefix for assistant messages and prompt ending
    ///
    /// # Returns
    ///
    /// A `PromptFormat` instance initialized with default formatting strings for
    /// basic chat interactions.
    fn default() -> Self {
        PromptFormat {
            begin: "",
            system_pre: "<system>",
            system_post: "</system>",
            user_pre: "<user>",
            user_post: "</user>",
            assistant_pre: "<assistant>",
            assistant_post: "</assistant>",
            end: "<assistant>",
            tool_calls: ToolCallsFormat::default(),
            tool_output: ToolOutputFormat::default(),
        }
    }
}

/// Defines the prompt format used by DeepSeek language models.
pub const DEEPSEEK_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "<|begin_of_sentence|>",
    system_pre: "",
    system_post: "",
    user_pre: "<|User|>",
    user_post: "",
    assistant_pre: "<|Assistant|>",
    assistant_post: "<|end_of_sentence|>",
    end: "<|Assistant|>",
    tool_calls: ToolCallsFormat {
        tool_calls_begin: "<|tool_calls_begin|>",
        tool_call_begin: "<|tool_call_begin|>",
        tool_sep: "<|tool_sep|>",
        tool_call_end: "<|tool_call_end|>",
        tool_calls_end: "<|tool_calls_end|>",
    },
    tool_output: ToolOutputFormat {
        tool_outputs_begin: "<|tool_outputs_begin|>",
        tool_output_begin: "<|tool_output_begin|>",
        tool_output_end: "<|tool_output_end|>",
        tool_outputs_end: "<|tool_outputs_end|>",
        end: "",
    },
};

/// Defines the prompt format used by Anthropic's language models like Claude.
pub const ANTHROPIC_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "",
    system_pre: "",
    system_post: "\n",
    user_pre: "\nHuman: ",
    user_post: "\n",
    assistant_pre: "\nAssistant: ",
    assistant_post: "\n",
    end: "\nAssistant:",
    tool_output: ToolOutputFormat {
        tool_outputs_begin: "<user>",
        tool_output_begin: "<tool_response>",
        tool_output_end: "</tool_response>",
        tool_outputs_end: "</user>",
        end: "",
    },
    tool_calls: ToolCallsFormat {
        tool_calls_begin: "<tool_calls>",
        tool_call_begin: "<tool_call>",
        tool_sep: ",",
        tool_call_end: "</tool_call>",
        tool_calls_end: "</tool_calls>",
    },
};

/// Defines the prompt format used by Mistral AI's language models.
pub const MISTRAL_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "",
    system_pre: "[INST] <<SYS>>",
    system_post: "<</SYS>> [/INST]",
    user_pre: "[INST]",
    user_post: "[/INST]",
    assistant_pre: "",
    assistant_post: "",
    end: "",
    ..ANTHROPIC_PROMPT_FORMAT
};

/// Defines the prompt format used by LLaMA 3 language models.
pub const LLAMA3_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "<|begin_of_text|>",
    system_pre: "<|start_header_id|>system<|end_header_id|>\n\n",
    system_post: "<|eot_id|>",
    user_pre: "<|start_header_id|>user<|end_header_id|>\n\n",
    user_post: "<|eot_id|>",
    assistant_pre: "<|start_header_id|>assistant<|end_header_id|>\n\n",
    assistant_post: "<|eot_id|>",
    end: "<|start_header_id|>assistant<|end_header_id|>\n\n",
    ..ANTHROPIC_PROMPT_FORMAT
};

/// Defines the prompt format used by Command-R language models.
pub const COMMAND_R_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "",
    system_pre: "<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|>",
    system_post: "<|END_OF_TURN_TOKEN|>",
    user_pre: "<|START_OF_TURN_TOKEN|><|USER_TOKEN|>",
    user_post: "<|END_OF_TURN_TOKEN|>",
    assistant_pre: "<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>",
    assistant_post: "<|END_OF_TURN_TOKEN|>",
    end: "<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>",
    ..ANTHROPIC_PROMPT_FORMAT
};

/// Defines the prompt format used by Qwen language models.
pub const QWEN_PROMPT_FORMAT: PromptFormat<'_> = PromptFormat {
    begin: "",
    system_pre: "<|im_start|>system\n",
    system_post: "<|im_end|>",
    user_pre: "<|im_start|>user\n",
    user_post: "<|im_end|>",
    assistant_pre: "<|im_start|>assistant\n",
    assistant_post: "<|im_end|>",
    end: "<|im_start|>assistant\n",
    ..ANTHROPIC_PROMPT_FORMAT
};

impl Chat {
    /// Converts a chat conversation into a formatted prompt string for language models
    ///
    /// # Arguments
    ///
    /// * `self` - The Chat instance containing the conversation messages
    /// * `tools` - Optional vector of Tool instances that can be called by the model
    /// * `model` - String identifier for the language model to format the prompt for
    ///
    /// # Returns
    ///
    /// Returns a Prompt instance containing the formatted conversation text with appropriate
    /// model-specific formatting and any tool definitions
    pub fn to_prompt<'a>(self, tools: Option<Vec<Tool>>, model: &str) -> Prompt<'a> {
        let format = if model.to_lowercase().contains("anthropic") {
            ANTHROPIC_PROMPT_FORMAT
        } else if model.to_lowercase().contains("mistral") {
            MISTRAL_PROMPT_FORMAT
        } else if model.to_lowercase().contains("llama3") {
            LLAMA3_PROMPT_FORMAT
        } else if model.to_lowercase().contains("command") {
            COMMAND_R_PROMPT_FORMAT
        } else if model.to_lowercase().contains("qwen") {
            QWEN_PROMPT_FORMAT
        } else if model.to_lowercase().contains("anthropic") {
            ANTHROPIC_PROMPT_FORMAT
        } else if model.to_lowercase().contains("deepseek") {
            DEEPSEEK_PROMPT_FORMAT
        } else {
            PromptFormat::default()
        };

        let pre_tools: &str = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;
        //        let post_tools: &str = r###"
        //</tools>
        //# Tool Calls
        //
        //For each tool call, return a json object with function name and arguments within \<tool_call\>\</tool_call\> XML tags:"
        //\<tool_call\>{"name": <function-name>, "arguments": <args-json-object>}\</tool_call\>
        //"###;

        let post_tools: &str = &format!(
            "\n </tools>\n # Tool Calls\n \nStart with the opening tag {}. For each tool call, return a json object with function name and arguments within {}{} tags:\n{}{{\"name\": <function-name>{} \"arguments\": <args-json-object>}}{}. close the tool calls section with {}\n",
            format.tool_calls.tool_calls_begin,
            format.tool_calls.tool_call_begin,
            format.tool_calls.tool_call_end,
            format.tool_calls.tool_call_begin,
            format.tool_calls.tool_sep,
            format.tool_calls.tool_call_end,
            format.tool_calls.tool_calls_end
        );

        let mut tools_message = String::new();
        if let Some(tools) = &tools {
            tools_message.push_str(pre_tools);
            for tool in tools {
                tools_message.push_str(&serde_json::to_string_pretty(tool).unwrap());
            }
            tools_message.push_str(post_tools);
        }
        let mut output = String::new();
        output.push_str(format.begin);
        for (i, message) in self.0.iter().enumerate() {
            match (i, message) {
                (0, Message::System { content }) => {
                    output.push_str(format.system_pre);
                    if content.is_empty() {
                        output.push_str("You are a helpful assistant.\n");
                    } else {
                        output.push_str(&format!("{}\n", content));
                    }
                    output.push_str(&tools_message);
                    output.push_str(format.system_post);
                }
                (_, Message::User { content }) => {
                    if i == 0 {
                        output.push_str(&format!(
                            "{}You are a helpful assistant.\n{}{}\n",
                            format.system_pre, &tools_message, format.system_post
                        ))
                    }
                    output.push_str(&format!(
                        "{}{}\n{}\n",
                        format.user_pre, content, format.user_post
                    ));
                }

                (
                    _,
                    Message::Assistant {
                        content,
                        tool_calls,
                    },
                ) if i > 0 => {
                    output.push_str(format.assistant_pre);
                    match (content, tool_calls) {
                        (Some(c), None) => output.push_str(&c.to_string()),
                        (None, Some(t)) => {
                            output.push_str(format.tool_calls.tool_calls_begin);
                            for tool_call in t {
                                let ToolCall::Function { function, .. } = tool_call;
                                output.push_str(&format!(
                                    "{}\n{}\n{}",
                                    format.tool_calls.tool_call_begin,
                                    serde_json::to_string(function).unwrap(),
                                    format.tool_calls.tool_call_end
                                ));
                            }
                            output.push_str(format.tool_calls.tool_calls_end);
                        }
                        // Maybe this is unreachable? Depends on the provider never answering like this.
                        (Some(c), Some(t)) => {
                            output.push_str(&c.to_string());
                            for tool_call in t {
                                let ToolCall::Function { function, .. } = tool_call;
                                output.push_str(&format!(
                                    "<tool_call>\n{}\n</tool_call>",
                                    serde_json::to_string(function).unwrap()
                                ));
                            }
                        }
                        (None, None) => {}
                    }
                    output.push_str(format.assistant_post);
                }
                (_, Message::Tool { content, .. }) if i > 0 => {
                    // Check if previous message was not a tool
                    if !matches!(self.0.get(i - 1), Some(Message::Tool { .. })) {
                        output.push_str(format.tool_output.tool_outputs_begin);
                    }

                    output.push_str(format.tool_output.tool_output_begin);
                    output.push_str(&content.to_string());
                    output.push_str(format.tool_output.tool_output_end);

                    // Check if next message is not a tool
                    if i == self.0.len() - 1
                        || !matches!(self.0.get(i + 1), Some(Message::Tool { .. }))
                    {
                        output.push_str(format.tool_output.tool_outputs_end);
                        output.push_str(format.tool_output.end);
                    }
                }
                (_, _) => {
                    eprintln!(
                        "Message {:?} not in the expected position (found at index {})",
                        message, i
                    );
                }
            }
        }
        //output.push_str(format.end);
        Prompt::from(Cow::Owned(output))
    }
}

use serde::Serialize;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::endpoints::completion::completion_response::{
    Choice, Completion, Message, ToolCall,
};
#[cfg(not(test))]
use straico_client::endpoints::completion::completion_response::Usage;

#[derive(Serialize, Debug, Clone)]
pub struct CompletionStream {
    pub choices: Vec<ChoiceStream>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    #[cfg(not(test))]
    pub usage: Usage,
    #[cfg(test)]
    pub usage: (),
}

#[derive(Serialize, Debug, Clone)]
pub struct ChoiceStream {
    pub index: u8,
    pub delta: Delta,
    pub finish_reason: Option<Box<str>>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

// State enum to track progress through the delta
#[derive(Debug, PartialEq)]
enum DeltaState {
    Role,
    Content,
    ToolCalls,
    Done,
}

pub struct DeltaIterator {
    state: DeltaState,
    role: Option<Box<str>>,
    content: Option<Box<str>>,
    tool_calls: Option<Vec<ToolCall>>,
}

pub struct ChoiceStreamIterator {
    index: u8,
    delta_iter: DeltaIterator,
    finish_reason: Option<Box<str>>,
}

pub struct CompletionStreamIterator {
    choices: Vec<ChoiceStreamIterator>,
    object: Box<str>,
    id: Box<str>,
    model: Box<str>,
    created: u64,
    #[cfg(not(test))]
    usage: Usage,
    #[cfg(test)]
    usage: (),
    done: bool,
}

impl IntoIterator for Delta {
    type Item = Delta;
    type IntoIter = DeltaIterator;

    fn into_iter(self) -> Self::IntoIter {
        let initial_state = if self.role.is_some() {
            DeltaState::Role
        } else if self.content.is_some() {
            DeltaState::Content
        } else if self.tool_calls.is_some() {
            DeltaState::ToolCalls
        } else {
            DeltaState::Done
        };

        DeltaIterator {
            state: initial_state,
            role: self.role,
            content: self.content,
            tool_calls: self.tool_calls,
        }
    }
}

impl IntoIterator for ChoiceStream {
    type Item = ChoiceStream;
    type IntoIter = ChoiceStreamIterator;

    fn into_iter(self) -> Self::IntoIter {
        ChoiceStreamIterator {
            index: self.index,
            delta_iter: self.delta.into_iter(),
            finish_reason: self.finish_reason,
        }
    }
}

impl IntoIterator for CompletionStream {
    type Item = CompletionStream;
    type IntoIter = CompletionStreamIterator;

    fn into_iter(self) -> Self::IntoIter {
        CompletionStreamIterator {
            choices: self
                .choices
                .into_iter()
                .map(IntoIterator::into_iter)
                .collect(),
            object: self.object,
            id: self.id,
            model: self.model,
            created: self.created,
            usage: self.usage,
            done: false,
        }
    }
}

impl Iterator for DeltaIterator {
    type Item = Delta;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            DeltaState::Role => {
                self.state = if self.content.is_some() {
                    DeltaState::Content
                } else if self.tool_calls.is_some() {
                    DeltaState::ToolCalls
                } else {
                    DeltaState::Done
                };
                let role = self.role.take()?;
                Some(Delta {
                    role: Some(role),
                    content: None,
                    tool_calls: None,
                })
            }
            DeltaState::Content => {
                self.state = if self.tool_calls.is_some() {
                    DeltaState::ToolCalls
                } else {
                    DeltaState::Done
                };
                let content = self.content.take()?;
                Some(Delta {
                    role: None,
                    content: Some(content),
                    tool_calls: None,
                })
            }
            DeltaState::ToolCalls => {
                self.state = DeltaState::Done;
                let tool_calls = self.tool_calls.take()?;
                Some(Delta {
                    role: None,
                    content: None,
                    tool_calls: Some(tool_calls),
                })
            }
            DeltaState::Done => None,
        }
    }
}

impl Iterator for ChoiceStreamIterator {
    type Item = ChoiceStream;

    fn next(&mut self) -> Option<Self::Item> {
        let delta = self.delta_iter.next()?;
        let finish_reason = if self.delta_iter.state == DeltaState::Done {
            self.finish_reason.clone()
        } else {
            None
        };
        Some(ChoiceStream {
            index: self.index,
            delta,
            finish_reason,
        })
    }
}

impl Iterator for CompletionStreamIterator {
    type Item = CompletionStream;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut choices_next = Vec::new();
        let mut all_done = true;

        for choice_iter in &mut self.choices {
            if let Some(next_choice) = choice_iter.next() {
                choices_next.push(next_choice);
                if choice_iter.delta_iter.state != DeltaState::Done {
                    all_done = false;
                }
            }
        }

        if choices_next.is_empty() {
            self.done = true;
            return None;
        }

        self.done = all_done;

        Some(CompletionStream {
            choices: choices_next,
            object: self.object.clone(),
            id: self.id.clone(),
            model: self.model.clone(),
            created: self.created,
            usage: self.usage.clone(),
        })
    }
}

impl From<Message> for Delta {
    fn from(value: Message) -> Self {
        match value {
            Message::User { content } => Delta {
                role: Some("user".into()),
                content: Some(content.to_string().into()),
                tool_calls: None,
            },
            Message::Assistant {
                content,
                tool_calls,
            } => Delta {
                role: Some("assistant".into()),
                content: content.map(|c| c.to_string().into()),
                tool_calls,
            },
            Message::System { content } => Delta {
                role: Some("system".into()),
                content: Some(content.to_string().into()),
                tool_calls: None,
            },
            Message::Tool { content } => Delta {
                role: Some("function".into()),
                content: Some(content.to_string().into()),
                tool_calls: None,
            },
        }
    }
}

impl From<Choice> for ChoiceStream {
    fn from(value: Choice) -> Self {
        Self {
            index: value.index,
            delta: value.message.into(),
            finish_reason: Some(value.finish_reason),
        }
    }
}

impl From<Completion> for CompletionStream {
    fn from(value: Completion) -> Self {
        Self {
            choices: value.choices.into_iter().map(Into::into).collect(),
            object: value.object,
            id: value.id,
            model: value.model,
            created: value.created,
            #[cfg(not(test))]
            usage: value.usage,
            #[cfg(test)]
            usage: (),
        }
    }
}

pub fn create_initial_chunk(model: &str, id: &str) -> Value {
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    json!({
        "choices": [{
            "delta": {
                "role": "assistant"
            },
            "index": 0
        }],
        "object": "chat.completion.chunk",
        "id": id,
        "model": model,
        "created": created
    })
}

pub fn create_heartbeat_chunk() -> Value {
    json!({
        "choices": [{
            "delta": {},
            "index": 0
        }],
        "object": "chat.completion.chunk"
    })
}

pub fn create_error_chunk(error: &str) -> Value {
    json!({
        "error": {
            "message": error
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_initial_chunk() {
        let model = "test-model";
        let id = "test-id";
        let chunk = create_initial_chunk(model, id);
        assert_eq!(chunk["object"], "chat.completion.chunk");
        assert_eq!(chunk["id"], id);
        assert_eq!(chunk["model"], model);
        assert_eq!(chunk["choices"][0]["delta"]["role"], "assistant");
        assert!(chunk["created"].is_u64());
    }

    #[test]
    fn test_create_heartbeat_chunk() {
        let chunk = create_heartbeat_chunk();
        assert_eq!(chunk["object"], "chat.completion.chunk");
        assert!(chunk["choices"][0]["delta"].is_object());
        assert!(chunk["choices"][0]["delta"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_create_error_chunk() {
        let error_message = "This is an error";
        let chunk = create_error_chunk(error_message);
        assert_eq!(chunk["error"]["message"], error_message);
    }

    #[test]
    fn test_completion_stream_iterator_simple() {
        let stream = CompletionStream {
            id: "cmpl-123".into(),
            object: "chat.completion".into(),
            created: 1677652288,
            model: "gpt-3.5-turbo-0613".into(),
            choices: vec![ChoiceStream {
                index: 0,
                delta: Delta {
                    role: Some("assistant".into()),
                    content: Some("Hello there!".into()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: (), // In test builds, usage is a unit type `()`
        };

        let chunks: Vec<CompletionStream> = stream.into_iter().collect();

        assert_eq!(chunks.len(), 2);

        let choice1 = &chunks[0].choices[0];
        assert_eq!(choice1.delta.role, Some("assistant".into()));
        assert!(choice1.delta.content.is_none());
        assert!(choice1.finish_reason.is_none());

        let choice2 = &chunks[1].choices[0];
        assert!(choice2.delta.role.is_none());
        assert_eq!(choice2.delta.content, Some("Hello there!".into()));
        assert_eq!(choice2.finish_reason, Some("stop".into()));
    }
}

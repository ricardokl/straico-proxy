use serde::Serialize;
use straico_client::endpoints::completion::completion_response::{
    Choice, Completion, Message, ToolCall, Usage,
};

#[derive(Serialize, Debug, Clone)]
pub struct CompletionStream {
    pub choices: Vec<ChoiceStream>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    pub usage: Usage,
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
    content_chunks: Vec<Box<str>>,
    content_index: usize,
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
    usage: Usage,
    done: bool,
}

impl IntoIterator for Delta {
    type Item = Delta;
    type IntoIter = DeltaIterator;

    fn into_iter(self) -> Self::IntoIter {
        // Pre-process content if it exists
        let content_chunks = if let Some(content) = self.content {
            content.split_inclusive(' ').map(Box::from).collect()
        } else {
            Vec::new()
        };

        // Determine initial state
        let initial_state = if self.role.is_some() {
            DeltaState::Role
        } else if !content_chunks.is_empty() {
            DeltaState::Content
        } else if self.tool_calls.is_some() {
            DeltaState::ToolCalls
        } else {
            DeltaState::Done
        };

        DeltaIterator {
            state: initial_state,
            role: self.role,
            content_chunks,
            content_index: 0,
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
                // Transition to next state
                self.state = if !self.content_chunks.is_empty() {
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
                if self.content_index < self.content_chunks.len() {
                    let chunk = self.content_chunks[self.content_index].clone();
                    self.content_index += 1;

                    // Check if we're done with content and should transition
                    if self.content_index >= self.content_chunks.len() {
                        self.state = if self.tool_calls.is_some() {
                            DeltaState::ToolCalls
                        } else {
                            DeltaState::Done
                        };
                    }

                    Some(Delta {
                        role: None,
                        content: Some(chunk),
                        tool_calls: None,
                    })
                } else {
                    // This shouldn't happen due to the state transition above,
                    // but handle it gracefully
                    self.state = if self.tool_calls.is_some() {
                        DeltaState::ToolCalls
                    } else {
                        DeltaState::Done
                    };
                    self.next()
                }
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

        // Only include finish_reason when we're at the end of the delta iterator
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

        // Collect next items from each choice iterator
        let mut choices_next = Vec::new();
        let mut all_done = true;

        for choice_iter in &mut self.choices {
            if let Some(next_choice) = choice_iter.next() {
                choices_next.push(next_choice);

                // If this iterator isn't done yet, not all are done
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

        // Create the next CompletionStream
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
            usage: value.usage,
        }
    }
}

use serde::Serialize;
use straico_client::endpoints::completion::completion_response::{
    Choice, Completion, Message, ToolCall, Usage,
};

#[derive(Serialize, Debug)]
pub struct CompletionStream {
    pub choices: Vec<ChoiceStream>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    pub usage: Usage,
}

#[derive(Serialize, Debug)]
pub struct ChoiceStream {
    pub index: u8,
    pub delta: Delta,
    pub finish_reason: Option<Box<str>>,
}

#[derive(Serialize, Debug)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

pub struct DeltaIterator<T, I, U> {
    pub role: T,
    pub content: Option<I>,
    pub tool_calls: Option<U>,
}

pub struct ChoiceStreamIterator<T, I, U> {
    pub index: u8,
    pub delta: DeltaIterator<T, I, U>,
    pub finish_reason: Option<Box<str>>,
}

pub struct CompletionStreamIterator<T, I, U> {
    pub choices: Vec<ChoiceStreamIterator<T, I, U>>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    pub usage: Usage,
}

impl IntoIterator for Delta {
    type Item = Delta;
    type IntoIter = DeltaIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        DeltaIterator {
            role: vec![self.role.unwrap()].into_iter(),
            content: self.content.map(|c| {
                c.split_inclusive(' ')
                    .map(Box::from)
                    .collect::<Vec<Box<_>>>()
                    .into_iter()
            }),
            tool_calls: self.tool_calls.map(|t| vec![t].into_iter()),
        }
    }
}

impl IntoIterator for ChoiceStream {
    type Item = ChoiceStream;
    type IntoIter = ChoiceStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        ChoiceStreamIterator {
            index: self.index,
            delta: self.delta.into_iter(),
            finish_reason: self.finish_reason,
        }
    }
}

impl IntoIterator for CompletionStream {
    type Item = CompletionStream;
    type IntoIter = CompletionStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        CompletionStreamIterator {
            choices: self.choices.into_iter().map(|x| x.into_iter()).collect(),
            object: self.object,
            id: self.id,
            model: self.model,
            created: self.created,
            usage: self.usage,
        }
    }
}

impl<I, T, U> Iterator for DeltaIterator<I, T, U>
where
    I: Iterator<Item = Box<str>>,
    T: Iterator<Item = Box<str>>,
    U: Iterator<Item = Vec<ToolCall>>,
{
    type Item = Delta;

    fn next(&mut self) -> Option<Self::Item> {
        let delta = Delta {
            role: self.role.next(),
            content: self.content.as_mut().and_then(Iterator::next),
            tool_calls: self.tool_calls.as_mut().and_then(Iterator::next),
        };
        if delta.content.is_none() && delta.tool_calls.is_none() {
            None
        } else {
            Some(delta)
        }
    }
}

impl Iterator
    for ChoiceStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >
{
    type Item = ChoiceStream;

    fn next(&mut self) -> Option<Self::Item> {
        let choice = ChoiceStream {
            index: self.index,
            delta: self.delta.next()?,
            finish_reason: self.finish_reason.clone(),
        };
        Some(choice)
    }
}

impl Iterator
    for CompletionStreamIterator<
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Box<str>>,
        std::vec::IntoIter<Vec<ToolCall>>,
    >
{
    type Item = CompletionStream;

    fn next(&mut self) -> Option<Self::Item> {
        let completion = CompletionStream {
            choices: self
                .choices
                .iter_mut()
                .map(|x| x.next())
                .collect::<Option<Vec<ChoiceStream>>>()?,
            object: self.object.clone(),
            id: self.id.clone(),
            model: self.model.clone(),
            created: self.created,
            usage: self.usage.clone(),
        };
        Some(completion)
    }
}

impl From<Message> for Delta {
    fn from(value: Message) -> Self {
        match value {
            Message::User { content } => Delta {
                role: Some("user".into()),
                content: Some(content.to_string().into_boxed_str()),
                tool_calls: None,
            },
            Message::Assistant {
                content,
                tool_calls,
            } => Delta {
                role: Some("assistant".into()),
                content: content.map(|c| c.to_string().into_boxed_str()),
                tool_calls,
            },
            Message::System { content } => Delta {
                role: Some("system".into()),
                content: Some(content.to_string().into_boxed_str()),
                tool_calls: None,
            },
            Message::Tool { content } => Delta {
                role: Some("function".into()),
                content: Some(content.to_string().into_boxed_str()),
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

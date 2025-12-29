use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatError {
    #[error(transparent)]
    ToolCalling(#[from] super::tool_calling::ToolCallingError),
}

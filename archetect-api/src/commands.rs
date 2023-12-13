use std::fmt::Debug;

use serde::{Deserialize, Serialize};

pub use crate::commands::bool_prompt_info::BoolPromptInfo;
pub use crate::commands::editor_prompt_info::EditorPromptInfo;
pub use crate::commands::int_prompt_info::IntPromptInfo;
pub use crate::commands::list_prompt_info::ListPromptInfo;
pub use crate::commands::multiselect_prompt_info::MultiSelectPromptInfo;
pub use crate::commands::prompt_info::{
    PromptInfo, PromptInfoItemsRestrictions, PromptInfoLengthRestrictions, PromptInfoPageable,
};
pub use crate::commands::select_prompt_info::SelectPromptInfo;
pub use crate::commands::text_prompt_info::TextPromptInfo;

mod bool_prompt_info;
mod editor_prompt_info;
mod int_prompt_info;
mod list_prompt_info;
mod multiselect_prompt_info;
mod prompt_info;
mod select_prompt_info;
mod text_prompt_info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CommandRequest {
    /// Prompt for Text
    PromptForText(TextPromptInfo),
    /// Prompt for a Signed Integer
    PromptForInt(IntPromptInfo),
    /// Prompt for a Boolean
    PromptForBool(BoolPromptInfo),
    /// Prompt for a List of Strings
    PromptForList(ListPromptInfo),
    /// Prompt to Select an item from a pre-defined list
    PromptForSelect(SelectPromptInfo),
    /// Prompt to Select multiple items  from a pre-defined list
    PromptForMultiSelect(MultiSelectPromptInfo),
    /// Prompt to gather text from an editor
    PromptForEditor(EditorPromptInfo),
    /// Log a String at Trace Level
    LogTrace(String),
    /// Log a String at Debug Level
    LogDebug(String),
    /// Log a String at Info Level
    LogInfo(String),
    /// Log a String at Warn Level
    LogWarn(String),
    /// Log a String at Error Level
    LogError(String),
    /// Print a String that may be potentially captured as output, such as on STDOUT
    Print(String),
    /// Print a String that show not be captured as output, such as on STDERR
    Display(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CommandResponse {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<String>),
    None,
    Error(String),
    Abort,
}

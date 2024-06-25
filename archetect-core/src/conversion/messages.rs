use archetect_api::{
    BoolPromptInfo, ClientMessage, EditorPromptInfo, ExistingFilePolicy, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, PromptInfo, PromptInfoPageable, ScriptMessage, SelectPromptInfo, TextPromptInfo,
    WriteDirectoryInfo, WriteFileInfo,
};

use crate::proto;
use crate::proto::script_message::Message;
use crate::proto::WriteDirectory;

impl From<proto::ClientMessage> for ClientMessage {
    fn from(value: proto::ClientMessage) -> Self {
        match value.message.expect("Valid Message") {
            proto::client_message::Message::Initialize(message) => ClientMessage::Initialize {
                answers_yaml: message.answers_yaml,
                switches: message.switches,
                use_defaults: message.use_defaults,
                use_defaults_all: message.use_defaults_all,
                destination: message.destination,
            },
            proto::client_message::Message::String(value) => ClientMessage::String(value),
            proto::client_message::Message::Integer(value) => ClientMessage::Integer(value),
            proto::client_message::Message::Boolean(value) => ClientMessage::Boolean(value),
            proto::client_message::Message::Error(message) => ClientMessage::Error(message),
            proto::client_message::Message::Array(array) => ClientMessage::Array(array.values),
            proto::client_message::Message::None(_unit) => ClientMessage::None,
            proto::client_message::Message::Abort(_unit) => ClientMessage::Abort,
            proto::client_message::Message::Ack(_unit) => ClientMessage::Ack,
        }
    }
}

impl From<ClientMessage> for proto::ClientMessage {
    fn from(value: ClientMessage) -> Self {
        match value {
            ClientMessage::String(value) => proto::ClientMessage {
                message: Some(proto::client_message::Message::String(value)),
            },
            ClientMessage::Integer(value) => proto::ClientMessage {
                message: Some(proto::client_message::Message::Integer(value)),
            },
            ClientMessage::Boolean(value) => proto::ClientMessage {
                message: Some(proto::client_message::Message::Boolean(value)),
            },
            ClientMessage::Array(values) => proto::ClientMessage {
                message: Some(proto::client_message::Message::Array(proto::Array { values })),
            },
            ClientMessage::None => proto::ClientMessage {
                message: Some(proto::client_message::Message::None(())),
            },
            ClientMessage::Error(message) => proto::ClientMessage {
                message: Some(proto::client_message::Message::Error(message)),
            },
            ClientMessage::Abort => proto::ClientMessage {
                message: Some(proto::client_message::Message::Abort(())),
            },
            ClientMessage::Initialize {
                answers_yaml,
                switches,
                use_defaults,
                use_defaults_all,
                destination,
            } => proto::ClientMessage {
                message: Some(proto::client_message::Message::Initialize(proto::Initialize {
                    answers_yaml,
                    switches,
                    use_defaults,
                    use_defaults_all,
                    destination,
                })),
            },
            ClientMessage::Ack => proto::ClientMessage {
                message: Some(proto::client_message::Message::Ack(())),
            },
        }
    }
}

impl From<proto::ScriptMessage> for ScriptMessage {
    fn from(value: proto::ScriptMessage) -> Self {
        match value.message.expect("Valid Message Required") {
            Message::LogTrace(message) => ScriptMessage::LogTrace(message),
            Message::LogDebug(message) => ScriptMessage::LogDebug(message),
            Message::LogInfo(message) => ScriptMessage::LogInfo(message),
            Message::LogWarn(message) => ScriptMessage::LogWarn(message),
            Message::LogError(message) => ScriptMessage::LogError(message),
            Message::Print(message) => ScriptMessage::Print(message),
            Message::Display(message) => ScriptMessage::Display(message),
            Message::PromptForText(prompt) => {
                let info = TextPromptInfo {
                    message: prompt.message,
                    key: prompt.key,
                    default: prompt.default,
                    min: prompt.min.map(|v| v as i64),
                    max: prompt.max.map(|v| v as i64),
                    help: prompt.help,
                    placeholder: prompt.placeholder,
                    optional: prompt.optional,
                };
                ScriptMessage::PromptForText(info)
            }
            Message::PromptForInt(prompt) => ScriptMessage::PromptForInt(IntPromptInfo {
                message: prompt.message,
                key: prompt.key,
                default: prompt.default,
                min: prompt.min,
                max: prompt.max,
                help: prompt.help,
                placeholder: prompt.placeholder,
                optional: prompt.optional,
            }),
            Message::PromptForSelect(prompt) => {
                let info = SelectPromptInfo::new(prompt.message, prompt.key, prompt.options)
                    .with_default(prompt.default)
                    .with_help(prompt.help)
                    .with_placeholder(prompt.placeholder)
                    .with_optional(prompt.optional);
                ScriptMessage::PromptForSelect(info)
            }
            Message::PromptForMultiSelect(prompt) => {
                let info = MultiSelectPromptInfo::new(prompt.message, prompt.key, prompt.options)
                    .with_defaults(prompt.defaults.map(|v| v.values))
                    .with_help(prompt.help)
                    .with_placeholder(prompt.placeholder)
                    .with_min_items(prompt.min_items.map(|v| v as usize))
                    .with_max_items(prompt.max_items.map(|v| v as usize))
                    .with_page_size(prompt.page_size.map(|v| v as usize))
                    .with_optional(prompt.optional);
                ScriptMessage::PromptForMultiSelect(info)
            }
            Message::PromptForBool(prompt) => {
                let info = BoolPromptInfo::new(prompt.message, prompt.key)
                    .with_default(prompt.default)
                    .with_help(prompt.help)
                    .with_placeholder(prompt.placeholder)
                    .with_optional(prompt.optional);
                ScriptMessage::PromptForBool(info)
            }
            Message::PromptForList(prompt) => {
                let info = ListPromptInfo::new(prompt.message, prompt.key)
                    .with_defaults(prompt.defaults.map(|v| v.values))
                    .with_help(prompt.help)
                    .with_placeholder(prompt.placeholder)
                    .with_min_items(prompt.min_items.map(|v| v as usize))
                    .with_max_items(prompt.max_items.map(|v| v as usize))
                    .with_optional(prompt.optional);
                ScriptMessage::PromptForList(info)
            }
            Message::PromptForEditor(prompt) => {
                let info = EditorPromptInfo::new(prompt.message, prompt.key)
                    .with_default(prompt.default)
                    .with_help(prompt.help)
                    .with_placeholder(prompt.placeholder)
                    .with_min(prompt.min.map(|v| v as usize))
                    .with_max(prompt.max.map(|v| v as usize));
                ScriptMessage::PromptForEditor(info)
            }
            Message::CompleteSuccess(_message) => ScriptMessage::CompleteSuccess,
            Message::CompleteError(error) => ScriptMessage::CompleteError { message: error.message },
            Message::WriteFile(prompt_info) => {
                let info = WriteFileInfo {
                    destination: prompt_info.destination,
                    contents: prompt_info.contents,
                    existing_file_policy: proto::ExistingFilePolicy::try_from(prompt_info.existing_files)
                        .unwrap_or(proto::ExistingFilePolicy::Preserve)
                        .into(),
                };
                ScriptMessage::WriteFile(info)
            }
            Message::WriteDirectory(prompt_info) => {
                let info = WriteDirectoryInfo { path: prompt_info.path };
                ScriptMessage::WriteDirectory(info)
            }
        }
    }
}

impl From<proto::ExistingFilePolicy> for ExistingFilePolicy {
    fn from(value: proto::ExistingFilePolicy) -> Self {
        match value {
            proto::ExistingFilePolicy::Unspecified => ExistingFilePolicy::Preserve,
            proto::ExistingFilePolicy::Preserve => ExistingFilePolicy::Preserve,
            proto::ExistingFilePolicy::Overwrite => ExistingFilePolicy::Overwrite,
            proto::ExistingFilePolicy::Prompt => ExistingFilePolicy::Prompt,
        }
    }
}

impl From<ExistingFilePolicy> for proto::ExistingFilePolicy {
    fn from(value: ExistingFilePolicy) -> Self {
        match value {
            ExistingFilePolicy::Overwrite => proto::ExistingFilePolicy::Overwrite,
            ExistingFilePolicy::Preserve => proto::ExistingFilePolicy::Preserve,
            ExistingFilePolicy::Prompt => proto::ExistingFilePolicy::Prompt,
        }
    }
}

impl From<ScriptMessage> for proto::ScriptMessage {
    fn from(value: ScriptMessage) -> Self {
        match value {
            ScriptMessage::PromptForText(info) => proto::ScriptMessage {
                message: Some(Message::PromptForText(proto::PromptForText {
                    message: info.message,
                    key: info.key,
                    default: info.default,
                    min: info.min.map(|v| v as u32),
                    max: info.max.map(|v| v as u32),
                    help: info.help,
                    placeholder: info.placeholder,
                    optional: info.optional,
                })),
            },
            ScriptMessage::PromptForInt(info) => proto::ScriptMessage {
                message: Some(Message::PromptForInt(proto::PromptForInt {
                    message: info.message,
                    key: info.key,
                    default: info.default,
                    min: info.min,
                    max: info.max,
                    help: info.help,
                    placeholder: info.placeholder,
                    optional: info.optional,
                })),
            },
            ScriptMessage::PromptForBool(info) => proto::ScriptMessage {
                message: Some(Message::PromptForBool(proto::PromptForBool {
                    message: info.message,
                    key: info.key,
                    default: info.default,
                    help: info.help,
                    placeholder: info.placeholder,
                    optional: info.optional,
                })),
            },
            ScriptMessage::PromptForList(info) => proto::ScriptMessage {
                message: Some(Message::PromptForList(proto::PromptForList {
                    message: info.message,
                    key: info.key,
                    defaults: to_array(info.defaults),
                    help: info.help,
                    placeholder: info.placeholder,
                    min_items: info.min_items.map(|v| v as u32),
                    max_items: info.max_items.map(|v| v as u32),
                    optional: info.optional,
                })),
            },
            ScriptMessage::PromptForSelect(info) => proto::ScriptMessage {
                message: Some(Message::PromptForSelect(proto::PromptForSelect {
                    message: info.message().to_string(),
                    key: info.key().map(|k| k.to_string()),
                    options: info.options().iter().map(|v| v.to_string()).collect(),
                    default: info.default(),
                    help: info.help().map(|v| v.to_string()),
                    placeholder: info.placeholder().map(|v| v.to_string()),
                    optional: info.optional(),
                    page_size: info.page_size().map(|v| v as u32),
                })),
            },
            ScriptMessage::PromptForMultiSelect(info) => proto::ScriptMessage {
                message: Some(Message::PromptForMultiSelect(proto::PromptForMultiSelect {
                    message: info.message,
                    key: info.key,
                    options: info.options,
                    defaults: to_array(info.defaults),
                    help: info.help,
                    placeholder: info.placeholder,
                    min_items: info.min_items.map(|v| v as u32),
                    max_items: info.max_items.map(|v| v as u32),
                    page_size: info.page_size.map(|v| v as u32),
                    optional: info.optional,
                })),
            },
            ScriptMessage::PromptForEditor(info) => proto::ScriptMessage {
                message: Some(Message::PromptForEditor(proto::PromptForEditor {
                    message: info.message,
                    key: info.key,
                    default: info.default,
                    min: info.min.map(|v| v as u64), // TODO: Fix type
                    max: info.max.map(|v| v as u64), // TODO: Fix type
                    help: info.help,
                    placeholder: info.placeholder,
                    optional: info.optional,
                })),
            },
            ScriptMessage::LogTrace(message) => proto::ScriptMessage {
                message: Some(Message::LogTrace(message)),
            },
            ScriptMessage::LogDebug(message) => proto::ScriptMessage {
                message: Some(Message::LogDebug(message)),
            },
            ScriptMessage::LogInfo(message) => proto::ScriptMessage {
                message: Some(Message::LogInfo(message)),
            },
            ScriptMessage::LogWarn(message) => proto::ScriptMessage {
                message: Some(Message::LogWarn(message)),
            },
            ScriptMessage::LogError(message) => proto::ScriptMessage {
                message: Some(Message::LogError(message)),
            },
            ScriptMessage::Print(message) => proto::ScriptMessage {
                message: Some(Message::Print(message)),
            },
            ScriptMessage::Display(message) => proto::ScriptMessage {
                message: Some(Message::Display(message)),
            },
            ScriptMessage::CompleteSuccess => proto::ScriptMessage {
                message: Some(Message::CompleteSuccess(proto::CompleteSuccess::default())),
            },
            ScriptMessage::CompleteError { message } => proto::ScriptMessage {
                message: Some(Message::CompleteError(proto::CompleteError { message })),
            },
            ScriptMessage::WriteFile(info) => proto::ScriptMessage {
                message: Some(Message::WriteFile(proto::WriteFile {
                    destination: info.destination,
                    contents: info.contents,
                    existing_files: proto::ExistingFilePolicy::from(info.existing_file_policy).into(),
                })),
            },
            ScriptMessage::WriteDirectory(info) => proto::ScriptMessage {
                message: Some(Message::WriteDirectory(WriteDirectory { path: info.path })),
            },
        }
    }
}

fn to_array(values: Option<Vec<String>>) -> Option<proto::Array> {
    if let Some(values) = values {
        Some(proto::Array { values })
    } else {
        None
    }
}

use archetect_api::{
    BoolPromptInfo, EditorPromptInfo, ExistingFilePolicy, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, SelectPromptInfo, TextPromptInfo, WriteDirectoryInfo, WriteFileInfo,
};

use super::grpc;
use archetect_api::ClientMessage as ApiClientMessage;
use archetect_api::ScriptMessage as ApiScriptMessage;

// --- ScriptMessage: API -> Proto ---

impl From<ApiScriptMessage> for grpc::ScriptMessage {
    fn from(value: ApiScriptMessage) -> Self {
        grpc::ScriptMessage {
            message: Some(value.into()),
        }
    }
}

impl From<ApiScriptMessage> for grpc::script_message::Message {
    fn from(value: ApiScriptMessage) -> Self {
        use grpc::script_message::Message;
        match value {
            ApiScriptMessage::PromptForText(info) => Message::PromptForText(grpc::PromptForText {
                message: info.message,
                key: info.key,
                default: info.default,
                min: info.min.map(|v| v as u32),
                max: info.max.map(|v| v as u32),
                help: info.help,
                placeholder: info.placeholder,
                optional: info.optional,
            }),
            ApiScriptMessage::PromptForInt(info) => Message::PromptForInt(grpc::PromptForInt {
                message: info.message,
                key: info.key,
                default: info.default,
                min: info.min,
                max: info.max,
                help: info.help,
                placeholder: info.placeholder,
                optional: info.optional,
            }),
            ApiScriptMessage::PromptForBool(info) => Message::PromptForBool(grpc::PromptForBool {
                message: info.message,
                key: info.key,
                default: info.default,
                help: info.help,
                placeholder: info.placeholder,
                optional: info.optional,
            }),
            ApiScriptMessage::PromptForList(info) => Message::PromptForList(grpc::PromptForList {
                message: info.message,
                key: info.key,
                defaults: info.defaults.map(|v| grpc::Array { values: v }),
                help: info.help,
                placeholder: info.placeholder,
                min_items: info.min_items.map(|v| v as u32),
                max_items: info.max_items.map(|v| v as u32),
                optional: info.optional,
            }),
            ApiScriptMessage::PromptForSelect(info) => {
                Message::PromptForSelect(grpc::PromptForSelect {
                    message: info.message,
                    key: info.key,
                    options: info.options,
                    default: info.default,
                    help: info.help,
                    placeholder: info.placeholder,
                    page_size: info.page_size.map(|v| v as u32),
                    optional: info.optional,
                    allow_other: info.allow_other,
                    other_label: info.other_label,
                })
            }
            ApiScriptMessage::PromptForMultiSelect(info) => {
                Message::PromptForMultiSelect(grpc::PromptForMultiSelect {
                    message: info.message,
                    key: info.key,
                    options: info.options,
                    defaults: info.defaults.map(|v| grpc::Array { values: v }),
                    help: info.help,
                    placeholder: info.placeholder,
                    min_items: info.min_items.map(|v| v as u32),
                    max_items: info.max_items.map(|v| v as u32),
                    page_size: info.page_size.map(|v| v as u32),
                    optional: info.optional,
                })
            }
            ApiScriptMessage::PromptForEditor(info) => {
                Message::PromptForEditor(grpc::PromptForEditor {
                    message: info.message,
                    key: info.key,
                    default: info.default,
                    min: info.min.map(|v| v as u64),
                    max: info.max.map(|v| v as u64),
                    help: info.help,
                    placeholder: info.placeholder,
                    optional: info.optional,
                })
            }
            ApiScriptMessage::LogTrace(msg) => Message::LogTrace(msg),
            ApiScriptMessage::LogDebug(msg) => Message::LogDebug(msg),
            ApiScriptMessage::LogInfo(msg) => Message::LogInfo(msg),
            ApiScriptMessage::LogWarn(msg) => Message::LogWarn(msg),
            ApiScriptMessage::LogError(msg) => Message::LogError(msg),
            ApiScriptMessage::Print(msg) => Message::Print(msg),
            ApiScriptMessage::Display(msg) => Message::Display(msg),
            ApiScriptMessage::CompleteSuccess => {
                Message::CompleteSuccess(grpc::CompleteSuccess {})
            }
            ApiScriptMessage::CompleteError(message) => {
                Message::CompleteError(grpc::CompleteError { message })
            }
            ApiScriptMessage::WriteFile(info) => Message::WriteFile(grpc::WriteFile {
                destination: info.destination,
                contents: info.contents,
                existing_files: api_policy_to_proto(info.existing_file_policy).into(),
            }),
            ApiScriptMessage::WriteDirectory(info) => {
                Message::WriteDirectory(grpc::WriteDirectory { path: info.path })
            }
        }
    }
}

// --- ScriptMessage: Proto -> API ---

impl From<grpc::ScriptMessage> for ApiScriptMessage {
    fn from(value: grpc::ScriptMessage) -> Self {
        use grpc::script_message::Message;
        match value.message.expect("Valid ScriptMessage required") {
            Message::LogTrace(msg) => ApiScriptMessage::LogTrace(msg),
            Message::LogDebug(msg) => ApiScriptMessage::LogDebug(msg),
            Message::LogInfo(msg) => ApiScriptMessage::LogInfo(msg),
            Message::LogWarn(msg) => ApiScriptMessage::LogWarn(msg),
            Message::LogError(msg) => ApiScriptMessage::LogError(msg),
            Message::Print(msg) => ApiScriptMessage::Print(msg),
            Message::Display(msg) => ApiScriptMessage::Display(msg),
            Message::PromptForText(p) => ApiScriptMessage::PromptForText(TextPromptInfo {
                message: p.message,
                key: p.key,
                default: p.default,
                min: p.min.map(|v| v as i64),
                max: p.max.map(|v| v as i64),
                help: p.help,
                placeholder: p.placeholder,
                optional: p.optional,
            }),
            Message::PromptForInt(p) => ApiScriptMessage::PromptForInt(IntPromptInfo {
                message: p.message,
                key: p.key,
                default: p.default,
                min: p.min,
                max: p.max,
                help: p.help,
                placeholder: p.placeholder,
                optional: p.optional,
            }),
            Message::PromptForBool(p) => ApiScriptMessage::PromptForBool(BoolPromptInfo {
                message: p.message,
                key: p.key,
                default: p.default,
                help: p.help,
                placeholder: p.placeholder,
                optional: p.optional,
            }),
            Message::PromptForList(p) => ApiScriptMessage::PromptForList(ListPromptInfo {
                message: p.message,
                key: p.key,
                defaults: p.defaults.map(|a| a.values),
                help: p.help,
                placeholder: p.placeholder,
                min_items: p.min_items.map(|v| v as usize),
                max_items: p.max_items.map(|v| v as usize),
                optional: p.optional,
            }),
            Message::PromptForSelect(p) => ApiScriptMessage::PromptForSelect(SelectPromptInfo {
                message: p.message,
                key: p.key,
                options: p.options,
                default: p.default,
                help: p.help,
                placeholder: p.placeholder,
                page_size: p.page_size.map(|v| v as usize),
                optional: p.optional,
                allow_other: p.allow_other,
                other_label: p.other_label,
            }),
            Message::PromptForMultiSelect(p) => {
                ApiScriptMessage::PromptForMultiSelect(MultiSelectPromptInfo {
                    message: p.message,
                    key: p.key,
                    options: p.options,
                    defaults: p.defaults.map(|a| a.values),
                    help: p.help,
                    placeholder: p.placeholder,
                    min_items: p.min_items.map(|v| v as usize),
                    max_items: p.max_items.map(|v| v as usize),
                    page_size: p.page_size.map(|v| v as usize),
                    optional: p.optional,
                })
            }
            Message::PromptForEditor(p) => ApiScriptMessage::PromptForEditor(EditorPromptInfo {
                message: p.message,
                key: p.key,
                default: p.default,
                min: p.min.map(|v| v as i64),
                max: p.max.map(|v| v as i64),
                help: p.help,
                placeholder: p.placeholder,
                optional: p.optional,
            }),
            Message::CompleteSuccess(_) => ApiScriptMessage::CompleteSuccess,
            Message::CompleteError(e) => ApiScriptMessage::CompleteError(e.message),
            Message::WriteFile(wf) => ApiScriptMessage::WriteFile(WriteFileInfo {
                destination: wf.destination,
                contents: wf.contents,
                existing_file_policy: proto_policy_to_api(wf.existing_files),
            }),
            Message::WriteDirectory(wd) => {
                ApiScriptMessage::WriteDirectory(WriteDirectoryInfo { path: wd.path })
            }
        }
    }
}

// --- ClientMessage: API -> Proto ---

impl From<ApiClientMessage> for grpc::ClientMessage {
    fn from(value: ApiClientMessage) -> Self {
        use grpc::client_message::Message;
        let message = match value {
            ApiClientMessage::String(v) => Message::String(v),
            ApiClientMessage::Integer(v) => Message::Integer(v),
            ApiClientMessage::Boolean(v) => Message::Boolean(v),
            ApiClientMessage::Array(values) => Message::Array(grpc::Array { values }),
            ApiClientMessage::None => Message::None(()),
            ApiClientMessage::Error(msg) => Message::Error(msg),
            ApiClientMessage::Abort => Message::Abort(()),
            ApiClientMessage::Ack => Message::Ack(()),
            ApiClientMessage::Initialize {
                answers_yaml,
                switches,
                use_defaults,
                use_defaults_all,
                destination,
            } => Message::Initialize(grpc::Initialize {
                answers_yaml,
                switches,
                use_defaults,
                use_defaults_all,
                destination,
            }),
        };
        grpc::ClientMessage {
            message: Some(message),
        }
    }
}

// --- ClientMessage: Proto -> API ---

impl From<grpc::ClientMessage> for ApiClientMessage {
    fn from(value: grpc::ClientMessage) -> Self {
        use grpc::client_message::Message;
        match value.message.expect("Valid ClientMessage required") {
            Message::Initialize(init) => ApiClientMessage::Initialize {
                answers_yaml: init.answers_yaml,
                switches: init.switches,
                use_defaults: init.use_defaults,
                use_defaults_all: init.use_defaults_all,
                destination: init.destination,
            },
            Message::String(v) => ApiClientMessage::String(v),
            Message::Integer(v) => ApiClientMessage::Integer(v),
            Message::Boolean(v) => ApiClientMessage::Boolean(v),
            Message::Error(msg) => ApiClientMessage::Error(msg),
            Message::Array(a) => ApiClientMessage::Array(a.values),
            Message::None(_) => ApiClientMessage::None,
            Message::Abort(_) => ApiClientMessage::Abort,
            Message::Ack(_) => ApiClientMessage::Ack,
        }
    }
}

// --- ExistingFilePolicy helpers ---

fn api_policy_to_proto(policy: ExistingFilePolicy) -> grpc::ExistingFilePolicy {
    match policy {
        ExistingFilePolicy::Overwrite => grpc::ExistingFilePolicy::Overwrite,
        ExistingFilePolicy::Preserve => grpc::ExistingFilePolicy::Preserve,
        ExistingFilePolicy::Prompt => grpc::ExistingFilePolicy::Prompt,
    }
}

fn proto_policy_to_api(value: i32) -> ExistingFilePolicy {
    match grpc::ExistingFilePolicy::try_from(value) {
        Ok(grpc::ExistingFilePolicy::Overwrite) => ExistingFilePolicy::Overwrite,
        Ok(grpc::ExistingFilePolicy::Prompt) => ExistingFilePolicy::Prompt,
        _ => ExistingFilePolicy::Preserve,
    }
}

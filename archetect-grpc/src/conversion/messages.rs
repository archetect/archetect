use archetect_api::{ClientMessage, IntPromptInfo, ScriptMessage, TextPromptInfo};
use crate::proto;

impl From<proto::ClientMessage> for ClientMessage {
    fn from(value: proto::ClientMessage) -> Self {
        match value.message.expect("Valid Message") {
            proto::client_message::Message::Initialize(value) => {
                ClientMessage::Initialize {
                    answers: value.answers,
                }
            }
            proto::client_message::Message::String(value) => {
                ClientMessage::String(value)
            }
            proto::client_message::Message::Integer(value) => {
                ClientMessage::Integer(value)
            }
            proto::client_message::Message::Boolean(value) => {
                ClientMessage::Boolean(value)
            }
            proto::client_message::Message::Error(message) => {
                ClientMessage::Error(message)
            }
            proto::client_message::Message::Array(array) => {
                ClientMessage::Array(array.values)
            }
            proto::client_message::Message::None(_unit) => {
                ClientMessage::None
            }
            proto::client_message::Message::Abort(_unit) => {
                ClientMessage::Abort
            }
        }
    }
}

impl From<ClientMessage> for proto::ClientMessage {
    fn from(value: ClientMessage) -> Self {
        match value {
            ClientMessage::String(value) => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::String(value)
                    )
                }
            }
            ClientMessage::Integer(value) => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Integer(value)
                    )
                }
            }
            ClientMessage::Boolean(value) => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Boolean(value)
                    )
                }
            }
            ClientMessage::Array(values) => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Array(
                            proto::Array {
                                values
                            }
                        )
                    )
                }
            }
            ClientMessage::None => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::None(())
                    )
                }
            }
            ClientMessage::Error(message) => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Error(message)
                    )
                }
            }
            ClientMessage::Abort => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Abort(())
                    )
                }
            }
            ClientMessage::Initialize { answers } => {
                proto::ClientMessage {
                    message: Some(
                        proto::client_message::Message::Initialize(
                            proto::Initialize {
                                answers,
                            }
                        )
                    ),
                }
            }
        }
    }
}

impl From<proto::ScriptMessage> for ScriptMessage {
    fn from(value: proto::ScriptMessage) -> Self {
        match value.message.expect("Valid Message Required") {
            proto::script_message::Message::PromptForText(prompt) => {
                ScriptMessage::PromptForText(
                    TextPromptInfo::new(prompt.message, prompt.key)
                )
            }
            proto::script_message::Message::PromptForInt(prompt) => {
                ScriptMessage::PromptForInt(
                    IntPromptInfo::new(prompt.message, prompt.key)
                )
            }
            proto::script_message::Message::LogTrace(message) => {
                ScriptMessage::LogTrace(message)
            }
            proto::script_message::Message::LogDebug(message) => {
                ScriptMessage::LogDebug(message)
            }
            proto::script_message::Message::LogInfo(message) => {
                ScriptMessage::LogInfo(message)
            }
            proto::script_message::Message::LogWarn(message) => {
                ScriptMessage::LogWarn(message)
            }
            proto::script_message::Message::LogError(message) => {
                ScriptMessage::LogError(message)
            }
            proto::script_message::Message::Print(message) => {
                ScriptMessage::Print(message)
            }
            proto::script_message::Message::Display(message) => {
                ScriptMessage::Display(message)
            }
        }
    }
}

impl From<ScriptMessage> for proto::ScriptMessage {
    fn from(value: ScriptMessage) -> Self {
        match value {
            ScriptMessage::PromptForText(info) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::PromptForText(
                            proto::PromptForText {
                                message: info.message,
                                key: info.key,
                            }
                        )
                    )
                }
            }
            ScriptMessage::PromptForInt(_) => {
                todo!()
            }
            ScriptMessage::PromptForBool(_) => {
                todo!()
            }
            ScriptMessage::PromptForList(_) => {
                todo!()
            }
            ScriptMessage::PromptForSelect(_) => {
                todo!()
            }
            ScriptMessage::PromptForMultiSelect(_) => {
                todo!()
            }
            ScriptMessage::PromptForEditor(_) => {
                todo!()
            }
            ScriptMessage::LogTrace(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::LogTrace(message)
                    ),
                }
            }
            ScriptMessage::LogDebug(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::LogDebug(message)
                    )
                }
            }
            ScriptMessage::LogInfo(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::LogInfo(message)
                    )
                }
            }
            ScriptMessage::LogWarn(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::LogWarn(message)
                    )
                }
            }
            ScriptMessage::LogError(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::LogError(message)
                    )
                }
            }
            ScriptMessage::Print(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::Print(message)
                    )
                }
            }
            ScriptMessage::Display(message) => {
                proto::ScriptMessage {
                    message: Some(
                        proto::script_message::Message::Display(message)
                    )
                }
            }
        }
    }
}

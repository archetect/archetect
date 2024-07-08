use rhai::{EvalAltResult, NativeCallContext};

use archetect_api::{ClientMessage, PromptInfo};
use ArchetypeScriptError::{
    AnswerNotOptional, AnswerTypeError, AnswerValidationError, DefaultTypeError, DefaultValidationError,
    HeadlessNoAnswer, InvalidPromptSetting, KeyedAnswerNotOptional, KeyedAnswerTypeError, KeyedAnswerValidationError,
    KeyedDefaultTypeError, KeyedDefaultValidationError, KeyedHeadlessNoAnswer, KeyedInvalidPromptSetting,
    KeyedUnexpectedPromptResponse, PromptError, UnexpectedPromptResponse,
};

use crate::errors::ArchetectIoDriverError;
use crate::errors::ArchetypeScriptError::KeyedInvalidSetSetting;

#[derive(Debug, thiserror::Error)]
pub enum ArchetypeScriptError {
    #[error("'{answer}' was provided as an answer to '{prompt}', but {requires}.")]
    AnswerValidationError {
        answer: String,
        prompt: String,
        requires: String,
    },
    #[error("'{answer}' was provided as an answer to '{prompt}' (key: '{key}'), but {requires}.")]
    KeyedAnswerValidationError {
        answer: String,
        prompt: String,
        key: String,
        requires: String,
    },
    #[error("'{answer}' was provided as an answer to '{prompt}', but mut be {requires}.")]
    AnswerTypeError {
        answer: String,
        prompt: String,
        requires: String,
    },
    #[error("'{answer}' was provided as an answer to '{prompt}' (key: '{key}'), but must be {requires}")]
    KeyedAnswerTypeError {
        answer: String,
        prompt: String,
        key: String,
        requires: String,
    },
    #[error("'{default}' was provided as a default to '{prompt}', but {requires}")]
    DefaultValidationError {
        default: String,
        prompt: String,
        requires: String,
    },
    #[error("'{default}' was provided as a default to '{prompt}' (key: '{key}'), but {requires}")]
    KeyedDefaultValidationError {
        default: String,
        prompt: String,
        key: String,
        requires: String,
    },
    #[error("'{default}' was provided as a default to '{prompt}', but mut be {requires}.")]
    DefaultTypeError {
        default: String,
        prompt: String,
        requires: String,
    },
    #[error("'{default}' was provided as a default to '{prompt}' (key: '{key}'), but must be {requires}")]
    KeyedDefaultTypeError {
        default: String,
        prompt: String,
        key: String,
        requires: String,
    },
    #[error("'{prompt}' was not answered and does not have a default value")]
    HeadlessNoAnswer { prompt: String },
    #[error("'{prompt}' (key: '{key}') was not answered and does not have a default value")]
    KeyedHeadlessNoAnswer { prompt: String, key: String },
    #[error("'{prompt}' is not optional")]
    AnswerNotOptional { prompt: String },
    #[error("'{prompt}' (key: '{key}') is not optional")]
    KeyedAnswerNotOptional { prompt: String, key: String },
    #[error("For the '{prompt}' prompt, the '{setting}' setting must be {requirement}")]
    InvalidPromptSetting {
        prompt: String,
        setting: String,
        requirement: String,
    },
    #[error("For the '{prompt}' prompt (key: '{key}'), the '{setting}' setting must be {requirement}")]
    KeyedInvalidPromptSetting {
        prompt: String,
        setting: String,
        requirement: String,
        key: String,
    },
    #[error("For 'set' (key: '{key}', the '{setting}' setting must be {requirement}")]
    KeyedInvalidSetSetting {
        setting: String,
        requirement: String,
        key: String,
    },
    #[error("{0}")]
    PromptError(String),
    #[error("When supplying a destination to a 'render', the destination must be either a String or a Path, but '{actual}' was provided")]
    RenderDestinationTypeError { actual: String },
    #[error("The '{prompt}' prompt expects {expected}, but received {actual}")]
    UnexpectedPromptResponse {
        prompt: String,
        expected: String,
        actual: String,
    },
    #[error("'{prompt}' (key: '{key}' expects {expected}, but received {actual}")]
    KeyedUnexpectedPromptResponse {
        prompt: String,
        expected: String,
        actual: String,
        key: String,
    },
    #[error("The path '{path}' contains path manipulation patterns, which are not allowed. Rendering and other file operations are restricted to the destination directory")]
    PathManipulationError { path: String },
    #[error(transparent)]
    ArchetectClientError(ArchetectIoDriverError),
}

impl ArchetypeScriptError {
    pub fn title(&self) -> &'static str {
        match self {
            AnswerValidationError { .. } | KeyedAnswerValidationError { .. } => "Answer Invalid",
            AnswerTypeError { .. } | KeyedAnswerTypeError { .. } => "Answer Type",
            DefaultValidationError { .. } | KeyedDefaultValidationError { .. } => "Default Invalid",
            HeadlessNoAnswer { .. } | KeyedHeadlessNoAnswer { .. } => "Headless Mode",
            AnswerNotOptional { .. } | KeyedAnswerNotOptional { .. } => "Required",
            InvalidPromptSetting { .. } | KeyedInvalidPromptSetting { .. } => "Invalid Setting",
            KeyedInvalidSetSetting { .. } => "Invalid Setting",
            DefaultTypeError { .. } | KeyedDefaultTypeError { .. } => "Default Type",
            PromptError(_) => "Prompt Error",
            UnexpectedPromptResponse { .. } | KeyedUnexpectedPromptResponse { .. } => "Unexpected Response",
            ArchetypeScriptError::RenderDestinationTypeError { .. } => "Invalid Destination",
            ArchetypeScriptError::PathManipulationError { .. } => "Path Error",
            ArchetypeScriptError::ArchetectClientError(..) => "Client Error",
        }
    }

    pub fn error_type(&self) -> ErrorType {
        match self {
            AnswerValidationError { .. } | KeyedAnswerValidationError { .. } => ErrorType::Function,
            AnswerTypeError { .. } | KeyedAnswerTypeError { .. } => ErrorType::Function,
            DefaultValidationError { .. } | KeyedDefaultValidationError { .. } => ErrorType::Function,
            HeadlessNoAnswer { .. } | KeyedHeadlessNoAnswer { .. } => ErrorType::Function,
            AnswerNotOptional { .. } | KeyedAnswerNotOptional { .. } => ErrorType::Function,
            InvalidPromptSetting { .. } | KeyedInvalidPromptSetting { .. } => ErrorType::Function,
            DefaultTypeError { .. } | KeyedDefaultTypeError { .. } => ErrorType::Function,
            PromptError(_) => ErrorType::System,
            UnexpectedPromptResponse { .. } | KeyedUnexpectedPromptResponse { .. } => ErrorType::Function,
            KeyedInvalidSetSetting { .. } => ErrorType::Function,
            ArchetypeScriptError::RenderDestinationTypeError { .. } => ErrorType::Function,
            ArchetypeScriptError::PathManipulationError { .. } => ErrorType::Function,
            ArchetypeScriptError::ArchetectClientError(..) => ErrorType::System,
        }
    }

    pub fn answer_validation_error<'a, D, P, R>(answer: D, prompt: &P, requirement: R) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: PromptInfo,
        R: Into<String>,
    {
        if let Some(key) = prompt.key() {
            KeyedAnswerValidationError {
                answer: answer.into(),
                prompt: prompt.message().to_string(),
                key: key.to_string(),
                requires: requirement.into(),
            }
        } else {
            AnswerValidationError {
                answer: answer.into(),
                prompt: prompt.message().to_string(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_type_error<'a, D, P, R>(answer: D, prompt: &P, requirement: R) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: PromptInfo,
        R: Into<String>,
    {
        if let Some(key) = prompt.key() {
            KeyedAnswerTypeError {
                answer: answer.into(),
                prompt: prompt.message().to_string(),
                key: key.to_string(),
                requires: requirement.into(),
            }
        } else {
            AnswerTypeError {
                answer: answer.into(),
                prompt: prompt.message().to_string(),
                requires: requirement.into(),
            }
        }
    }

    pub fn default_validation_error<'a, D, P, R>(default: D, prompt: &P, requirement: R) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: PromptInfo,
        R: Into<String>,
    {
        if let Some(key) = prompt.key() {
            KeyedDefaultValidationError {
                default: default.into(),
                prompt: prompt.message().to_string(),
                key: key.to_string(),
                requires: requirement.into(),
            }
        } else {
            DefaultValidationError {
                default: default.into(),
                prompt: prompt.message().to_string(),
                requires: requirement.into(),
            }
        }
    }

    pub fn default_type_error<'a, D, P, R>(default: D, prompt: &P, requirement: R) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: PromptInfo,
        R: Into<String>,
    {
        if let Some(key) = prompt.key() {
            KeyedDefaultTypeError {
                default: default.into(),
                prompt: prompt.message().to_string(),
                key: key.to_string(),
                requires: requirement.into(),
            }
        } else {
            DefaultTypeError {
                default: default.into(),
                prompt: prompt.message().to_string(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_not_optional<'a, P>(prompt: &P) -> ArchetypeScriptError
    where
        P: PromptInfo,
    {
        if let Some(key) = prompt.key() {
            KeyedAnswerNotOptional {
                prompt: prompt.message().to_string(),
                key: key.to_string(),
            }
        } else {
            AnswerNotOptional {
                prompt: prompt.message().to_string(),
            }
        }
    }

    pub fn headless_no_answer<'a, P>(prompt: &P) -> ArchetypeScriptError
    where
        P: PromptInfo,
    {
        if let Some(key) = prompt.key() {
            KeyedHeadlessNoAnswer {
                prompt: prompt.message().to_string(),
                key: key.to_string(),
            }
        } else {
            HeadlessNoAnswer {
                prompt: prompt.message().to_string(),
            }
        }
    }

    pub fn invalid_promptinfo_setting<'a, P, S, R>(prompt: &P, setting: S, requirement: R) -> ArchetypeScriptError
    where
        P: PromptInfo,
        S: Into<String>,
        R: Into<String>,
    {
        ArchetypeScriptError::invalid_prompt_setting(prompt.message(), prompt.key(), setting, requirement)
    }

    pub fn invalid_prompt_setting<'a, P, K, S, R>(
        prompt: P,
        key: Option<K>,
        setting: S,
        requirement: R,
    ) -> ArchetypeScriptError
    where
        P: AsRef<str>,
        K: AsRef<str>,
        S: Into<String>,
        R: Into<String>,
    {
        if let Some(key) = key {
            KeyedInvalidPromptSetting {
                prompt: prompt.as_ref().to_string(),
                setting: setting.into(),
                requirement: requirement.into(),
                key: key.as_ref().to_string(),
            }
        } else {
            InvalidPromptSetting {
                prompt: prompt.as_ref().to_string(),
                setting: setting.into(),
                requirement: requirement.into(),
            }
        }
    }

    pub fn unexpected_prompt_response<'a, P, E>(prompt: &P, expected: E, actual: ClientMessage) -> ArchetypeScriptError
    where
        P: PromptInfo,
        E: Into<String>,
    {
        if let Some(key) = prompt.key() {
            KeyedUnexpectedPromptResponse {
                prompt: prompt.message().to_string(),
                expected: expected.into(),
                actual: format!("{:?}", actual),
                key: key.to_string(),
            }
        } else {
            UnexpectedPromptResponse {
                prompt: prompt.message().to_string(),
                expected: expected.into(),
                actual: format!("{:?}", actual),
            }
        }
    }
}

pub struct ArchetypeScriptErrorWrapper<'a>(pub &'a NativeCallContext<'a>, pub ArchetypeScriptError);

impl<'a> From<ArchetypeScriptErrorWrapper<'a>> for Box<EvalAltResult> {
    fn from(value: ArchetypeScriptErrorWrapper<'a>) -> Self {
        match value.1.error_type() {
            ErrorType::Function => {
                let fn_name = value.0.fn_name().to_owned();
                let source = value
                    .0
                    .source()
                    .unwrap_or_else(|| value.0.global_runtime_state().source().unwrap_or("<unknown>"))
                    .to_owned();
                let position = value.0.position();
                let error = EvalAltResult::ErrorSystem(value.1.title().to_string(), Box::new(value.1));
                Box::new(EvalAltResult::ErrorInFunctionCall(
                    fn_name,
                    source,
                    Box::new(error),
                    position,
                ))
            }
            ErrorType::System => Box::new(EvalAltResult::ErrorSystem(
                value.1.title().to_string(),
                Box::new(value.1),
            )),
        }
    }
}

#[derive(PartialOrd, PartialEq, Clone, Debug)]
pub enum ErrorType {
    Function,
    System,
}

use rhai::{EvalAltResult, NativeCallContext};

use archetect_api::CommandResponse;
use ArchetypeScriptError::{
    AnswerNotOptional, AnswerTypeError, AnswerValidationError, DefaultTypeError, DefaultValidationError,
    HeadlessNoAnswer, InvalidSetting, KeyedAnswerNotOptional, KeyedAnswerTypeError, KeyedAnswerValidationError,
    KeyedDefaultTypeError, KeyedDefaultValidationError, KeyedHeadlessNoAnswer, KeyedInvalidSetting,
    KeyedUnexpectedPromptResponse, PromptError, UnexpectedPromptResponse,
};

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
    #[error("For '{prompt}', the '{setting}' setting must be {requirement}")]
    InvalidSetting {
        prompt: String,
        setting: String,
        requirement: String,
    },
    #[error("For '{prompt}' (key: '{key}'), the '{setting}' setting must be {requirement}")]
    KeyedInvalidSetting {
        prompt: String,
        setting: String,
        requirement: String,
        key: String,
    },
    #[error("{0}")]
    PromptError(String),
    #[error("'{prompt}' expects {expected}, but received {actual}")]
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
}

impl ArchetypeScriptError {
    pub fn title(&self) -> &'static str {
        match self {
            AnswerValidationError { .. } | KeyedAnswerValidationError { .. } => "Answer Invalid",
            AnswerTypeError { .. } | KeyedAnswerTypeError { .. } => "Answer Type",
            DefaultValidationError { .. } | KeyedDefaultValidationError { .. } => "Default Invalid",
            HeadlessNoAnswer { .. } | KeyedHeadlessNoAnswer { .. } => "Headless Mode",
            AnswerNotOptional { .. } | KeyedAnswerNotOptional { .. } => "Required",
            InvalidSetting { .. } | KeyedInvalidSetting { .. } => "Invalid Setting",
            DefaultTypeError { .. } | KeyedDefaultTypeError { .. } => "Default Type",
            PromptError(_) => "Prompt Error",
            UnexpectedPromptResponse { .. } | KeyedUnexpectedPromptResponse { .. } => "Unexpected Response",
        }
    }

    pub fn error_type(&self) -> ErrorType {
        match self {
            AnswerValidationError { .. } | KeyedAnswerValidationError { .. } => ErrorType::Function,
            AnswerTypeError { .. } | KeyedAnswerTypeError { .. } => ErrorType::Function,
            DefaultValidationError { .. } | KeyedDefaultValidationError { .. } => ErrorType::Function,
            HeadlessNoAnswer { .. } | KeyedHeadlessNoAnswer { .. } => ErrorType::Function,
            AnswerNotOptional { .. } | KeyedAnswerNotOptional { .. } => ErrorType::Function,
            InvalidSetting { .. } | KeyedInvalidSetting { .. } => ErrorType::Function,
            DefaultTypeError { .. } | KeyedDefaultTypeError { .. } => ErrorType::Function,
            PromptError(_) => ErrorType::System,
            UnexpectedPromptResponse { .. } | KeyedUnexpectedPromptResponse { .. } => { ErrorType::Function}
        }
    }

    pub fn answer_validation_error<'a, D, P, K, R>(
        answer: D,
        prompt: P,
        key: Option<K>,
        requirement: R,
    ) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: Into<String>,
        K: AsRef<str>,
        R: Into<String>,
    {
        if let Some(key) = key {
            KeyedAnswerValidationError {
                answer: answer.into(),
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
                requires: requirement.into(),
            }
        } else {
            AnswerValidationError {
                answer: answer.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_type_error<'a, D, P, K, R>(
        answer: D,
        prompt: P,
        key: Option<K>,
        requirement: R,
    ) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: Into<String>,
        K: AsRef<str>,
        R: Into<String>,
    {
        if let Some(key) = key {
            KeyedAnswerTypeError {
                answer: answer.into(),
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
                requires: requirement.into(),
            }
        } else {
            AnswerTypeError {
                answer: answer.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn default_validation_error<'a, D, P, K, R>(
        default: D,
        prompt: P,
        key: Option<K>,
        requirement: R,
    ) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: Into<String>,
        K: AsRef<str>,
        R: Into<String>,
    {
        if let Some(key) = key {
            KeyedDefaultValidationError {
                default: default.into(),
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
                requires: requirement.into(),
            }
        } else {
            DefaultValidationError {
                default: default.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn default_type_error<'a, D, P, K, R>(
        default: D,
        prompt: P,
        key: Option<K>,
        requirement: R,
    ) -> ArchetypeScriptError
    where
        D: Into<String>,
        P: Into<String>,
        K: AsRef<str>,
        R: Into<String>,
    {
        if let Some(key) = key {
            KeyedDefaultTypeError {
                default: default.into(),
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
                requires: requirement.into(),
            }
        } else {
            DefaultTypeError {
                default: default.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_not_optional<'a, P, K>(prompt: P, key: Option<K>) -> ArchetypeScriptError
    where
        P: Into<String>,
        K: AsRef<str>,
    {
        if let Some(key) = key {
            KeyedAnswerNotOptional {
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
            }
        } else {
            AnswerNotOptional { prompt: prompt.into() }
        }
    }

    pub fn headless_no_answer<'a, P, K>(prompt: P, key: Option<K>) -> ArchetypeScriptError
    where
        P: Into<String>,
        K: AsRef<str>,
    {
        if let Some(key) = key {
            KeyedHeadlessNoAnswer {
                prompt: prompt.into(),
                key: key.as_ref().to_string(),
            }
        } else {
            HeadlessNoAnswer { prompt: prompt.into() }
        }
    }

    pub fn invalid_setting<'a, P, S, R, K>(
        prompt: P,
        setting: S,
        requirement: R,
        key: Option<K>,
    ) -> ArchetypeScriptError
    where
        P: Into<String>,
        S: Into<String>,
        R: Into<String>,
        K: AsRef<str>,
    {
        if let Some(key) = key {
            KeyedInvalidSetting {
                prompt: prompt.into(),
                setting: setting.into(),
                requirement: requirement.into(),
                key: key.as_ref().to_string(),
            }
        } else {
            InvalidSetting {
                prompt: prompt.into(),
                setting: setting.into(),
                requirement: requirement.into(),
            }
        }
    }

    pub fn unexpected_prompt_response<'a, P, K, E>(
        prompt: P,
        key: Option<K>,
        expected: E,
        actual: CommandResponse,
    ) -> ArchetypeScriptError
        where
            P: Into<String>,
            K: AsRef<str>,
            E: Into<String>,
    {
        if let Some(key) = key {
            KeyedUnexpectedPromptResponse {
                prompt: prompt.into(),
                expected: expected.into(),
                actual: format!("{:?}", actual),
                key: key.as_ref().to_string(),
            }
        } else {
            UnexpectedPromptResponse {
                prompt: prompt.into(),
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

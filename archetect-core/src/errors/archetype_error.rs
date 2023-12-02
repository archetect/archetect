use std::borrow::Cow;
use std::path::PathBuf;

use camino::Utf8PathBuf;
use rhai::EvalAltResult;

use crate::errors::{RenderError, RequirementsError};

#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("The specified archetype is missing an archetype.yml or archetype.yaml file")]
    ArchetypeConfigMissing,
    #[error("The specified archetype config `{path}` does not exist")]
    ArchetypeConfigNotFound { path: PathBuf },
    #[error("The specified archetype manifest `{path}` does not exist")]
    ArchetypeManifestNotFound { path: Utf8PathBuf },
    #[error("Archetype by key `{key}` does not exist")]
    ArchetypeKeyNotFound { key: String },
    #[error(transparent)]
    ArchetypeScriptError(#[from] EvalAltResult),
    #[error("Invalid Answers Config")]
    InvalidAnswersConfig,
    #[error(transparent)]
    RenderError(#[from] RenderError),
    #[error("IO Error in Archetype: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Archetype Configuration Error in `{path}`: {source}")]
    YamlError { path: PathBuf, source: serde_yaml::Error },
    #[error("Archetype Manifest Syntax error in `{path}`: {source}")]
    ArchetypeManifestSyntaxError {
        path: Utf8PathBuf,
        source: serde_yaml::Error,
    },
    #[error("Operation was interrupted")]
    OperationInterrupted,
    #[error("Value is required")]
    ValueRequired,
    #[error("Archetype requirements failure:\n\n{0}")]
    RequirementsError(#[from] RequirementsError),
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
    #[error("'{prompt}' (key: '{key}') was not answered and does not have a default value")]
    KeyedHeadlessNoAnswer {
        prompt: String,
        key: String,
    },
    #[error("'{prompt}' was not answered and does not have a default value")]
    HeadlessNoAnswer {
        prompt: String,
    },
    #[error("'{prompt}' (key: '{key}') is not optional")]
    KeyedAnswerNotOptional {
        prompt: String,
        key: String,
    },
    #[error("'{prompt}' is not optional")]
    AnswerNotOptional {
        prompt: String,
    },
    #[error("When specifying the '{setting}' setting, it must be {requires}, but it was {actual}")]
    InvalidSetting {
        setting: String,
        requires: String,
        actual: String,
    },
    #[error("Archetype Script Aborted")]
    ScriptAbortError,
}

impl ArchetypeError {
    pub fn answer_validation_error<'a, D, P, K, R>(answer: D, prompt: P, key: Option<K>, requirement: R) -> ArchetypeError
    where
        D: Into<String>,
        P: Into<String>,
        K: Into<Cow<'a, str>>,
        R: Into<String>,
    {
        if let Some(key) = key {
            ArchetypeError::KeyedAnswerValidationError {
                answer: answer.into(),
                prompt: prompt.into(),
                key: key.into().to_string(),
                requires: requirement.into(),
            }
        } else {
            ArchetypeError::AnswerValidationError {
                answer: answer.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_type_error<'a, D, P, K, R>(answer: D, prompt: P, key: Option<K>, requirement: R) -> ArchetypeError
    where
        D: Into<String>,
        P: Into<String>,
        K: Into<Cow<'a, str>>,
        R: Into<String>,
    {
        if let Some(key) = key {
            ArchetypeError::KeyedAnswerTypeError {
                answer: answer.into(),
                prompt: prompt.into(),
                key: key.into().to_string(),
                requires: requirement.into(),
            }
        } else {
            ArchetypeError::AnswerTypeError {
                answer: answer.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn default_validation_error<'a, D, P, K, R>(default: D, prompt: P, key: Option<K>, requirement: R) -> ArchetypeError
        where
            D: Into<String>,
            P: Into<String>,
            K: Into<Cow<'a, str>>,
            R: Into<String>,
    {
        if let Some(key) = key {
            ArchetypeError::KeyedDefaultValidationError {
                default: default.into(),
                prompt: prompt.into(),
                key: key.into().to_string(),
                requires: requirement.into(),
            }
        } else {
            ArchetypeError::DefaultValidationError {
                default: default.into(),
                prompt: prompt.into(),
                requires: requirement.into(),
            }
        }
    }

    pub fn answer_not_optional<'a, P, K>(prompt: P, key: Option<K>) -> ArchetypeError
        where P: Into<String>,
              K: Into<Cow<'a, str>>
    {
        if let Some(key) = key {
            ArchetypeError::KeyedAnswerNotOptional {
                prompt: prompt.into(),
                key: key.into().to_string(),
            }
        } else {
            ArchetypeError::AnswerNotOptional {
                prompt: prompt.into(),
            }
        }
    }

    pub fn headless_no_answer<'a, P, K>(prompt: P, key: Option<K>) -> ArchetypeError
    where P: Into<String>,
          K: Into<Cow<'a, str>>
    {
        if let Some(key) = key {
            ArchetypeError::KeyedHeadlessNoAnswer {
                prompt: prompt.into(),
                key: key.into().to_string(),
            }
        } else {
            ArchetypeError::HeadlessNoAnswer {
                prompt: prompt.into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use glob::Pattern;

    #[test]
    fn test_glob_full_directory_path() {
        assert!(Pattern::new("*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/h*/*/*ects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }

    #[test]
    fn test_glob_full_file_path() {
        assert!(Pattern::new("*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/h*/*/*ects*jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/**/*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }
}

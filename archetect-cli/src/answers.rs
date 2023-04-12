use archetect_core::config::AnswerConfigError;
use archetect_core::ArchetectError;
use camino::Utf8Path;
use rhai::{Dynamic, Engine, Map};
use std::fs;

pub fn read_answers<P: AsRef<Utf8Path>>(path: P) -> Result<Map, ArchetectError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerConfigError::MissingError,
        });
    }
    match path.extension() {
        Some("yml") => read_yaml_answers(&path),
        Some("yaml") => read_yaml_answers(&path),
        Some("json") => read_json_answers(&path),
        Some("rhai") => read_rhai_answers(&path),
        Some(_extension) => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerConfigError::InvalidFileType,
        }),
        None => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerConfigError::InvalidFileType,
        }),
    }
}

fn read_yaml_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = serde_yaml::from_str(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError { path: path.to_string(), source: AnswerConfigError::InvalidYamlAnswerFileStructure })
    }
}

fn read_json_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = serde_json::from_str(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError { path: path.to_string(), source: AnswerConfigError::InvalidJsonAnswerFileStructure })
    }
}

fn read_rhai_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = Engine::new().eval::<Dynamic>(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError { path: path.to_string(), source: AnswerConfigError::InvalidRhaiAnswerFileStructure })
    }
}

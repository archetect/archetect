mod builder;
mod model;
#[cfg(test)]
mod tests;
mod types;

pub use builder::*;
pub use model::*;
pub use types::*;

use std::path::Path;

/// Parse an AML model from a YAML string.
pub fn parse_yaml(yaml: &str) -> Result<ResolvedModel, serde_yaml::Error> {
    let model: AmlModel = serde_yaml::from_str(yaml)?;
    Ok(ResolvedModel::from_model(model))
}

/// Load and parse an AML model from a file path.
pub fn load_file(path: &Path) -> Result<ResolvedModel, AmlError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| AmlError::Io(path.to_string_lossy().to_string(), e))?;
    parse_yaml(&contents).map_err(AmlError::Parse)
}

#[derive(Debug, thiserror::Error)]
pub enum AmlError {
    #[error("Failed to read AML file '{0}': {1}")]
    Io(String, std::io::Error),
    #[error("Failed to parse AML YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
}

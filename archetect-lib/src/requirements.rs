use semver::{VersionReq, Version};
use std::path::{PathBuf};
use std::fs;
use crate::Archetect;

#[derive(Debug, Deserialize, Serialize)]
pub struct Requirements {
    #[serde(rename = "archetect")]
    archetect_requirement: VersionReq,
}

impl Requirements {
    pub fn new(archetect_version: VersionReq) -> Requirements {
        Requirements{ archetect_requirement: archetect_version }
    }

    pub fn archetect_version(&self) -> &VersionReq {
        &self.archetect_requirement
    }

    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Option<Requirements>, RequirementsError> {
        let mut path = path.into();
        if path.is_dir() {
            let candidates = vec!["requirements.yml", "requirements.yaml"];
            for candidate in candidates {
                let config_file = path.join(candidate);
                if config_file.exists() {
                    path = config_file;
                    break;
                }
            }
        }
        if !path.exists() || path.is_dir() {
            Ok(None)
        } else {
            let config = fs::read_to_string(&path)?;
            match serde_yaml::from_str::<Requirements>(&config) {
                Ok(result) => {
                    return Ok(Some(result));
                }
                Err(error) => {
                    return Err(RequirementsError::DeserializationError { path: path, cause: error });
                }
            }
        }
    }

    pub fn verify(&self, archetect: &Archetect) -> Result<(), RequirementsError> {
        if !self.archetect_requirement.matches(&archetect.version()) {
            Err(RequirementsError::ArchetectVersion(archetect.version().clone(), self.archetect_requirement.clone()))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum RequirementsError {
    DeserializationError { path: PathBuf, cause: serde_yaml::Error },
    ArchetectVersion(Version, VersionReq),
    IoError(std::io::Error)
}

impl From<std::io::Error> for RequirementsError {
    fn from(error: std::io::Error) -> Self {
        RequirementsError::IoError(error)
    }
}

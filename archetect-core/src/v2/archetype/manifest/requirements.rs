use crate::errors::{ArchetypeError, RequirementsError};
use semver::VersionReq;

use crate::v2::runtime::context::RuntimeContext;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeRequirements {
    #[serde(rename = "archetect")]
    archetect_version: VersionReq,
}

impl RuntimeRequirements {
    pub fn archetect_version(&self) -> &VersionReq {
        &self.archetect_version
    }

    pub fn check_requirements(&self, runtime_context: &RuntimeContext) -> Result<(), ArchetypeError> {
        let version = runtime_context.archetect_version();

        if !self.archetect_version.matches(version) {
            return Err(RequirementsError::ArchetectVersion(
                runtime_context.archetect_version().clone(),
                self.archetect_version.clone(),
            )
            .into());
        }

        Ok(())
    }
}

impl Default for RuntimeRequirements {
    fn default() -> Self {
        RuntimeRequirements {
            archetect_version: VersionReq::any(),
        }
    }
}

#[cfg(test)]
mod tests {
    use semver::{Version, VersionReq};

    #[test]
    fn test_version_equals() {
        let version = Version::parse("1.0.0").unwrap();
        let requirement = VersionReq::parse("1.0.0").unwrap();
        assert!(requirement.matches(&version));
    }
}

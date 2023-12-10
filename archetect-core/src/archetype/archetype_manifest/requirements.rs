use semver::VersionReq;

use crate::errors::RequirementsError;
use crate::Archetect;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeRequirements {
    #[serde(rename = "archetect")]
    archetect_version: VersionReq,
}

impl RuntimeRequirements {
    pub fn archetect_version(&self) -> &VersionReq {
        &self.archetect_version
    }

    pub fn check_requirements(&self, archetect: &Archetect) -> Result<(), RequirementsError> {
        let version = archetect.version();

        if !self.archetect_version.matches(version) {
            return Err(RequirementsError::ArchetectVersion(
                archetect.version().clone(),
                self.archetect_version.clone(),
            ));
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

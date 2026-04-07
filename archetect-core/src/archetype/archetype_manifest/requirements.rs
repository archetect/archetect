use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::Archetect;
use crate::errors::RequirementsError;

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

        // Treat the requirement as a minimum version: the running archetect must be
        // at least the version the archetype requires. This allows archetect 3.x to
        // render archetypes that require 2.x, since 3.x is a superset of 2.x.
        let min_version = extract_minimum_version(&self.archetect_version);
        if version < &min_version {
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
        let archetect_version = VersionReq::parse(env!("CARGO_PKG_VERSION")).unwrap();
        RuntimeRequirements {
            archetect_version,
        }
    }
}

/// Extract the minimum version from a VersionReq by parsing the comparators.
/// Falls back to treating the requirement string as a version directly.
fn extract_minimum_version(req: &VersionReq) -> Version {
    if let Some(comp) = req.comparators.first() {
        Version::new(comp.major, comp.minor.unwrap_or(0), comp.patch.unwrap_or(0))
    } else {
        Version::new(0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use semver::{Version, VersionReq};
    use super::extract_minimum_version;

    #[test]
    fn test_version_equals() {
        let version = Version::parse("1.0.0").unwrap();
        let requirement = VersionReq::parse("1.0.0").unwrap();
        assert!(requirement.matches(&version));
    }

    #[test]
    fn test_newer_major_satisfies_older_requirement() {
        // archetect 3.0.0 should satisfy requires: "2.0.0"
        let version = Version::parse("3.0.0").unwrap();
        let req = VersionReq::parse("2.0.0").unwrap();
        let min = extract_minimum_version(&req);
        assert!(version >= min);
    }

    #[test]
    fn test_older_version_fails_newer_requirement() {
        // archetect 2.1.0 should NOT satisfy requires: "3.0.0"
        let version = Version::parse("2.1.0").unwrap();
        let req = VersionReq::parse("3.0.0").unwrap();
        let min = extract_minimum_version(&req);
        assert!(version < min);
    }

    #[test]
    fn test_same_major_newer_minor_satisfies() {
        // archetect 2.1.0 should satisfy requires: "2.0.0"
        let version = Version::parse("2.1.0").unwrap();
        let req = VersionReq::parse("2.0.0").unwrap();
        let min = extract_minimum_version(&req);
        assert!(version >= min);
    }
}

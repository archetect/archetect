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
        check_version(archetect.version(), &self.archetect_version)
    }
}

/// Enforce the `requires: archetect:` contract.
///
/// Major versions are strictly separated: an archetype requiring 2.x renders
/// only with archetect 2, and one requiring 3.x only with archetect 3 — the
/// scripting and templating engines are not compatible across majors. Within
/// the same major, the requirement is a minimum: the running archetect must
/// be at least the version the archetype requires.
fn check_version(version: &Version, requirement: &VersionReq) -> Result<(), RequirementsError> {
    let min_version = extract_minimum_version(requirement);

    if version.major < min_version.major {
        // The archetype targets a newer major than this binary.
        return Err(RequirementsError::ArchetectVersion(
            version.clone(),
            requirement.clone(),
        ));
    }

    if version.major > min_version.major {
        // The archetype targets an older major — point at the legacy binary.
        return Err(RequirementsError::ArchetectVersionMajor(
            version.clone(),
            requirement.clone(),
            min_version.major,
        ));
    }

    if version < &min_version {
        return Err(RequirementsError::ArchetectVersion(
            version.clone(),
            requirement.clone(),
        ));
    }

    Ok(())
}

impl Default for RuntimeRequirements {
    fn default() -> Self {
        let archetect_version = VersionReq::parse(env!("CARGO_PKG_VERSION"))
            .expect("CARGO_PKG_VERSION is always a valid semver requirement");
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
    use crate::errors::RequirementsError;
    use super::check_version;

    fn check(version: &str, requirement: &str) -> Result<(), RequirementsError> {
        let version = Version::parse(version).unwrap();
        let requirement = VersionReq::parse(requirement).unwrap();
        check_version(&version, &requirement)
    }

    #[test]
    fn test_version_equals() {
        assert!(check("3.0.0", "3.0.0").is_ok());
    }

    #[test]
    fn test_same_major_newer_minor_satisfies() {
        // archetect 3.1.0 satisfies requires: "3.0.0"
        assert!(check("3.1.0", "3.0.0").is_ok());
    }

    #[test]
    fn test_same_major_older_version_fails() {
        // archetect 3.0.0 does NOT satisfy requires: "3.1.0"
        assert!(matches!(
            check("3.0.0", "3.1.0"),
            Err(RequirementsError::ArchetectVersion(_, _))
        ));
    }

    #[test]
    fn test_newer_major_rejects_older_requirement() {
        // Majors are strictly separated: archetect 3.x must NOT render an
        // archetype that requires 2.x — point the user at archetect2.
        assert!(matches!(
            check("3.0.0", "2.0.0"),
            Err(RequirementsError::ArchetectVersionMajor(_, _, 2))
        ));
    }

    #[test]
    fn test_older_major_rejects_newer_requirement() {
        // archetect 2.1.0 does NOT satisfy requires: "3.0.0"
        assert!(matches!(
            check("2.1.0", "3.0.0"),
            Err(RequirementsError::ArchetectVersion(_, _))
        ));
    }
}

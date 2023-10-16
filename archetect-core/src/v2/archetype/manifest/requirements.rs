use semver::VersionReq;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchetypeRequirements {
    pub(crate) archetect: VersionReq,
}

impl ArchetypeRequirements {
    pub fn archetect_version_req(&self) -> &VersionReq {
        &self.archetect
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

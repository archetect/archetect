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

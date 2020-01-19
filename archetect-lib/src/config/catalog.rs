#[derive(Debug)]
pub enum CatalogConfigError {
    IOError(std::io::Error),
}

impl From<std::io::Error> for CatalogConfigError {
    fn from(cause: std::io::Error) -> Self {
        CatalogConfigError::IOError(cause)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogInfo {
    description: String,
    source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogConfig {
    entries: Vec<CatalogConfigEntry>,
}

impl CatalogConfig {
    pub fn new() -> CatalogConfig {
        CatalogConfig { entries: vec![] }
    }

    pub fn entries(&self) -> &[CatalogConfigEntry] {
        self.entries.as_slice()
    }

    pub fn with_entry(mut self, entry: CatalogConfigEntry) -> CatalogConfig {
        self.add_entry(entry);
        self
    }

    pub fn add_entry(&mut self, entry: CatalogConfigEntry) {
        self.entries.push(entry);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogConfigEntry {
    #[serde(rename = "type")]
    pub entry_type: CatalogConfigEntryType,
    pub description: String,
    pub source: String,
}

impl CatalogConfigEntry {
    pub fn new<D: Into<String>, S: Into<String>>(
        description: D,
        source: S,
        entry_type: CatalogConfigEntryType,
    ) -> CatalogConfigEntry {
        CatalogConfigEntry {
            description: description.into(),
            source: source.into(),
            entry_type,
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn entry_type(&self) -> &CatalogConfigEntryType {
        &self.entry_type
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CatalogConfigEntryType {
    Catalog,
    Archetype,
}

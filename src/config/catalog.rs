#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogConfig {
    entries: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogEntry {
    description: String,
    location: String,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        CatalogConfig { entries: vec![] }
    }
}

impl CatalogConfig {
    pub fn add_entry(&mut self, entry: CatalogEntry) {
        self.entries.push(entry);
    }
}

impl CatalogEntry {
    pub fn new<D: Into<String>, L: Into<String>>(description: D, location: L) -> CatalogEntry {
        CatalogEntry {
            description: description.into(),
            location: location.into(),
        }
    }
}

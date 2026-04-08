use git2;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use archetect_api::{ContextMap, ContextValue};

use crate::configuration::configuration_local_section::ConfigurationLocalsSection;
use crate::configuration::configuration_security_sections::{
    ConfigurationSecuritySection, ShellExecPolicy,
};
use crate::configuration::configuration_update_section::ConfigurationUpdateSection;
use crate::manifest::CatalogEntry;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configuration {
    /// Unified catalog — named, addressable archetype references.
    /// Project-level `.archetect.yaml` files can override this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    catalog: Option<LinkedHashMap<String, CatalogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headless: Option<bool>,
    answers: ContextMap,
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    security: ConfigurationSecuritySection,
    #[serde(skip_serializing_if = "Option::is_none")]
    switches: Option<Vec<String>>,
}

impl Configuration {
    pub fn headless(&self) -> bool {
        self.headless.unwrap_or_default()
    }

    pub fn with_headless(mut self, value: bool) -> Self {
        self.headless = Some(value);
        self
    }

    pub fn offline(&self) -> bool {
        self.offline.unwrap_or_default()
    }

    pub fn with_offline(mut self, value: bool) -> Self {
        self.offline = Some(value);
        self
    }
    pub fn updates(&self) -> &ConfigurationUpdateSection {
        &self.updates
    }

    pub fn locals(&self) -> &ConfigurationLocalsSection {
        &self.locals
    }

    pub fn security(&self) -> &ConfigurationSecuritySection {
        &self.security
    }

    /// Resolved shell execution policy.
    pub fn shell_exec_policy(&self) -> ShellExecPolicy {
        self.security.shell_exec_policy()
    }

    /// Override the shell execution policy. Used by MCP mode to force `Forbidden`
    /// and by `--allow-exec` to force `Allowed`.
    pub fn with_shell_exec_policy(mut self, policy: ShellExecPolicy) -> Self {
        self.security.set_shell_exec_policy(policy);
        self
    }

    /// Returns the unified catalog if set.
    pub fn catalog(&self) -> Option<&LinkedHashMap<String, CatalogEntry>> {
        self.catalog.as_ref()
    }

    /// Replace the catalog. Used by project config detection to swap in
    /// project-level catalogs.
    pub fn with_catalog(mut self, catalog: LinkedHashMap<String, CatalogEntry>) -> Self {
        self.catalog = Some(catalog);
        self
    }

    pub fn set_catalog(&mut self, catalog: LinkedHashMap<String, CatalogEntry>) {
        self.catalog = Some(catalog);
    }

    pub fn answers(&self) -> &ContextMap {
        &self.answers
    }

    pub fn with_answer<K: Into<String>, V: Into<ContextValue>>(mut self, key: K, value: V) -> Self {
        self.answers.insert(key.into(), value.into());
        self
    }

    pub fn switches(&self) -> &[String] {
        if let Some(switches) = &self.switches {
            switches
        } else {
            Default::default()
        }
    }

    pub fn with_switch<S: Into<String>>(mut self, switch: S) -> Self {
        self.switches
            .get_or_insert_with(|| Default::default())
            .push(switch.into());
        self
    }

    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(&self).expect("Unexpected error converting Configuration to yaml")
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            catalog: Some(default_catalog()),
            headless: Default::default(),
            offline: Default::default(),
            updates: Default::default(),
            security: Default::default(),
            answers: default_answers(),
            locals: Default::default(),
            switches: Default::default(),
        }
    }
}

fn default_catalog() -> LinkedHashMap<String, CatalogEntry> {
    let mut catalog = LinkedHashMap::new();
    // TODO(v3-catalog-repo): point this at the v3 master catalog once the
    // archetect/* git org is reorganized for v3. The current URL points at
    // the v2 catalog; v3 needs its own repo with unified Manifest format.
    catalog.insert(
        "archetect".to_string(),
        CatalogEntry {
            description: Some("Archetect Catalog".to_string()),
            source: Some("https://github.com/archetect/archetect.catalog.git".to_string()),
            catalog: None,
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
        },
    );
    catalog
}

fn default_answers() -> ContextMap {
    let mut results = ContextMap::new();

    if let Ok(config) = git2::Config::open_default() {
        let name = config.get_string("user.name");
        let email = config.get_string("user.email");

        if let Ok(ref name) = name {
            results.insert("author_name".to_string(), ContextValue::String(name.clone()));
        }

        if let Ok(ref email) = email {
            results.insert("author_email".to_string(), ContextValue::String(email.clone()));
        }

        if let (Ok(ref name), Ok(ref email)) = (&name, &email) {
            results.insert(
                "author_full".to_string(),
                ContextValue::String(format!("{} <{}>", name, email)),
            );
        }
    }

    results
}

impl Configuration {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() -> anyhow::Result<()> {
        let configuration = Configuration::default();
        println!("{}", serde_yaml::to_string(&configuration)?);
        Ok(())
    }
}

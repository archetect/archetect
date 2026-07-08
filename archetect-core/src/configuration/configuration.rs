use git2;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use archetect_api::{ContextMap, ContextValue};

use crate::configuration::configuration_client_section::ConfigurationClientSection;
use crate::configuration::configuration_local_section::ConfigurationLocalsSection;
use crate::configuration::configuration_security_sections::{
    ConfigurationSecuritySection, ShellExecPolicy,
};
use crate::configuration::configuration_server_section::ConfigurationServerSection;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    answers: ContextMap,
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    security: ConfigurationSecuritySection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    server: Option<ConfigurationServerSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client: Option<ConfigurationClientSection>,
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

    pub fn dry_run(&self) -> bool {
        self.dry_run.unwrap_or_default()
    }

    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.dry_run = Some(value);
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

    pub fn set_switches(&mut self, switches: Vec<String>) {
        self.switches = Some(switches);
    }

    pub fn server(&self) -> Option<&ConfigurationServerSection> {
        self.server.as_ref()
    }

    pub fn client(&self) -> Option<&ConfigurationClientSection> {
        self.client.as_ref()
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
            dry_run: Default::default(),
            updates: Default::default(),
            security: Default::default(),
            answers: default_answers(),
            locals: Default::default(),
            server: Default::default(),
            client: Default::default(),
            switches: Default::default(),
        }
    }
}

fn default_catalog() -> LinkedHashMap<String, CatalogEntry> {
    let mut catalog = LinkedHashMap::new();
    // The v3 master catalog lives at archetect/archetect-catalog (new repo,
    // not the v2 `archetect.catalog` repo which has a dot-suffixed name and
    // a different manifest format). Once version-aware source resolution
    // (docs/specs/version-aware-source-resolution.md) lands, this bare URL
    // will auto-resolve to the highest matching v3.* tag in that repo.
    catalog.insert(
        "archetect".to_string(),
        CatalogEntry {
            description: Some("Archetect Catalog".to_string()),
            source: Some("https://github.com/archetect/archetect-catalog.git".to_string()),
            catalog: None,
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            server: None,
            library: false,
            show: true,
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
    use crate::configuration::ConfigurationClientTlsSection;
    use indoc::indoc;

    #[test]
    fn test_defaults() -> anyhow::Result<()> {
        let configuration = Configuration::default();
        println!("{}", serde_yaml::to_string(&configuration)?);
        Ok(())
    }

    #[test]
    fn test_server_section_parses() {
        // Exercise the section type directly — the full `Configuration`
        // struct has several non-optional sibling fields (answers, locals,
        // updates, security) that come from figment layering in the real
        // loader, and we don't need them to validate the new schema.
        let yaml = indoc! {r#"
            host: 127.0.0.1
            port: 9000
            tls:
              cert: /etc/archetect/server.crt
              key: /etc/archetect/server.key
              client_ca: /etc/archetect/clients-ca.crt
        "#};
        let section: ConfigurationServerSection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(section.host(), Some("127.0.0.1"));
        assert_eq!(section.port(), Some(9000));
        let tls = section.tls().expect("tls subsection");
        assert_eq!(tls.cert().to_str(), Some("/etc/archetect/server.crt"));
        assert_eq!(tls.key().to_str(), Some("/etc/archetect/server.key"));
        assert_eq!(
            tls.client_ca().and_then(|p| p.to_str()),
            Some("/etc/archetect/clients-ca.crt")
        );
    }

    #[test]
    fn test_client_section_parses() {
        let yaml = indoc! {r#"
            endpoint: https://archetect.example.com:8443
            connect:
              timeout_secs: 10
              retries: 3
              backoff_base_ms: 500
              max_backoff_secs: 30
            keepalive:
              interval_secs: 60
              timeout_secs: 15
            tls:
              ca: /etc/archetect/ca.crt
              client_cert: /etc/archetect/me.crt
              client_key: /etc/archetect/me.key
              domain: archetect.example.com
        "#};
        let section: ConfigurationClientSection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            section.endpoint(),
            Some("https://archetect.example.com:8443")
        );
        let connect = section.connect().expect("connect subsection");
        assert_eq!(connect.timeout_secs(), Some(10));
        assert_eq!(connect.retries(), Some(3));
        assert_eq!(connect.backoff_base_ms(), Some(500));
        assert_eq!(connect.max_backoff_secs(), Some(30));
        let ka = section.keepalive().expect("keepalive subsection");
        assert_eq!(ka.interval_secs(), Some(60));
        assert_eq!(ka.timeout_secs(), Some(15));
        let tls = section.tls().expect("tls subsection");
        assert_eq!(tls.ca().and_then(|p| p.to_str()), Some("/etc/archetect/ca.crt"));
        assert_eq!(
            tls.client_cert().and_then(|p| p.to_str()),
            Some("/etc/archetect/me.crt")
        );
        assert_eq!(
            tls.client_key().and_then(|p| p.to_str()),
            Some("/etc/archetect/me.key")
        );
        assert_eq!(tls.domain(), Some("archetect.example.com"));
    }

    #[test]
    fn test_empty_sections_parse() {
        // A `tls:` key with no fields under it still gives us a section —
        // this is what "enable TLS with defaults" looks like.
        let section: ConfigurationClientTlsSection =
            serde_yaml::from_str("{}").expect("empty map parses");
        assert!(section.ca().is_none());
        assert!(section.client_cert().is_none());
    }
}

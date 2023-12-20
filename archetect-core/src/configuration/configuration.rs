use git2;
use linked_hash_map::LinkedHashMap;
use rhai::{Dynamic, Identifier, Map};

use crate::catalog::CatalogEntry;
use crate::configuration::{RenderCatalogInfo, RenderGroupInfo};
use crate::configuration::ArchetectCommand::RenderGroup;
use crate::configuration::configuration_actions_section::ArchetectCommand;
use crate::configuration::configuration_local_section::ConfigurationLocalsSection;
use crate::configuration::configuration_security_sections::ConfigurationSecuritySection;
use crate::configuration::configuration_update_section::ConfigurationUpdateSection;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    actions: LinkedHashMap<String, ArchetectCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headless: Option<bool>,
    answers: Map,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    catalogs: LinkedHashMap<String, Vec<CatalogEntry>>,
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

    pub fn commands(&self) -> &LinkedHashMap<String, ArchetectCommand> {
        &self.actions
    }

    pub fn get_command<C: AsRef<str>>(&self, command_name: C) -> Option<&ArchetectCommand> {
        self.actions.get(command_name.as_ref())
    }

    pub fn answers(&self) -> &Map {
        &self.answers
    }

    pub fn with_answer<K: Into<Identifier>, V: Into<Dynamic>>(mut self, key: K, value: V) -> Self {
        self.answers.insert(key.into(), value.into());
        self
    }

    pub fn catalogs(&self) -> &LinkedHashMap<String, Vec<CatalogEntry>> {
        &self.catalogs
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
            actions: default_commands(),
            headless: Default::default(),
            offline: Default::default(),
            updates: Default::default(),
            security: Default::default(),
            answers: default_answers(),
            catalogs: default_catalog(),
            locals: Default::default(),
            switches: Default::default(),
        }
    }
}

fn default_commands() -> LinkedHashMap<String, ArchetectCommand> {
    let mut commands = LinkedHashMap::new();
    let mut entries = vec![];

    let archetect_catalog = CatalogEntry::Catalog {
        description: "Archetect".to_string(),
        info: RenderCatalogInfo::new("https://github.com/archetect/archetect.catalog.git"),
    };

    entries.push(archetect_catalog);

    let command = RenderGroupInfo::new(entries);

    commands.insert("default".to_string(), RenderGroup(command));
    commands
}

fn default_catalog() -> LinkedHashMap<String, Vec<CatalogEntry>> {
    let mut catalogs = LinkedHashMap::new();
    catalogs.insert(
        "default".to_owned(),
        vec![CatalogEntry::Catalog {
            description: "Archetect".to_owned(),
            info: RenderCatalogInfo::new("https://github.com/archetect/archetect.catalog.git"),
        }],
    );
    catalogs
}

fn default_answers() -> Map {
    let mut results = Map::new();

    if let Ok(config) = git2::Config::open_default() {
        let name = config.get_string("user.name");
        let email = config.get_string("user.email");

        if name.is_ok() {
            results.insert("author_name".to_owned().into(), name.as_ref().unwrap().into());
        }

        if email.is_ok() {
            results.insert("author_email".to_owned().into(), email.as_ref().unwrap().into());
        }

        if name.is_ok() && email.is_ok() {
            results.insert(
                "author_full".to_owned().into(),
                format!("{} <{}>", name.as_ref().unwrap(), email.as_ref().unwrap()).into(),
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

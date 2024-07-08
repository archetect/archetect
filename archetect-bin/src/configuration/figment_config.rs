use std::collections::BTreeMap;

use clap::ArgMatches;
use figment::{Figment, Profile, Provider};
use figment::error::Kind;
use figment::providers::{Format, Serialized, Yaml};
use figment::Result;
use figment::value::Value;

use archetect_core::configuration::Configuration;
use archetect_core::system::SystemLayout;

use crate::configuration::figment_clap_provider::{ArgExtractor, ClapMatches};

pub const CONFIGURATION_FILE: &str = "archetect.yaml";
pub const DOT_CONFIGURATION_FILE: &str = ".archetect.yaml";

pub fn load_user_config<L: SystemLayout>(layout: &L, args: &ArgMatches) -> Result<Configuration> {
    let defaults = Figment::new().merge(Serialized::defaults(Configuration::default()));
    let user = Figment::new().merge(Yaml::file(layout.etc_dir().join(CONFIGURATION_FILE)));
    let current_dir_hidden = Figment::new().merge(Yaml::file(CONFIGURATION_FILE));
    let current_dir = Figment::new().merge(Yaml::file(DOT_CONFIGURATION_FILE));

    let merged = smart_merge(vec![defaults, user, current_dir_hidden, current_dir])?.merge(
        ClapMatches::new(args.clone())
            .extract(ArgExtractor::flag("updates.force", "force-update"))
            .extract(ArgExtractor::flag("offline", "offline"))
            .extract(ArgExtractor::flag("headless", "headless"))
            .extract(ArgExtractor::flag("locals.enabled", "local"))
            .extract(ArgExtractor::flag("security.allow_exec", "allow-exec"))
            .extract(ArgExtractor::u16("server.port", "port"))
            .extract(ArgExtractor::string("server.host", "host")),
    );

    merged.extract()
}

fn smart_merge(figments: Vec<Figment>) -> Result<Figment> {
    let mut results = Figment::new();
    let mut actions = BTreeMap::new();
    for f in figments {
        if let Ok(data) = f.data() {
            if let Some(defaults) = data.get(&Profile::Default) {
                for (key, value) in defaults {
                    match (key.as_str(), value) {
                        ("actions", actions_value) => match actions_value {
                            Value::Dict(_tag, actions_dict) => {
                                for (action_key, action_info) in actions_dict {
                                    actions.insert(action_key.to_owned(), action_info.to_owned());
                                }
                            }
                            other => {
                                return Err(figment::Error::from(Kind::InvalidValue(
                                    other.to_actual(),
                                    "'actions' must be a map".to_string(),
                                )))
                            }
                        },
                        (key, value) => {
                            results = results.merge(Serialized::default(key, value));
                        }
                    }
                }
            }
        }
    }

    results = results.merge(Serialized::default("actions", actions));

    Ok(results)
}

#[cfg(test)]
mod tests {
    use archetect_core::system::RootedSystemLayout;

    use crate::cli;
    use crate::configuration::figment_config::load_user_config;

    #[test]
    fn test_actions_merge() -> anyhow::Result<()> {
        let layout = RootedSystemLayout::temp()?;
        let arg_vec = vec!["archetect"];
        let matches = cli::command().get_matches_from(arg_vec);
        let _configuration = load_user_config(&layout, &matches)?;
        Ok(())
    }
}

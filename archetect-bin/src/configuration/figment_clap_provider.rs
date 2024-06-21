use clap::ArgMatches;
use clap::parser::ValueSource;
use figment::{Error, Figment, Metadata, Profile, Provider};
use figment::providers::Serialized;
use figment::value::{Dict, Map, Value};

pub struct ClapMatches {
    matches: ArgMatches,
    extractors: Vec<ArgExtractor>,
}

impl ClapMatches {
    pub fn new(matches: ArgMatches) -> ClapMatches {
        ClapMatches {
            matches,
            extractors: Default::default(),
        }
    }

    pub fn extract(mut self, extractor: ArgExtractor) -> Self {
        self.extractors.push(extractor);
        self
    }
}

impl Provider for ClapMatches {
    fn metadata(&self) -> Metadata {
        Metadata::named("Command Line Args").source("Clap")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let mut figment = Figment::new();
        for extractor in &self.extractors {
            if let Some((path, value)) = extractor.extract(&self.matches) {
                figment = figment.merge(Serialized::global(path.as_str(), value));
            }
        }
        figment.data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Global)
    }
}

pub struct ArgExtractor {
    path: String,
    key: String,
    arg_type: ArgType,
}

pub enum ArgType {
    String,
    U16,
    Bool,
    Flag,
}

impl ArgExtractor {
    pub fn new<P: Into<String>, K: Into<String>>(path: P, key: K, arg_type: ArgType) -> ArgExtractor {
        Self {
            path: path.into(),
            key: key.into(),
            arg_type,
        }
    }

    pub fn string<P: Into<String>, K: Into<String>>(path: P, key: K) -> ArgExtractor {
        Self::new(path, key, ArgType::String)
    }

    pub fn u16<P: Into<String>, K: Into<String>>(path: P, key: K) -> ArgExtractor {
        Self::new(path, key, ArgType::U16)
    }

    pub fn bool<P: Into<String>, K: Into<String>>(path: P, key: K) -> ArgExtractor {
        Self::new(path, key, ArgType::Bool)
    }

    pub fn flag<P: Into<String>, K: Into<String>>(path: P, key: K) -> ArgExtractor {
        Self::new(path, key, ArgType::Flag)
    }

    fn extract(&self, matches: &ArgMatches) -> Option<(String, Value)> {
        let key = self.key.as_str();
        match self.arg_type {
            ArgType::String => match matches.value_source(key) {
                None => None,
                Some(source) => match source {
                    ValueSource::CommandLine | ValueSource::EnvVariable => matches
                        .get_one::<String>(key)
                        .map(|v| (self.path.to_owned(), Value::from(v.as_str()))),
                    _ => None,
                },
            },
            ArgType::U16 => match matches.value_source(key) {
                None => None,
                Some(source) => match source {
                    ValueSource::CommandLine | ValueSource::EnvVariable => matches
                        .get_one::<u16>(key)
                        .map(|v| (self.path.to_owned(), Value::from(*v))),
                    _ => None,
                },
            },
            ArgType::Bool => {
                match matches.value_source(key) {
                    None => None,
                    Some(source) => match source {
                        // Only override if explicitly set; don't consider a default as an override
                        ValueSource::CommandLine | ValueSource::EnvVariable => {
                            Some((self.path.to_owned(), Value::from(matches.get_flag(key))))
                        }
                        _ => None,
                    },
                }
            }
            ArgType::Flag => match matches.value_source(key) {
                None => None,
                Some(source) => match source {
                    // Only override if explicitly set; don't consider a default as an override
                    ValueSource::CommandLine | ValueSource::EnvVariable => matches
                        .get_one::<bool>(key)
                        .map(|v| (self.path.to_owned(), Value::from(*v))),
                    _ => None,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use figment::Figment;

    use crate::cli;
    use crate::configuration::figment_clap_provider::{ArgExtractor, ClapMatches};

    #[test]
    fn test_clap_extractor() -> anyhow::Result<()> {
        let args = vec!["archetect", "-U"];
        let matches = cli::command().get_matches_from(args);

        let figment = Figment::new().merge(
            ClapMatches::new(matches)
                .extract(ArgExtractor::u16("server.port", "port"))
                .extract(ArgExtractor::flag("updates.force", "force-update")),
        );

        println!("{figment:#?}");
        Ok(())
    }
}

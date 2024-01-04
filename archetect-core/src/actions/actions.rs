use serde::{Deserialize, Serialize};
use crate::actions::action_info::{RenderArchetypeInfo, RenderCatalogInfo, RenderGroupInfo};
use crate::{Archetect, CacheCommand};
use crate::errors::ArchetectError;
use crate::source::SourceCommand;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ArchetectAction {
    #[serde(rename = "group")]
    RenderGroup{
        description: String,
        #[serde(flatten)]
        info: RenderGroupInfo,
    },
    #[serde(rename = "catalog")]
    RenderCatalog {
        description: String,
        #[serde(flatten)]
        info: RenderCatalogInfo,
    },
    #[serde(rename = "archetype")]
    RenderArchetype{
        description: String,
        #[serde(flatten)]
        info: RenderArchetypeInfo,
    },
}

impl ArchetectAction {
    pub fn description(&self) -> &str {
        match self {
            ArchetectAction::RenderGroup { description, info: _ } => description.as_str(),
            ArchetectAction::RenderCatalog { description, info: _ } => description.as_str(),
            ArchetectAction::RenderArchetype { description, info: _} => description.as_str(),
        }
    }

    pub fn execute_cache_command(&self, archetect: &Archetect, command: CacheCommand) -> Result<(), ArchetectError> {
        match self {
            ArchetectAction::RenderGroup { description: _, info } => {
                for entry in info.actions() {
                    entry.execute_cache_command(archetect, command)?;
                }
            }
            ArchetectAction::RenderCatalog { info, .. } => {
                let catalog = archetect.new_catalog(info.source())?;
                match command {
                    CacheCommand::Pull | CacheCommand::PullAll => {
                        if let Some(source) = catalog.source() {
                            source.execute(SourceCommand::Pull)?;
                        }
                        if let CacheCommand::PullAll = command {
                            for entry in catalog.entries() {
                                entry.execute_cache_command(archetect, command)?;
                            }
                        }
                    }
                    CacheCommand::Invalidate => {
                        if let Some(source) = catalog.source() {
                            source.execute(SourceCommand::Invalidate)?;
                        }
                    }
                    CacheCommand::View => unreachable!(),
                }
            }
            ArchetectAction::RenderArchetype { description: _, info } => {
                let source = archetect.new_source(info.source())?;
                match command {
                    CacheCommand::Pull | CacheCommand::PullAll => {
                        source.execute(SourceCommand::Pull)?;
                    }
                    CacheCommand::Invalidate => {
                        source.execute(SourceCommand::Invalidate)?;
                    }
                    CacheCommand::View => unreachable!(),
                }
            }
        }

        Ok(())
    }
}
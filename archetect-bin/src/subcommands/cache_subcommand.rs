use std::fs;

use clap::ArgMatches;
use tracing::error;

use archetect_core::actions::ArchetectAction;
use archetect_core::Archetect;
use archetect_core::CacheManager;
use archetect_core::catalog::{Catalog, CatalogManifest};
use archetect_core::errors::ArchetectError;
use archetect_inquire::Confirm;

pub fn handle_cache_subcommand(args: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    let cache_manager = CacheManager::new(archetect.clone());
    match args.subcommand() {
        None => {
            error!("Subcommand expected");
        }
        Some(("manage", _args)) => {
            let action = args.get_one::<String>("action").expect("Expected Action");
            match archetect.configuration().action(action) {
                None => {
                    return Err(ArchetectError::MissingAction(
                        action.to_owned(),
                        archetect
                            .configuration()
                            .actions()
                            .keys()
                            .map(|v| v.to_string())
                            .collect::<Vec<String>>(),
                    ));
                }
                Some(action) => match action {
                    ArchetectAction::RenderGroup { info, .. } => {
                        let manifest = CatalogManifest::new().with_entries(info.actions().to_vec());
                        let catalog = Catalog::new(archetect.clone(), manifest);
                        cache_manager.manage(&catalog)?;
                    }
                    ArchetectAction::RenderCatalog { info, .. } => {
                        let catalog = archetect.new_catalog(info.source())?;
                        cache_manager.manage(&catalog)?;
                    }
                    ArchetectAction::RenderArchetype { info, .. } => {
                        cache_manager.manage_archetype(info)?;
                    }
                },
            }
        }
        Some(("clear", _args)) => {
            let prompt =
                Confirm::new("Are you sure you want to remove all cached Archetypes and Catalogs?").with_default(false);
            match prompt.prompt() {
                Ok(proceed) => {
                    if proceed {
                        let paths = fs::read_dir(archetect.layout().cache_dir()).unwrap();
                        for path in paths {
                            if let Ok(path) = path {
                                fs::remove_dir_all(path.path())?;
                            }
                        }
                    }
                }
                Err(_error) => {}
            }
        }
        Some((command_name, _args)) => {
            error!("Unimplemented command: cache {}", command_name);
        }
    }

    Ok(())
}

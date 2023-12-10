use std::fs;

use clap::ArgMatches;
use log::error;
use archetect_core::CacheManager;

use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use archetect_inquire::Confirm;

pub fn handle_cache_subcommand(args: &ArgMatches, runtime_context: &RuntimeContext) -> Result<(), ArchetectError> {

    match args.subcommand() {
        None => {
            error!("Subcommand expected");
        }
        Some(("manage", _args)) => {
            let cache_manager = CacheManager::new(runtime_context.clone());
            cache_manager.manage(&runtime_context.configuration().catalog())?;
        }
        Some(("pull", _args)) => {
            runtime_context.configuration().catalog().cache(&runtime_context)?;
        }
        Some(("clear", _args)) => {
            let prompt = Confirm::new("Are you sure you want to remove all cached Archetypes and Catalogs?")
                .with_default(false);
            match prompt.prompt() {
                Ok(proceed) => {
                    if proceed {
                        let paths = fs::read_dir(runtime_context.layout().cache_dir()).unwrap();
                        for path in paths {
                            if let Ok(path) = path {
                                fs::remove_dir_all(path.path())?;
                            }
                        }
                    }
                }
                Err(_error) => {
                }
            }

        }
        Some((command_name, _args)) => {
            error!("Unimplemented command: cache {}", command_name);
        }
    }

    Ok(())
}
use std::collections::HashSet;

use camino::Utf8PathBuf;
use clap::ArgMatches;
use archetect_api::{ContextMap, ScriptMessage, ScriptIoHandle};
use archetect_core::{self};
use archetect_core::Archetect;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::{ArchetectError, ArchetypeError, CatalogError, SourceError};
use archetect_core::flags::overlay_flag_tokens;
use archetect_core::source::SourceContents;
use archetect_core::system::{SystemLayout, XdgSystemLayout};
use archetect_terminal_io::TerminalScriptIoHandle;
use ArchetypeError::{PromptAborted, ScriptAbortError};

use crate::answers::{insert_dotted, parse_answer_pair, parse_answer_value};
use crate::subcommands::handle_commands_subcommand;

mod answers;
mod cli;
mod configuration;
mod subcommands;
pub mod vendor;

/// Resolve the render destination from CLI args. Positional
/// `destination-pos` (second positional on render / global / top-level
/// action) wins over the `--destination` / `--dest` flag. Falls back to
/// `.` when neither is supplied.
fn resolve_destination(args: &ArgMatches) -> String {
    if let Some(pos) = args.get_one::<String>("destination-pos") {
        if !pos.is_empty() {
            return pos.clone();
        }
    }
    args.get_one::<String>("destination")
        .cloned()
        .unwrap_or_else(|| ".".to_string())
}

fn main() {
    let matches = cli::command()
        .get_matches();
    cli::configure(&matches);

    let driver = TerminalScriptIoHandle::default();
    let layout = match XdgSystemLayout::new() {
        Ok(layout) => layout,
        Err(err) => {
            let _ = driver.send(ScriptMessage::LogError(format!("Failed to initialize: {}", err)));
            std::process::exit(1);
        }
    };

    match execute(matches, driver.clone(), layout) {
        Ok(()) => (),
        Err(error) => {
            match error {
                // Handled when the script ends by the IO Driver
                ArchetectError::ArchetypeError(ScriptAbortError) => {}
                ArchetectError::CatalogError(CatalogError::SelectionCancelled) => {}
                // User-initiated cancel (Esc / Ctrl-C at a prompt). Exit
                // quietly — no stack trace, no error banner.
                ArchetectError::ArchetypeError(PromptAborted) => {
                    std::process::exit(130);
                }
                _ => {
                    let _ = driver.send(ScriptMessage::LogError(format!("{}", error)));
                }
            }

            std::process::exit(-1);
        }
    }
}

fn execute<D: ScriptIoHandle, L: SystemLayout>(matches: ArgMatches, driver: D, layout: L) -> Result<(), ArchetectError> {
    // The `global` subcommand bypasses project config detection so users can
    // access the global catalog from inside a project that has its own .archetect.yaml.
    let is_global_subcommand = matches!(matches.subcommand(), Some(("global", _)));
    let configuration = if is_global_subcommand {
        configuration::load_global_config(&layout, &matches)
    } else {
        configuration::load_user_config(&layout, &matches)
    }
    .map_err(|err| ArchetectError::ConfigError(err.to_string()))?;

    let mut answers = ContextMap::new();
    // Load answers from merged configuration
    for (identifier, value) in configuration.answers() {
        answers.insert(identifier.clone(), value.clone());
    }
    load_explicit_answers(&matches, &mut answers)?;

    // MCP mode forces shell execution to Forbidden — no escape hatch.
    let configuration = if matches!(matches.subcommand(), Some(("mcp", _))) {
        configuration.with_shell_exec_policy(archetect_core::configuration::ShellExecPolicy::Forbidden)
    } else {
        configuration
    };

    // If --allow-exec is set (or env var, or config), emit a prominent warning.
    if matches!(
        configuration.shell_exec_policy(),
        archetect_core::configuration::ShellExecPolicy::Allowed
    ) {
        eprintln!(
            "\n\x1b[33;1m⚠  WARNING: Shell execution is enabled.\x1b[0m\n\
             Any archetype invoked from this point can run arbitrary commands\n\
             on your system without further confirmation.\n"
        );
    }

    let archetect = Archetect::builder()
        .with_configuration(configuration)
        .with_driver(driver)
        .with_layout(layout)
        .build()?;

    match matches.subcommand() {
        Some(("completions", args)) => cli::completions(args)?,
        Some(("ls", args)) => handle_commands_subcommand(args, &archetect),
        Some(("search", args)) => subcommands::handle_search_subcommand(args, &archetect),
        Some(("render", args)) => render(args, archetect, answers)?,
        Some(("global", args)) => execute_global_dispatch(args, archetect, answers)?,
        Some(("config", args)) => subcommands::handle_config_subcommand(args, &archetect)?,
        Some(("cache", args)) => subcommands::handle_cache_subcommand(args, &archetect)?,
        Some(("check", args)) => subcommands::handle_check_subcommand(args, &archetect)?,
        Some(("ide", args)) => {
            match args.subcommand() {
                Some(("setup", setup_args)) => {
                    let manage = subcommands::Manage::parse(setup_args.get_one::<String>("manage").map(String::as_str))?;
                    subcommands::handle_ide_subcommand(archetect.layout().as_ref(), manage)?
                }
                _ => {}
            }
        }
        Some(("system", args)) => {
            match args.subcommand() {
                Some(("layout", _)) => {
                    print!("{}", archetect.layout().as_ref());
                }
                _ => {
                    // Bare `archetect system` — show layout by default
                    print!("{}", archetect.layout().as_ref());
                }
            }
        }
        Some(("interface", args)) => {
            // Derivation wants the ARCHETYPE's interface, not "the
            // interface minus whatever this user's config pre-answers" —
            // only explicit -a/-A answers narrow the probe.
            let mut explicit_answers = ContextMap::new();
            load_explicit_answers(&matches, &mut explicit_answers)?;
            let switches = get_switches(&matches, archetect.configuration())?;
            subcommands::handle_interface_subcommand(args, &archetect, explicit_answers, switches)?
        }
        Some(("learn", args)) => subcommands::handle_learn_subcommand(args, &archetect)?,
        Some(("eval", args)) => subcommands::handle_eval_subcommand(args, &archetect)?,
        Some(("introspect", args)) => subcommands::handle_introspect_subcommand(args)?,
        Some(("skill", args)) => subcommands::handle_skill_subcommand(args)?,
        Some(("mcp", _)) => subcommands::handle_mcp_subcommand(archetect)?,
        Some(("server", args)) => subcommands::handle_server_subcommand(args, archetect)?,
        Some(("connect", args)) => {
            let render_context = configure_render_context(
                archetect_core::archetype::render_context::RenderContext::new(
                    camino::Utf8PathBuf::from(resolve_destination(args)),
                    answers,
                ),
                &archetect,
                args,
            )?;
            let client_cfg = archetect.configuration().client().cloned();
            let endpoint = subcommands::resolve_endpoint(args, client_cfg.as_ref())?;
            let options = subcommands::resolve_client_options(args, client_cfg.as_ref());
            archetect_core::client::start_with_options(render_context, endpoint, options)?;
        }
        Some((_, _args)) => {
            execute_catalog_dispatch(&matches, archetect, answers)?;
        },
        None => {
            execute_catalog_dispatch(&matches, archetect, answers)?;
        }
    }

    Ok(())
}

/// Dispatch for `archetect global [path]`. The configuration was loaded without
/// project detection (see `execute()`), so we just dispatch on the catalog directly.
/// The `path` argument is positional on the `global` subcommand.
fn execute_global_dispatch(
    args: &ArgMatches,
    archetect: Archetect,
    answers: ContextMap,
) -> Result<(), ArchetectError> {
    let catalog = archetect.configuration().catalog().ok_or_else(|| {
        ArchetectError::ConfigError(
            "No catalog defined in global configuration".to_string(),
        )
    })?;

    let destination = shellexpand::full(&resolve_destination(args))?.to_string();
    let destination = Utf8PathBuf::from(destination);
    let render_context = configure_render_context(
        RenderContext::new(destination, answers),
        &archetect,
        args,
    )?;

    let path_str = args.get_one::<String>("path").map(String::as_str).unwrap_or("");
    let path = if path_str.is_empty() { None } else { Some(path_str) };

    archetect_core::catalog::dispatch::dispatch(&archetect, catalog, path, render_context)?;
    Ok(())
}

/// New unified dispatch for `archetect [path] [destination]`.
///
/// Looks up the requested path in the resolved catalog (project's if a project
/// config was detected, otherwise global). Empty/default path → present the
/// catalog as a menu. Path → resolve and render (or present submenu if a group).
fn execute_catalog_dispatch(
    matches: &ArgMatches,
    archetect: Archetect,
    answers: ContextMap,
) -> Result<(), ArchetectError> {
    use clap::parser::ValueSource;

    let action_name = matches
        .get_one::<String>("action")
        .cloned()
        .unwrap_or_default();
    let action_was_default = matches.value_source("action") == Some(ValueSource::DefaultValue);

    let catalog = archetect.configuration().catalog().ok_or_else(|| {
        ArchetectError::ConfigError(
            "No catalog defined in configuration. The default config provides one — \
             check that your archetect.yaml hasn't accidentally cleared the catalog field."
                .to_string(),
        )
    })?;

    let destination = shellexpand::full(&resolve_destination(matches))?.to_string();
    let destination = Utf8PathBuf::from(destination);
    let render_context = configure_render_context(
        RenderContext::new(destination, answers),
        &archetect,
        matches,
    )?;

    // If the user didn't pass an explicit action and "default" isn't a catalog
    // entry, present the catalog as a menu instead of erroring.
    let path = if action_was_default && !catalog.contains_key(&action_name) {
        None
    } else {
        Some(action_name.as_str())
    };

    archetect_core::catalog::dispatch::dispatch(&archetect, catalog, path, render_context)?;
    Ok(())
}

/// Answers supplied explicitly on this invocation: `-A` files, then `-a`
/// flags (last wins). Values are parsed as YAML for consistent type
/// semantics with answer files:
///   -a count=42        → Integer
///   -a price=1.5       → Float
///   -a active=true     → Boolean
///   -a name=hello      → String
///   -a 'phone="5551234"' → String (YAML quoted)
///   -a 'tags=[a, b]'   → Array
///   -a 'db={host: localhost}' → Map
///   -a db.host=localhost → nested Map via dotted key
fn load_explicit_answers(matches: &ArgMatches, answers: &mut ContextMap) -> Result<(), ArchetectError> {
    if let Some(answer_files) = matches.get_many::<String>("answer-file") {
        for answer_file in answer_files {
            let results = answers::read_answers(answer_file)?;
            answers.extend(results);
        }
    }
    if let Some(answer_matches) = matches.get_many::<String>("answer") {
        for answer_match in answer_matches {
            let (key, raw_value) = parse_answer_pair(answer_match)
                .map_err(|e| ArchetectError::ConfigError(format!("Invalid answer '{}': {}", answer_match, e)))?;
            let value = parse_answer_value(&raw_value);
            insert_dotted(answers, &key, value);
        }
    }
    Ok(())
}

pub fn render(matches: &ArgMatches, archetect: Archetect, answers: ContextMap) -> Result<(), ArchetectError> {
    use clap::parser::ValueSource;

    let source = matches.get_one::<String>("source").expect("`source` is a required clap argument");
    let source = archetect.new_source(source)?;
    let destination = shellexpand::full(&resolve_destination(matches))?.to_string();
    let destination = Utf8PathBuf::from(destination);
    let render_context = configure_render_context(RenderContext::new(destination, answers), &archetect, matches)?;

    // Read the global `action` arg. If the user didn't supply one
    // explicitly, treat it as None (menu / default script path).
    let action_name = matches.get_one::<String>("action").cloned().unwrap_or_default();
    let action_was_default = matches.value_source("action") == Some(ValueSource::DefaultValue);
    let action = if action_was_default || action_name.is_empty() {
        None
    } else {
        Some(action_name.as_str())
    };

    match source.source_contents() {
        SourceContents::Archetype => {
            let archetype = Archetype::new(archetect, source)?;
            archetype.check_requirements()?;
            Ok(archetype.render_with_action(render_context, action).map(|_| ())?)
        }
        SourceContents::Unknown => {
            Err(SourceError::UnknownSourceContent.into())
        }
    }
}

fn configure_render_context(
    render_context: RenderContext,
    archetect: &Archetect,
    matches: &ArgMatches,
) -> Result<RenderContext, ArchetectError> {
    Ok(render_context
        .with_switches(get_switches(matches, archetect.configuration())?)
        .with_use_defaults_all(matches.get_flag("use-defaults-all"))
        .with_use_defaults(get_defaults(matches)?))
}

fn get_switches(matches: &ArgMatches, configuration: &Configuration) -> Result<HashSet<String>, ArchetectError> {
    let mut switches = HashSet::new();
    overlay_flag_tokens(
        &mut switches,
        configuration.switches().iter().map(String::as_str),
        "switch",
        "configuration",
    )?;
    if let Some(cli_switches) = matches.get_many::<String>("switches") {
        overlay_flag_tokens(
            &mut switches,
            cli_switches.map(String::as_str),
            "switch",
            "command line",
        )?;
    }
    Ok(switches)
}

fn get_defaults(matches: &ArgMatches) -> Result<HashSet<String>, ArchetectError> {
    let mut defaults = HashSet::new();
    if let Some(cli_defaults) = matches.get_many::<String>("use-defaults") {
        overlay_flag_tokens(
            &mut defaults,
            cli_defaults.map(String::as_str),
            "use-default",
            "command line",
        )?;
    }
    Ok(defaults)
}
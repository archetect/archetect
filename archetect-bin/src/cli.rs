use std::io;

use clap::builder::BoolishValueParser;
use clap::{command, value_parser, Arg, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use log::Level;

use archetect_core::errors::ArchetectError;

use crate::cli;
use crate::vendor::loggerv;

pub fn command() -> Command {
    command!()
        .name("archetect")
        .help_expected(true)
        .args(render_args(false))
        .subcommand(
            Command::new("render")
                .about("Render an Archetype")
                .long_about(
                    "Render an Archetype from an archetype or catalog directory or git URL.\n\
                     The destination defaults to the current directory; override with --dest <path>."
                )
                .arg(
                    Arg::new("source")
                        .help("The Archetype or Catalog source directory or git URL")
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    Arg::new("destination-pos")
                        .help("Directory to render into. Overrides --destination when both are supplied.")
                        .action(ArgAction::Set)
                        .required(false),
                )
                .args(render_args(true)),
        )
        .subcommand(
            Command::new("global")
                .about("Run a catalog action from the global config, bypassing any project .archetect.yaml")
                .long_about(
                    "Bypasses project config detection. Useful when you're inside a generated project\n\
                     (which has a .archetect.yaml that overrides the global catalog) but want to\n\
                     access the global catalog — for example, to bootstrap a sub-project."
                )
                .arg(
                    Arg::new("path")
                        .help("Catalog path to render (slash-separated). Empty = present the menu.")
                        .action(ArgAction::Set)
                        .default_value("")
                )
                .arg(
                    Arg::new("destination-pos")
                        .help("Directory to render into. Overrides --destination when both are supplied.")
                        .action(ArgAction::Set)
                        .required(false),
                )
                .args(render_args(true)),
        )
        .subcommand(
            Command::new("config")
                .arg_required_else_help(true)
                .about("Manage Archetect's configuration")
                .subcommand(Command::new("merged").about(
                    "Show Archetect's merged configuration after applying command line arguments, \
                    environment variables, and configuration files.",
                ))
                .subcommand(Command::new("defaults").about(
                    "Show Archetect's default configuration, which may be used for re-creating \
                    a configuration file.",
                ))
                .subcommand(Command::new("edit").about("Open Archetect's config file in an editor"))
                .args(render_args(true)),
        )
        .arg(
            Arg::new("action")
                .help("Catalog path to browse or named action to run")
                .long_help(
                    "Navigate to a catalog entry by path (e.g. 'services/grpc') or run a named\n\
                     action defined in global or project-local .archetect.yaml. Defaults to the\n\
                     configured default action."
                )
                .default_value("default")
                .action(ArgAction::Set)
                .global(true)
        )
        .arg(
            Arg::new("destination-pos")
                .help("Directory to render into (optional second positional)")
                .long_help(
                    "Target directory for any render the action performs. Mirrors\n\
                     `archetect render <source> <destination>` shape for the top-level\n\
                     action form. Overrides --destination when both are supplied."
                )
                .action(ArgAction::Set)
                .required(false),
        )
        .subcommand(
            Command::new("search")
                .visible_alias("find")
                .about("Search the resolved catalog by keyword (matches name, description, path, tags)")
                .long_about(
                    "Full-text search across the resolved catalog tree. All terms must\n\
                     match (AND semantics). Searches entry names, descriptions, paths,\n\
                     and metadata fields like languages, frameworks, and tags.\n\
                     \n\
                     Hidden entries (show: false) are excluded by default; pass -a / --all\n\
                     to include them.\n\
                     \n\
                     Examples:\n\
                     \n\
                     archetect search rust              # all rust-related entries\n\
                     archetect search rust cli          # entries matching both terms\n\
                     archetect search starter -a        # include hidden/component entries"
                )
                .arg(
                    clap::Arg::new("terms")
                        .help("One or more search terms (matched as AND)")
                        .action(clap::ArgAction::Append)
                        .num_args(1..)
                        .trailing_var_arg(true)
                )
                .arg(
                    clap::Arg::new("all")
                        .help("Include components, libraries, and entries with show: false")
                        .short('a')
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("ls")
                .visible_alias("list")
                .about("List the resolved catalog tree (project's if present, otherwise global)")
                .long_about(
                    "Recursively resolves and prints the catalog tree. Three entry kinds:\n\
                     \n\
                     \x20\x20📦 Archetype — renderable (resolved source has archetype.lua).\n\
                     \x20\x20📂 Catalog   — navigation node.\n\
                     \x20\x20🧩 Component — declared inside an archetype, or yaml show: false.\n\
                     \n\
                     Components are hidden by default; pass -a / --all to include them.\n\
                     An optional path filters the tree, preserving ancestor context so\n\
                     every visible indent is a path you can dispatch.\n\
                     \n\
                     Examples:\n\
                     \n\
                     archetect ls                         # archetypes + catalogs only\n\
                     archetect ls -a                      # include components / hidden\n\
                     archetect ls archetect/rust/cli      # filtered to a subtree"
                )
                .arg(
                    clap::Arg::new("ls-path")
                        .help("Catalog path to drill into (optional)")
                        .action(clap::ArgAction::Set)
                        .required(false)
                        .default_value("")
                )
                .arg(
                    clap::Arg::new("all")
                        .help("Include components, libraries, and entries with show: false")
                        .short('a')
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                )
        )
        .arg(
            Arg::new("verbosity")
                .help("Increase verbosity level")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .global(true),
        )
        .arg(
            Arg::new("config-file")
                .help("Supply an additional configuration file.")
                .long_help(
                    "Supply an additional configuration file to supplement or override \
                user and/or default configuration.",
                )
                .long("config-file")
                .short('c')
                .action(ArgAction::Set)
                .global(true)
                .value_name("config"),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("generator")
                        .help("Shell Generator")
                        .value_parser(value_parser!(Shell)),
                ),
        )
        .subcommand(
            Command::new("system")
                .about("Show Archetect system information")
                .subcommand(
                    Command::new("layout")
                        .about("Show system directory paths (config, cache, data)"),
                ),
        )
        .subcommand(
            Command::new("cache")
                .about("Manage Archetect's archetype/catalog cache")
                .subcommand(Command::new("clear").about("Remove Archetect's entire Repository Cache"))
                .subcommand(
                    Command::new("pull")
                        .about("Recursively pull all archetypes/catalogs reachable from a source or the configured catalog")
                        .long_about(
                            "Resolves the source (or the configured catalog if no source is given), walks its\n\
                             catalog tree, and pulls every reachable archetype. Following the 'archetypes all\n\
                             the way down' model: if a leaf entry points to an archetype that itself has a\n\
                             catalog, those entries are pulled too. Idempotent within a single run — sources\n\
                             are deduplicated."
                        )
                        .arg(
                            Arg::new("source")
                                .help("The archetype/catalog source (Git URL or local path). If omitted, pulls the configured catalog.")
                                .action(ArgAction::Set)
                        )
                )
                .subcommand(
                    Command::new("invalidate")
                        .about("Recursively invalidate cached archetypes/catalogs reachable from a source or the configured catalog")
                        .long_about(
                            "Walks the catalog tree and invalidates the cache timestamp for each reachable\n\
                             archetype, forcing a re-fetch on next render. If no source is given, invalidates\n\
                             the configured catalog."
                        )
                        .arg(
                            Arg::new("source")
                                .help("The archetype/catalog source (Git URL or local path). If omitted, invalidates the configured catalog.")
                                .action(ArgAction::Set)
                        )
                )
                .subcommand(
                    Command::new("prune")
                        .about("Reap materialized source trees unused past the configured retention")
                        .long_about(
                            "The cache keeps an immutable working tree per resolved commit. This removes trees\n\
                             not used within the retention window (default 90 days), skipping any a render\n\
                             session still holds. Safe to run anytime; runs opportunistically as well."
                        )
                ),
        )
        .subcommand(
            Command::new("check")
                .about("Check Archetect's environment for problems")
        )
        .subcommand(
            Command::new("ide")
                .about("IDE integration tools")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("setup")
                        .about("Set up IDE support for Lua archetypes")
                        .long_about(
                            "Installs Lua type annotations to ~/.local/share/archetect/lua/annotations/ for IDE\n\
                             autocompletion. If run inside a Lua archetype directory (containing archetype.yaml and\n\
                             archetype.lua), also creates or merges a .luarc.json pointing to the installed\n\
                             annotations. The merge is non-destructive: an entry from another tool (e.g. prova)\n\
                             is preserved, so `archetect ide setup` and `prova ide setup` can run in either order."
                        )
                        .arg(
                            Arg::new("manage")
                                .long("manage")
                                .value_name("auto|always|never")
                                .value_parser(["auto", "always", "never"])
                                .default_value("always")
                                .help(
                                    "How to manage .luarc.json: 'always' create-or-merge (default), 'auto' create \
                                     if absent else hint, 'never' install stubs only"
                                )
                        )
                )
        )
        .subcommand(
            Command::new("mcp")
                .about("Start MCP stdio server for AI agent integration")
        )
        .subcommand(
            Command::new("server")
                .about("Start Archetect Server")
                .arg(
                    Arg::new("host")
                        .help("The interface to bind to")
                        .long("host")
                        .default_value("0.0.0.0")
                        .action(ArgAction::Set)
                        .env("ARCHETECT_SERVER_HOST"),
                )
                .arg(
                    Arg::new("port")
                        .help("The port to listen on")
                        .long("port")
                        .short('p')
                        .default_value("8080")
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(u16))
                        .env("ARCHETECT_SERVER_PORT"),
                )
                .arg(
                    Arg::new("tls-cert")
                        .help("Path to a PEM-encoded TLS server certificate. Enables TLS when supplied with --tls-key.")
                        .long("tls-cert")
                        .env("ARCHETECT_SERVER_TLS_CERT")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("tls-key")
                        .help("Path to a PEM-encoded TLS server private key. Required with --tls-cert.")
                        .long("tls-key")
                        .env("ARCHETECT_SERVER_TLS_KEY")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("tls-client-ca")
                        .help("Path to a PEM-encoded CA cert for verifying client certificates (enables mutual TLS).")
                        .long("tls-client-ca")
                        .env("ARCHETECT_SERVER_TLS_CLIENT_CA")
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("connect")
                .about("Connect to an Archetect Server")
                .arg(
                    Arg::new("endpoint")
                        .help("The Archetect Server endpoint (e.g. http://localhost:8080). Falls back to client.endpoint in archetect.yaml if omitted.")
                        .action(ArgAction::Set)
                        .required(false),
                )
                .arg(
                    Arg::new("connect-timeout")
                        .help("TCP connect timeout in seconds (applied per attempt)")
                        .long("connect-timeout")
                        .env("ARCHETECT_CONNECT_TIMEOUT")
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(u64))
                        .default_value("5"),
                )
                .arg(
                    Arg::new("connect-retries")
                        .help("Maximum number of connect retry attempts before giving up")
                        .long("connect-retries")
                        .env("ARCHETECT_CONNECT_RETRIES")
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(u32))
                        .default_value("5"),
                )
                .arg(
                    Arg::new("tls")
                        .help("Enable TLS for the client connection. Implied when any other --tls-* flag is supplied.")
                        .long("tls")
                        .env("ARCHETECT_CLIENT_TLS")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("tls-ca")
                        .help("Path to a PEM-encoded CA cert to trust (in addition to the system trust store).")
                        .long("tls-ca")
                        .env("ARCHETECT_CLIENT_TLS_CA")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("tls-client-cert")
                        .help("Path to a PEM-encoded client certificate for mutual TLS.")
                        .long("tls-client-cert")
                        .env("ARCHETECT_CLIENT_TLS_CERT")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("tls-client-key")
                        .help("Path to a PEM-encoded client private key for mutual TLS. Required with --tls-client-cert.")
                        .long("tls-client-key")
                        .env("ARCHETECT_CLIENT_TLS_KEY")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("tls-domain")
                        .help("Override the domain name used for TLS SNI and cert verification.")
                        .long("tls-domain")
                        .env("ARCHETECT_CLIENT_TLS_DOMAIN")
                        .action(ArgAction::Set),
                )
                .args(render_args(true)),
        )
        .allow_external_subcommands(true)
}

fn render_args(global: bool) -> Vec<Arg> {
    let mut args = vec![];
    args.push(
        Arg::new("destination")
            .help("The directory to render the Archetype in to")
            .long("destination")
            .visible_alias("dest")
            .default_value(".")
            .action(ArgAction::Set)
            .global(global),
    );
    args.push(
        Arg::new("answer")
            .help("Supply a key=value pair as an answer to a variable question.")
            .long_help(VALID_ANSWER_INPUTS)
            .long("answer")
            .short('a')
            .action(ArgAction::Append)
            .value_name("prompt key=value")
            .global(global),
    );

    args.push(
        Arg::new("answer-file")
            .help("Supply an answers file in JSON or YAML format as answers to variable questions.")
            .long_help(
                "Supply an answers file in JSON or YAML format as answers to variable questions. This option may \
                     be specified more than once.",
            )
            .long("answer-file")
            .short('A')
            .action(ArgAction::Append)
            .value_name("path")
            .global(global)
        ,
    );

    args.push(
        Arg::new("use-defaults")
            .help("Use the configured default value for a prompt key ('<key>' or '<key>=false' to unset an inherited one)")
            .long("use-default")
            .short('d')
            .value_delimiter(',')
            .action(ArgAction::Append)
            .value_name("prompt key")
            .global(global),
    );

    args.push(
        Arg::new("use-defaults-all")
            .help("Use the configured default values for all prompts without explicit answers")
            .long("use-defaults-all")
            .short('D')
            .alias("use-defaults-unanswered")
            .action(ArgAction::SetTrue)
            .global(global),
    );

    args.push(
        Arg::new("switches")
            .help("Enable switches that may trigger functionality within Archetypes ('<name>' enables, '<name>=false' disables an inherited one)")
            .long("switch")
            .short('s')
            .action(ArgAction::Append)
            .value_name("switch name")
            .global(global),
    );

    args.push(
        Arg::new("offline")
            .help("Only use directories and already-cached remote git URLs")
            .short('o')
            .long("offline")
            .env("ARCHETECT_OFFLINE")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args.push(
        Arg::new("allow-exec")
            .help("Allow Archetypes to execute arbitrary commands")
            .long("allow-exec")
            .alias("ae")
            .short('e')
            .env("ARCHETECT_ALLOW_EXEC")
            .action(ArgAction::Set)
            .default_missing_value("true")
            .default_value("false")
            .num_args(0..=1)
            .value_parser(BoolishValueParser::new())
            .global(global),
    );
    args.push(
            Arg::new("headless")
                .help("Expect all inputs to be resolved by answers, defaults, and optional values, never waiting on interactive user input.")
                .long("headless")
                .env("ARCHETECT_HEADLESS")
                .action(ArgAction::SetTrue)
                .global(global)
        );
    args.push(
        Arg::new("local")
            .help("Use local development checkouts where available and configured")
            .long("local")
            .short('l')
            .env("ARCHETECT_LOCAL")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args.push(
        Arg::new("force-update")
            .help("Force updates for all Catalogs and Archetypes when rendering")
            .long("force-update")
            .short('U')
            .env("ARCHETECT_FORCE_UPDATE")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args.push(
        Arg::new("dry-run")
            .help("Show what would be rendered without writing files to disk")
            .long("dry-run")
            .short('n')
            .env("ARCHETECT_DRY_RUN")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args
}

pub fn configure(matches: &ArgMatches) {
    loggerv::Logger::new()
        .output(&Level::Error, loggerv::Output::Stderr)
        .output(&Level::Warn, loggerv::Output::Stderr)
        .output(&Level::Info, loggerv::Output::Stderr)
        .output(&Level::Debug, loggerv::Output::Stderr)
        .output(&Level::Trace, loggerv::Output::Stderr)
        .verbosity(matches.get_count("verbosity") as u64)
        .level(false)
        .prefix("archetect")
        .no_module_path()
        .module_path(false)
        .base_level(Level::Info)
        .init()
        .expect("loggerv initialization is infallible in this configuration");
}

pub fn completions(matches: &ArgMatches) -> Result<(), ArchetectError> {
    if let Some(generator) = matches.get_one::<Shell>("generator") {
        let mut command = cli::command();
        generate(*generator, &mut command, "archetect", &mut io::stdout());
    } else {
        return Err(ArchetectError::GeneralError("Invalid completions shell".to_owned()));
    }

    Ok(())
}

const VALID_ANSWER_INPUTS: &str = "Supply a key=value pair as an answer to a variable question. \
                                   This option may be specified more than once.\n\
                                   \nValid Input Examples:\n\
                                   \nkey=value\
                                   \nkey='multi-word value'\
                                   \nkey=\"multi-word value\"\
                                   \n\"key=value\"\
                                   \n'key=value'\
                                   \n'key=\"multi-word value\"'\
                                   \n\"key = 'multi-word value'\"\
                                   ";

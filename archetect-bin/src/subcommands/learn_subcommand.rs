//! `archetect learn` / `introspect` / `skill` — the autodidact surface on the CLI rail.
//! One renderer with the MCP tools (archetect-core `learn`/`help`), so the surfaces cannot
//! disagree; only the SPELLING of moves differs per transport.

use clap::ArgMatches;

use archetect_core::errors::ArchetectError;
use archetect_core::help;
use archetect_core::learn::{self, RenderEnv, Transport, SKILL};
use archetect_core::Archetect;

pub fn handle_learn_subcommand(args: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    let env = RenderEnv::from_configuration(archetect.configuration(), archetect.layout().as_ref());
    match learn::answer(args.get_one::<String>("topic").map(String::as_str), &env, Transport::Cli) {
        Ok(text) => {
            println!("{}", text.trim_end());
            Ok(())
        }
        Err(message) => Err(ArchetectError::GeneralError(format!("learn: {message}"))),
    }
}

pub fn handle_introspect_subcommand(args: &ArgMatches) -> Result<(), ArchetectError> {
    let entries = help::core_entries();
    let entries = match args.get_one::<String>("filter") {
        Some(needle) => help::filter(&entries, needle),
        None => entries,
    };
    if args.get_flag("json") {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "signature": e.signature,
                    "summary": e.summary,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "entries": items }))
                .unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    if entries.is_empty() {
        println!("(no entries match — try a shorter filter, or none for the whole surface)");
        return Ok(());
    }
    for e in &entries {
        println!("{} {}", e.name, e.signature);
        if !e.summary.is_empty() {
            println!("    {}", e.summary);
        }
    }
    Ok(())
}

pub fn handle_skill_subcommand(args: &ArgMatches) -> Result<(), ArchetectError> {
    if !args.get_flag("install") {
        print!("{SKILL}");
        return Ok(());
    }
    let root = std::env::current_dir()
        .map_err(|e| ArchetectError::GeneralError(format!("skill: cannot resolve cwd: {e}")))?;
    let dir = root.join(".claude/skills/archetect");
    let path = dir.join("SKILL.md");
    std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&path, SKILL))
        .map_err(|e| ArchetectError::GeneralError(format!("skill: could not write {}: {e}", path.display())))?;
    println!("wrote {}", path.display());
    Ok(())
}

use std::collections::HashSet;

use clap::ArgMatches;

use archetect_api::ContextMap;
use archetect_core::errors::ArchetectError;
use archetect_core::interface::{
    check_drift, probe_interface, DerivedInterface, InterfacePrompt, ProbeOptions,
};
use archetect_core::manifest::Manifest;
use archetect_core::system::{SystemLayout, XdgSystemLayout};
use archetect_core::Archetect;

/// `archetect interface <source>` — derive an archetype's interface by
/// probing it: run the script against a recording driver, print the
/// prompt transcript. `--json` for tooling, `--answers-template` for a
/// ready-to-fill `-A` file, `--check` to compare against a declared
/// (deprecated) `interface:` block, `--explore` to map branches.
pub fn handle_interface_subcommand(
    matches: &ArgMatches,
    archetect: &Archetect,
    answers: ContextMap,
    switches: HashSet<String>,
) -> Result<(), ArchetectError> {
    let target = matches
        .get_one::<String>("source")
        .expect("`source` is a required clap argument");

    // Accept a direct source OR a catalog path — sources first (a
    // path on disk should never be shadowed by a catalog entry name).
    let source = resolve_target(archetect, target)?;

    let options = ProbeOptions {
        answers,
        switches,
        explore: matches.get_flag("explore"),
        ..ProbeOptions::default()
    };

    let layout_factory = || -> Result<Box<dyn SystemLayout>, ArchetectError> {
        Ok(Box::new(XdgSystemLayout::new()?))
    };
    let derived = probe_interface(archetect, &layout_factory, &source, &options)?;

    if matches.get_flag("check") {
        return run_check(archetect, &source, &derived);
    }

    if matches.get_flag("answers-template") {
        print!("{}", answers_template(&source, &derived));
        return Ok(());
    }

    if matches.get_flag("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&derived)
                .map_err(|e| ArchetectError::GeneralError(e.to_string()))?
        );
        return Ok(());
    }

    print!("{}", human_summary(&source, &derived));
    Ok(())
}

fn resolve_target(archetect: &Archetect, target: &str) -> Result<String, ArchetectError> {
    if archetect.new_source(target).is_ok() {
        return Ok(target.to_string());
    }
    if let Some(catalog) = archetect.configuration().catalog() {
        if let Some(archetect_core::catalog::dispatch::PathTarget::Leaf(entry)) =
            archetect_core::catalog::dispatch::walk_path(archetect, catalog, target)
        {
            if let Some(source) = entry.source {
                return Ok(source);
            }
        }
    }
    Err(ArchetectError::GeneralError(format!(
        "'{}' is neither a resolvable source nor a catalog leaf path",
        target
    )))
}

fn run_check(
    archetect: &Archetect,
    source: &str,
    derived: &DerivedInterface,
) -> Result<(), ArchetectError> {
    let resolved = archetect.new_source(source)?;
    let manifest = Manifest::load(resolved.path()?)?;
    let Some(declared) = manifest.interface.as_ref() else {
        println!("no declared interface — nothing to drift. (The derived interface is the truth; keep it that way.)");
        return Ok(());
    };
    let findings = check_drift(declared, derived);
    if findings.is_empty() {
        println!(
            "declared interface agrees with the script ({} prompt(s), {} switch(es)) — safe to delete the declaration.",
            derived.prompts.len(),
            derived.switches.len()
        );
        return Ok(());
    }
    let mut report = String::from("interface drift detected:\n");
    for finding in &findings {
        report.push_str(&format!("  - {}\n", finding));
    }
    report.push_str("The script is the truth; fix or delete the declared interface.");
    Err(ArchetectError::GeneralError(report))
}

fn describe_prompt(prompt: &InterfacePrompt) -> String {
    let envelope = &prompt.envelope;
    let mut parts: Vec<String> = Vec::new();
    let type_name = serde_json::to_value(&envelope.prompt_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "?".into());
    parts.push(type_name);
    if envelope.default.is_none() && !envelope.optional {
        parts.push("required".into());
    }
    if let Some(default) = &envelope.default {
        parts.push(format!("default: {}", default));
    }
    if envelope.optional {
        parts.push("optional".into());
    }
    if let Some(pattern) = &envelope.pattern {
        parts.push(format!("pattern: {}", pattern));
    }
    if let Some(options) = &envelope.options {
        let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
        parts.push(format!("options: [{}]", values.join(", ")));
    }
    if let Some(group) = &envelope.group {
        parts.push(format!("group: {}", group));
    }
    for condition in &prompt.appears_when {
        parts.push(format!("when {} = {}", condition.key, condition.equals));
    }
    parts.join("  ·  ")
}

fn human_summary(source: &str, derived: &DerivedInterface) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Derived interface — {}\n", source));
    out.push_str(&format!(
        "mode: {:?} · coverage: {:?} · {} run(s){}{}\n\n",
        derived.mode,
        derived.coverage,
        derived.runs,
        if derived.budget_hit { " · BUDGET HIT" } else { "" },
        if derived.completed { "" } else { " · INCOMPLETE" },
    ));
    if let Some(error) = &derived.error {
        out.push_str(&format!("stopped by: {}\n\n", error));
    }
    if derived.prompts.is_empty() {
        out.push_str("(no prompts reached the driver — everything was pre-answered, or the script asks nothing)\n");
    } else {
        out.push_str("Prompts (answer with -a <key>=<value> / -A <file> / MCP answers):\n");
        for prompt in &derived.prompts {
            let key = prompt.envelope.key.clone().unwrap_or_else(|| "?".into());
            out.push_str(&format!("  {:<20} {}\n", key, describe_prompt(prompt)));
            out.push_str(&format!("  {:<20}   \"{}\"\n", "", prompt.envelope.message));
        }
    }
    if !derived.switches.is_empty() {
        out.push_str(&format!(
            "\nSwitches (enable with -s <name>; never prompted):\n  {}\n",
            derived.switches.join(", ")
        ));
    }
    out.push_str("\nHeadless one-shot: `archetect interface --answers-template` writes a fill-in answers file.\n");
    out
}

fn yaml_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => serde_yaml::to_string(s)
            .map(|y| y.trim_end().to_string())
            .unwrap_or_else(|_| format!("\"{}\"", s)),
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(yaml_scalar).collect();
            format!("[{}]", inner.join(", "))
        }
        other => other.to_string(),
    }
}

fn answers_template(source: &str, derived: &DerivedInterface) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Answers for {}\n# Generated by `archetect interface --answers-template`.\n# Use: archetect render <source> --destination <dir> --headless -A <this file>\n",
        source
    ));
    if !matches!(derived.coverage, archetect_core::interface::ProbeCoverage::Complete) {
        out.push_str(
            "# NOTE: coverage is not `complete` — branches taken with different answers may\n# surface prompts this template does not list (the render will name them).\n",
        );
    }
    out.push('\n');
    for prompt in &derived.prompts {
        let envelope = &prompt.envelope;
        let Some(key) = envelope.key.as_deref() else { continue };
        let mut annotation: Vec<String> = Vec::new();
        annotation.push(format!("{}", envelope.message));
        if let Some(pattern) = &envelope.pattern {
            annotation.push(format!("pattern: {}", pattern));
        }
        if let Some(options) = &envelope.options {
            let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
            annotation.push(format!("one of: [{}]", values.join(", ")));
        }
        for condition in &prompt.appears_when {
            annotation.push(format!("only when {} = {}", condition.key, condition.equals));
        }
        out.push_str(&format!("# {}\n", annotation.join(" — ")));
        match &envelope.default {
            Some(default) => out.push_str(&format!("{}: {}\n\n", key, yaml_scalar(default))),
            None if envelope.optional => {
                out.push_str(&format!("# {}:            # optional — uncomment to answer\n\n", key))
            }
            None => out.push_str(&format!(
                "# {}:            # REQUIRED — uncomment and fill in\n\n",
                key
            )),
        }
    }
    if !derived.switches.is_empty() {
        out.push_str(&format!(
            "# Switches this archetype consults (pass -s <name> on the CLI; not answerable here):\n#   {}\n",
            derived.switches.join(", ")
        ));
    }
    out
}

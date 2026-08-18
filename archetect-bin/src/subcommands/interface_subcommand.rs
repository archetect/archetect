use std::collections::{BTreeMap, HashSet};

use clap::ArgMatches;

use archetect_api::ContextMap;
use archetect_core::errors::ArchetectError;
use archetect_core::interface::{
    probe_interface, DerivedInterface, InterfaceNode, InterfacePrompt, InterfaceSegment,
    ProbeOptions,
};
use archetect_core::system::{SystemLayout, XdgSystemLayout};
use archetect_core::Archetect;

/// `archetect interface <source>` — derive an archetype's interface by
/// probing it: run the script against a recording driver, print the
/// prompt transcript. `--json` for tooling, `--answers-template` for a
/// ready-to-fill `-A` file, `--explore` to map branches.
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

/// Index the flat prompt list by the key a layout node references.
fn prompts_by_key(derived: &DerivedInterface) -> BTreeMap<String, &InterfacePrompt> {
    derived
        .prompts
        .iter()
        .map(|prompt| {
            let key = prompt
                .envelope
                .key
                .clone()
                .unwrap_or_else(|| prompt.envelope.message.clone());
            (key, prompt)
        })
        .collect()
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

fn summarize_node(
    node: &InterfaceNode,
    depth: usize,
    lookup: &BTreeMap<String, &InterfacePrompt>,
    seen: &mut HashSet<String>,
    out: &mut String,
) {
    let indent = "  ".repeat(depth + 1);
    match node {
        InterfaceNode::Prompt { key } => {
            seen.insert(key.clone());
            match lookup.get(key) {
                Some(prompt) => {
                    out.push_str(&format!("{}{:<20} {}\n", indent, key, describe_prompt(prompt)));
                    out.push_str(&format!("{}{:<20}   \"{}\"\n", indent, "", prompt.envelope.message));
                }
                // A layout key with no envelope would mean the two halves of
                // the probe disagreed; say so rather than drop the row.
                None => out.push_str(&format!("{}{:<20} (no envelope recorded)\n", indent, key)),
            }
        }
        InterfaceNode::Page(segment) => summarize_segment("PAGE", segment, depth, lookup, seen, out),
        InterfaceNode::Section(segment) => {
            summarize_segment("SECTION", segment, depth, lookup, seen, out)
        }
    }
}

fn summarize_segment(
    label: &str,
    segment: &InterfaceSegment,
    depth: usize,
    lookup: &BTreeMap<String, &InterfacePrompt>,
    seen: &mut HashSet<String>,
    out: &mut String,
) {
    let indent = "  ".repeat(depth + 1);
    out.push_str(&format!(
        "\n{}▸ {} {}  ({})\n",
        indent, label, segment.title, segment.key
    ));
    if let Some(help) = &segment.help {
        out.push_str(&format!("{}    {}\n", indent, help));
    }
    if segment.children.is_empty() {
        out.push_str(&format!("{}    (asks nothing)\n", indent));
    }
    for child in &segment.children {
        summarize_node(child, depth + 1, lookup, seen, out);
    }
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
    if derived.prompts.is_empty() && derived.layout.is_empty() {
        out.push_str("(no prompts reached the driver — everything was pre-answered, or the script asks nothing)\n");
    } else {
        out.push_str("Prompts (answer with -a <key>=<value> / -A <file> / MCP answers):\n");
        let lookup = prompts_by_key(derived);
        let mut seen: HashSet<String> = HashSet::new();
        let mut after_segment = false;
        for node in &derived.layout {
            // A loose prompt following a container needs the same breathing
            // room a container gets before itself.
            if after_segment && matches!(node, InterfaceNode::Prompt { .. }) {
                out.push('\n');
            }
            after_segment = !matches!(node, InterfaceNode::Prompt { .. });
            summarize_node(node, 0, &lookup, &mut seen, &mut out);
        }
        // Belt and braces: anything the layout somehow missed still shows up.
        for (key, prompt) in &lookup {
            if !seen.contains(key) {
                out.push_str(&format!("  {:<20} {}\n", key, describe_prompt(prompt)));
                out.push_str(&format!("  {:<20}   \"{}\"\n", "", prompt.envelope.message));
            }
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

fn template_entry(prompt: &InterfacePrompt, out: &mut String) {
    let envelope = &prompt.envelope;
    let Some(key) = envelope.key.as_deref() else { return };
    let mut annotation: Vec<String> = Vec::new();
    annotation.push(envelope.message.to_string());
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

/// Does this node contribute anything a human could fill in? A review page
/// that asks nothing is a real wizard step, but in an ANSWERS file it is a
/// heading over emptiness — so the template skips it.
fn node_has_answers(node: &InterfaceNode, lookup: &BTreeMap<String, &InterfacePrompt>) -> bool {
    match node {
        InterfaceNode::Prompt { key } => lookup
            .get(key)
            .is_some_and(|prompt| prompt.envelope.key.is_some()),
        InterfaceNode::Page(segment) | InterfaceNode::Section(segment) => segment
            .children
            .iter()
            .any(|child| node_has_answers(child, lookup)),
    }
}

/// Walk the layout, emitting each container as a comment banner above the
/// keys it covers. YAML keys stay at column zero — the file has to remain a
/// valid `-A` answer file, so the structure is carried in comments only.
fn template_node(
    node: &InterfaceNode,
    lookup: &BTreeMap<String, &InterfacePrompt>,
    seen: &mut HashSet<String>,
    out: &mut String,
) {
    if !node_has_answers(node, lookup) {
        return;
    }
    match node {
        InterfaceNode::Prompt { key } => {
            seen.insert(key.clone());
            if let Some(prompt) = lookup.get(key) {
                template_entry(prompt, out);
            }
        }
        InterfaceNode::Page(segment) => {
            out.push_str(&format!(
                "# ═══════════════════ {} ═══════════════════\n",
                segment.title
            ));
            if let Some(help) = &segment.help {
                out.push_str(&format!("# {}\n", help));
            }
            out.push('\n');
            for child in &segment.children {
                template_node(child, lookup, seen, out);
            }
        }
        InterfaceNode::Section(segment) => {
            out.push_str(&format!("# ─── {} ───\n", segment.title));
            if let Some(help) = &segment.help {
                out.push_str(&format!("# {}\n", help));
            }
            out.push('\n');
            for child in &segment.children {
                template_node(child, lookup, seen, out);
            }
        }
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

    let lookup = prompts_by_key(derived);
    let mut seen: HashSet<String> = HashSet::new();
    for node in &derived.layout {
        template_node(node, &lookup, &mut seen, &mut out);
    }
    for (key, prompt) in &lookup {
        if !seen.contains(key) {
            template_entry(prompt, &mut out);
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

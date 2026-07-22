//! The interface probe — derive an archetype's interface by asking it.
//!
//! Runs the archetype's script against a recording IO driver: every
//! prompt's envelope is captured, then auto-answered (default first,
//! else a type-synthetic value), writes are acknowledged without
//! touching disk, exec is forbidden, and `switches.is_enabled` queries
//! are recorded. The transcript IS the interface — same envelopes the
//! MCP session and terminal render from, delivered all at once.
//!
//! This is the engine behind `archetect interface <source>`, the MCP
//! `describe` tool, and (in exploration mode) the computed
//! batch/interactive classification. See
//! `docs/plans/dynamic-interface.md`.

mod probe_driver;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use archetect_api::{ContextMap, PromptEnvelope, PromptType};

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::configuration::ShellExecPolicy;
use crate::errors::ArchetectError;
use crate::source::SourceContents;
use crate::system::SystemLayout;
use crate::Archetect;

pub use probe_driver::ProbeDriver;

/// How much of the prompt tree a probe result covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeCoverage {
    /// Every reachable branch was explored (exploration mode, budget
    /// respected, all runs completed).
    Complete,
    /// One pass through the script answering defaults — branches taken
    /// elsewhere may hide further prompts.
    DefaultPath,
    /// The probe stopped early (budget, script error, or an input it
    /// could not synthesize). `prompts` holds the mapped prefix.
    Partial,
}

/// Batch/interactive classification — computed, never declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceMode {
    /// The full prompt tree is mapped: all inputs can be supplied up
    /// front and the render will not ask anything else.
    Batch,
    /// The script has behavior the probe could not fully map — drive it
    /// through the prompt-by-prompt session.
    Interactive,
}

/// A condition under which a prompt appears, derived from exploration:
/// "this prompt was only seen in runs where `key` = `equals`".
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppearsWhen {
    pub key: String,
    pub equals: serde_json::Value,
}

/// One prompt in the derived interface: its envelope plus, in
/// exploration mode, the conditions under which it appears.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfacePrompt {
    #[serde(flatten)]
    pub envelope: PromptEnvelope,
    /// Empty means the prompt appears unconditionally (on every explored
    /// path). Only populated by exploration mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub appears_when: Vec<AppearsWhen>,
}

/// The derived interface — the probe's transcript, structured.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DerivedInterface {
    pub mode: InterfaceMode,
    pub coverage: ProbeCoverage,
    pub prompts: Vec<InterfacePrompt>,
    /// Switch names the script consulted via `switches.is_enabled` —
    /// never prompted, so this recording is their only discovery path.
    pub switches: Vec<String>,
    /// True when the run(s) completed; false means `error` says why not.
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// True when the prompt-count budget stopped a run.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub budget_hit: bool,
    /// Number of probe runs performed (1 for default-path, more under
    /// exploration).
    pub runs: usize,
}

/// Probe configuration.
#[derive(Clone, Debug)]
pub struct ProbeOptions {
    /// Pre-supplied answers — prompts they satisfy never reach the
    /// driver, which deliberately narrows what gets probed.
    pub answers: ContextMap,
    /// Switches to enable for the run.
    pub switches: std::collections::HashSet<String>,
    /// Maximum prompts recorded per run before aborting (loop guard).
    pub prompt_budget: usize,
    /// Explore select/confirm branches instead of a single default path.
    pub explore: bool,
    /// Maximum probe runs in exploration mode.
    pub run_budget: usize,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        ProbeOptions {
            answers: ContextMap::new(),
            switches: Default::default(),
            prompt_budget: 256,
            explore: false,
            run_budget: 32,
        }
    }
}

/// One probe run's raw outcome.
struct RunOutcome {
    prompts: Vec<PromptEnvelope>,
    switches: Vec<String>,
    completed: bool,
    error: Option<String>,
    budget_hit: bool,
}

/// Probe an archetype source and derive its interface.
///
/// `base` supplies configuration + source resolution; the probe builds
/// its own `Archetect` around a recording driver, with shell exec
/// FORBIDDEN and all writes acknowledged but discarded.
pub fn probe_interface(
    base: &Archetect,
    layout_factory: &dyn Fn() -> Result<Box<dyn SystemLayout>, ArchetectError>,
    source: &str,
    options: &ProbeOptions,
) -> Result<DerivedInterface, ArchetectError> {
    let baseline = run_probe(base, layout_factory, source, options, &BTreeMap::new())?;

    if !options.explore {
        let coverage = if baseline.completed {
            ProbeCoverage::DefaultPath
        } else {
            ProbeCoverage::Partial
        };
        return Ok(DerivedInterface {
            // A single default path never proves batch-safety.
            mode: InterfaceMode::Interactive,
            coverage,
            prompts: baseline
                .prompts
                .iter()
                .map(|envelope| InterfacePrompt {
                    envelope: envelope.clone(),
                    appears_when: Vec::new(),
                })
                .collect(),
            switches: baseline.switches.clone(),
            completed: baseline.completed,
            error: baseline.error.clone(),
            budget_hit: baseline.budget_hit,
            runs: 1,
        });
    }

    explore(base, layout_factory, source, options, baseline)
}

/// Exploration: fork the probe at each select/confirm decision point,
/// one override per run, discovering prompts hidden behind non-default
/// branches. Coverage is per-decision (not the full cartesian product):
/// enough to map "conditional sections" keyed off a single choice — the
/// declared goal — while staying inside the run budget.
fn explore(
    base: &Archetect,
    layout_factory: &dyn Fn() -> Result<Box<dyn SystemLayout>, ArchetectError>,
    source: &str,
    options: &ProbeOptions,
    baseline: RunOutcome,
) -> Result<DerivedInterface, ArchetectError> {
    use serde_json::Value as Json;

    // (override-map, outcome) per run; baseline is run 0 with no overrides.
    let mut runs: Vec<(BTreeMap<String, Json>, RunOutcome)> = Vec::new();
    let mut planned: Vec<BTreeMap<String, Json>> = Vec::new();
    let mut seen_overrides: std::collections::HashSet<String> = Default::default();
    let mut decisions_seen: std::collections::HashSet<String> = Default::default();

    // Queue every non-default branch of every decision prompt in a run.
    fn plan_from(
        outcome: &RunOutcome,
        base_overrides: &BTreeMap<String, Json>,
        planned: &mut Vec<BTreeMap<String, Json>>,
        seen: &mut std::collections::HashSet<String>,
        decisions: &mut std::collections::HashSet<String>,
    ) {
        for envelope in &outcome.prompts {
            let Some(key) = envelope.key.clone() else { continue };
            if base_overrides.contains_key(&key) {
                continue;
            }
            let branch_values: Vec<Json> = match envelope.prompt_type {
                PromptType::Select => envelope
                    .options
                    .iter()
                    .flatten()
                    .map(|o| Json::String(o.value.clone()))
                    .collect(),
                PromptType::Bool => vec![Json::Bool(true), Json::Bool(false)],
                _ => continue,
            };
            decisions.insert(key.clone());
            for value in branch_values {
                let mut overrides = base_overrides.clone();
                overrides.insert(key.clone(), value);
                let fingerprint = serde_json::to_string(&overrides).unwrap_or_default();
                if seen.insert(fingerprint) {
                    planned.push(overrides);
                }
            }
        }
    }

    plan_from(&baseline, &BTreeMap::new(), &mut planned, &mut seen_overrides, &mut decisions_seen);
    runs.push((BTreeMap::new(), baseline));

    let mut budget_exhausted = false;
    while let Some(overrides) = planned.pop() {
        if runs.len() >= options.run_budget {
            budget_exhausted = true;
            break;
        }
        let outcome = run_probe(base, layout_factory, source, options, &overrides)?;
        // New decisions discovered down this branch get their own runs.
        plan_from(&outcome, &overrides, &mut planned, &mut seen_overrides, &mut decisions_seen);
        runs.push((overrides, outcome));
    }

    let all_completed = runs.iter().all(|(_, o)| o.completed);
    let any_budget_hit = runs.iter().any(|(_, o)| o.budget_hit);
    let fully_explored = !budget_exhausted && all_completed && !any_budget_hit;

    // Merge: prompts union in first-seen order.
    let mut prompts: Vec<InterfacePrompt> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut by_key: BTreeMap<String, (PromptEnvelope, Vec<usize>)> = BTreeMap::new();
    for (run_idx, (_, outcome)) in runs.iter().enumerate() {
        for envelope in &outcome.prompts {
            let key = envelope
                .key
                .clone()
                .unwrap_or_else(|| envelope.message.clone());
            let entry = by_key.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                (envelope.clone(), Vec::new())
            });
            entry.1.push(run_idx);
        }
    }

    let total_runs = runs.len();
    for key in &order {
        let (envelope, appeared_in) = &by_key[key];
        let mut appears_when = Vec::new();
        if appeared_in.len() < total_runs {
            // Conditional: for each decision key, if the runs where this
            // prompt appeared agree on a single value (and that decision
            // varies overall), that value is the condition.
            for decision in &decisions_seen {
                if decision == key {
                    continue;
                }
                let values: std::collections::HashSet<String> = appeared_in
                    .iter()
                    .filter_map(|&idx| runs[idx].0.get(decision).map(|v| v.to_string()))
                    .collect();
                let all_values: std::collections::HashSet<String> = runs
                    .iter()
                    .filter_map(|(o, _)| o.get(decision).map(|v| v.to_string()))
                    .collect();
                if values.len() == 1 && all_values.len() > 1 {
                    if let Some(value) = appeared_in
                        .iter()
                        .find_map(|&idx| runs[idx].0.get(decision).cloned())
                    {
                        appears_when.push(AppearsWhen {
                            key: decision.clone(),
                            equals: value,
                        });
                    }
                }
            }
        }
        prompts.push(InterfacePrompt {
            envelope: envelope.clone(),
            appears_when,
        });
    }

    let mut switches: Vec<String> = runs
        .iter()
        .flat_map(|(_, o)| o.switches.iter().cloned())
        .collect();
    switches.sort();
    switches.dedup();

    let first_error = runs.iter().find_map(|(_, o)| o.error.clone());
    Ok(DerivedInterface {
        mode: if fully_explored {
            InterfaceMode::Batch
        } else {
            InterfaceMode::Interactive
        },
        coverage: if fully_explored {
            ProbeCoverage::Complete
        } else if all_completed {
            ProbeCoverage::DefaultPath
        } else {
            ProbeCoverage::Partial
        },
        prompts,
        switches,
        completed: all_completed,
        error: first_error,
        budget_hit: any_budget_hit || budget_exhausted,
        runs: total_runs,
    })
}

/// Execute one probe run.
fn run_probe(
    base: &Archetect,
    layout_factory: &dyn Fn() -> Result<Box<dyn SystemLayout>, ArchetectError>,
    source: &str,
    options: &ProbeOptions,
    overrides: &BTreeMap<String, serde_json::Value>,
) -> Result<RunOutcome, ArchetectError> {
    // Probe configuration: exec is FORBIDDEN — the probe runs author code
    // without render intent; a script that requires exec fails the run
    // and classifies interactive, which is honest.
    let configuration = base
        .configuration()
        .clone()
        .with_shell_exec_policy(ShellExecPolicy::Forbidden);

    let driver = ProbeDriver::new(options.prompt_budget, overrides.clone());
    let state = driver.state();

    let archetect = Archetect::builder()
        .with_driver(driver)
        .with_configuration(configuration)
        .with_layout(layout_factory()?)
        .build()?;

    let resolved = archetect.new_source(source)?;
    match resolved.source_contents() {
        SourceContents::Archetype => {}
        SourceContents::Unknown => {
            return Err(ArchetectError::GeneralError(format!(
                "'{}' is not an archetype (no archetype.yaml)",
                source
            )));
        }
    }
    let archetype = Archetype::new(archetect.clone(), resolved)?;
    archetype.check_requirements()?;

    let recorder = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    // The destination is never written (the driver discards writes) but
    // scripts may read/format it — give them a plausible path.
    let destination = std::env::temp_dir().join("archetect-interface-probe");
    let render_context = RenderContext::new(
        camino::Utf8PathBuf::from_path_buf(destination).unwrap_or_else(|_| "archetect-interface-probe".into()),
        options.answers.clone(),
    )
    .with_switches(options.switches.clone())
    .with_switch_recorder(recorder.clone());

    let result = archetype.render(render_context);

    let recorded = state.lock().expect("probe state lock");
    let switches: Vec<String> = recorder
        .lock()
        .map(|set| set.iter().cloned().collect())
        .unwrap_or_default();

    Ok(RunOutcome {
        prompts: recorded.prompts.clone(),
        switches,
        completed: result.is_ok(),
        error: result.err().map(|e| e.to_string()),
        budget_hit: recorded.budget_hit,
    })
}

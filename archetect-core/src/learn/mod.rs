//! `archetect learn` — the progressive-disclosure topic catalog (docs/plans/autodidact.md).
//!
//! The embedded skill is the entry point; depth lives here, one screen per topic, so an agent
//! learns Archetect from the binary alone — no source tree, no docs site. Topics are static
//! doctrine (embedded markdown) plus **dynamic slots** (`{{slot}}`) computed from the resolved
//! configuration at the moment of asking, so a topic is always true for THIS environment and
//! degrades imperatively when nothing is configured.
//!
//! Invalid states are unrepresentable where the type system can manage it: a [`Topic`] without
//! content cannot compile (`include_str!` per variant, exhaustive matches), the slot vocabulary
//! is a closed enum, and the in-crate tests close the rest (every `{{slot}}` parses, every topic
//! titles itself, aliases never collide). Ported from prova's `learn.rs`; the engines will share
//! a crate once both stabilize (plan §4.2).

use crate::configuration::Configuration;
use crate::system::SystemLayout;

/// The embedded skill — the practice document served by `archetect skill` and as the MCP
/// server's `instructions`, so a connected agent starts knowing the loop.
pub const SKILL: &str = include_str!("skill.md");

/// Every topic the catalog serves. Adding a variant without a markdown file (or vice versa) is a
/// compile error; forgetting it in a match is too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Generation,
    Environment,
    Rendering,
    Authoring,
    Manifest,
    Prompts,
    Cases,
    Templates,
    Catalogs,
    Composition,
    Model,
    Sources,
    Mcp,
}

impl Topic {
    pub const ALL: &'static [Topic] = &[
        Topic::Generation,
        Topic::Environment,
        Topic::Rendering,
        Topic::Authoring,
        Topic::Manifest,
        Topic::Prompts,
        Topic::Cases,
        Topic::Templates,
        Topic::Catalogs,
        Topic::Composition,
        Topic::Model,
        Topic::Sources,
        Topic::Mcp,
    ];

    /// Intuitive names resolve instead of bouncing off our taxonomy (`archetect learn aml`
    /// works). Collisions with keys or each other are forbidden by test.
    const ALIASES: &'static [(&'static str, Topic)] = &[
        ("practice", Topic::Generation),
        ("loop", Topic::Generation),
        ("config", Topic::Environment),
        ("configuration", Topic::Environment),
        ("render", Topic::Rendering),
        ("headless", Topic::Rendering),
        ("flags", Topic::Rendering),
        ("answers", Topic::Rendering),
        ("switches", Topic::Rendering),
        ("archetype", Topic::Authoring),
        ("archetypes", Topic::Authoring),
        ("scripting", Topic::Authoring),
        ("lua", Topic::Authoring),
        ("context", Topic::Authoring),
        ("archetype.yaml", Topic::Manifest),
        ("interface", Topic::Prompts),
        ("prompt", Topic::Prompts),
        ("case", Topic::Cases),
        ("casing", Topic::Cases),
        ("inflections", Topic::Cases),
        ("template", Topic::Templates),
        ("atl", Topic::Templates),
        ("filters", Topic::Templates),
        ("catalog", Topic::Catalogs),
        ("federation", Topic::Catalogs),
        ("compose", Topic::Composition),
        ("components", Topic::Composition),
        ("libraries", Topic::Composition),
        ("library", Topic::Composition),
        ("aml", Topic::Model),
        ("modeling", Topic::Model),
        ("source", Topic::Sources),
        ("cache", Topic::Sources),
        ("git", Topic::Sources),
        ("locals", Topic::Sources),
        ("server", Topic::Mcp),
        ("agent", Topic::Mcp),
    ];

    pub fn key(self) -> &'static str {
        match self {
            Topic::Generation => "generation",
            Topic::Environment => "environment",
            Topic::Rendering => "rendering",
            Topic::Authoring => "authoring",
            Topic::Manifest => "manifest",
            Topic::Prompts => "prompts",
            Topic::Cases => "cases",
            Topic::Templates => "templates",
            Topic::Catalogs => "catalogs",
            Topic::Composition => "composition",
            Topic::Model => "model",
            Topic::Sources => "sources",
            Topic::Mcp => "mcp",
        }
    }

    /// The embedded doctrine. One file per variant; the pairing is what makes an undocumented
    /// topic unrepresentable.
    fn source(self) -> &'static str {
        match self {
            Topic::Generation => include_str!("topics/generation.md"),
            Topic::Environment => include_str!("topics/environment.md"),
            Topic::Rendering => include_str!("topics/rendering.md"),
            Topic::Authoring => include_str!("topics/authoring.md"),
            Topic::Manifest => include_str!("topics/manifest.md"),
            Topic::Prompts => include_str!("topics/prompts.md"),
            Topic::Cases => include_str!("topics/cases.md"),
            Topic::Templates => include_str!("topics/templates.md"),
            Topic::Catalogs => include_str!("topics/catalogs.md"),
            Topic::Composition => include_str!("topics/composition.md"),
            Topic::Model => include_str!("topics/model.md"),
            Topic::Sources => include_str!("topics/sources.md"),
            Topic::Mcp => include_str!("topics/mcp.md"),
        }
    }

    /// The one-line hook shown in the listing — parsed from the topic's own title line
    /// (`# <key> — <hook>`), so it is written exactly once. Format enforced by test.
    pub fn hook(self) -> &'static str {
        let first = self.source().lines().next().unwrap_or("");
        match first.split_once(" — ") {
            Some((_, hook)) => hook,
            None => first,
        }
    }

    pub fn resolve(input: &str) -> Option<Topic> {
        let needle = input.trim().to_lowercase();
        Topic::ALL
            .iter()
            .copied()
            .find(|t| t.key() == needle)
            .or_else(|| {
                Topic::ALIASES
                    .iter()
                    .find(|(alias, _)| *alias == needle)
                    .map(|(_, t)| *t)
            })
    }
}

/// Which surface is asking. The truth is identical; the SPELLING of moves is not — an MCP agent
/// calls tools, a CLI agent runs commands, and each learns the other exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Cli,
    Mcp,
}

/// The closed slot vocabulary. A `{{name}}` outside this enum fails the in-crate tests, and
/// every variant must render (exhaustive match), including its unconfigured degradation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    CatalogTree,
    Locals,
    CacheState,
    ProjectConfig,
    Annotations,
}

impl Slot {
    fn parse(name: &str) -> Option<Slot> {
        match name {
            "catalog_tree" => Some(Slot::CatalogTree),
            "locals" => Some(Slot::Locals),
            "cache_state" => Some(Slot::CacheState),
            "project_config" => Some(Slot::ProjectConfig),
            "annotations" => Some(Slot::Annotations),
            _ => None,
        }
    }
}

/// One top-level entry of the configured catalog, pre-digested for rendering.
pub struct CatalogFact {
    pub key: String,
    /// "group" | "archetype" | "remote".
    pub kind: &'static str,
    pub description: String,
}

/// What the renderer knows about where it is running — plain data, computed by whichever surface
/// has a [`Configuration`] in hand, so CLI and MCP cannot disagree about how facts are gathered.
#[derive(Default)]
pub struct RenderEnv {
    /// Top-level entries of the configured catalog, in declaration order. `None` = no catalog
    /// configured at all.
    pub catalog: Option<Vec<CatalogFact>>,
    pub locals_enabled: bool,
    pub locals_paths: Vec<String>,
    /// Cache root + materialized tree count, when the layout could be read.
    pub cache_dir: Option<String>,
    pub cache_trees: Option<usize>,
    /// The project `.archetect.yaml` in reach of the working directory, if any.
    pub project_config: Option<String>,
    /// Whether the IDE annotation stubs are installed (`None` = could not determine).
    pub annotations_installed: Option<bool>,
    /// Switches enabled by configuration (config files + project config).
    pub switches: Vec<String>,
}

impl RenderEnv {
    /// Compute the environment's facts from a resolved configuration + system layout — the same
    /// pair every verb already runs with.
    pub fn from_configuration(config: &Configuration, layout: &dyn SystemLayout) -> RenderEnv {
        let catalog = config.catalog().map(|entries| {
            entries
                .iter()
                .map(|(key, entry)| CatalogFact {
                    key: key.clone(),
                    kind: if entry.is_group() {
                        "group"
                    } else if entry.is_remote() {
                        "remote"
                    } else {
                        "archetype"
                    },
                    description: entry.description.clone().unwrap_or_default(),
                })
                .collect()
        });

        let cache_dir = layout.cache_dir();
        let trees_root = std::path::Path::new(cache_dir.as_str()).join("trees");
        let cache_trees = std::fs::read_dir(&trees_root).ok().map(|repos| {
            repos
                .filter_map(|r| r.ok())
                .filter_map(|r| std::fs::read_dir(r.path()).ok())
                .flat_map(|trees| trees.filter_map(|t| t.ok()))
                .filter(|t| t.path().is_dir())
                .count()
        });

        let project_config = std::env::current_dir().ok().and_then(|cwd| {
            cwd.ancestors()
                .map(|dir| dir.join(".archetect.yaml"))
                .find(|p| p.exists())
                .map(|p| p.display().to_string())
        });

        let annotations = std::path::Path::new(layout.data_dir().as_str())
            .join("lua/annotations/archetect.lua");

        RenderEnv {
            catalog,
            locals_enabled: config.locals().enabled(),
            locals_paths: config
                .locals()
                .paths()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            cache_dir: Some(cache_dir.to_string()),
            cache_trees,
            project_config,
            annotations_installed: Some(annotations.exists()),
            switches: config.switches().to_vec(),
        }
    }
}

fn render_slot(slot: Slot, env: &RenderEnv, transport: Transport) -> String {
    match slot {
        Slot::CatalogTree => match &env.catalog {
            Some(entries) if !entries.is_empty() => {
                let width = entries.iter().map(|e| e.key.len()).max().unwrap_or(0);
                let mut rows: Vec<String> = entries
                    .iter()
                    .map(|e| {
                        let marker = match e.kind {
                            "group" => "▸",
                            "remote" => "🛰",
                            _ => " ",
                        };
                        format!("  {marker} {:<width$}  {}", e.key, e.description)
                    })
                    .collect();
                rows.push(String::new());
                rows.push(match transport {
                    Transport::Cli => "Walk it: `archetect ls [path]` · `archetect search \
                                       <terms>` · render: `archetect <path>`."
                        .into(),
                    Transport::Mcp => "Walk it: `catalog_browse { path? }` · `catalog_search { \
                                       query }` · render: `catalog_render { path, destination }`."
                        .into(),
                });
                format!("**Configured catalog** (top level):\n{}", rows.join("\n"))
            }
            Some(_) => "**Configured catalog**: declared but empty.".into(),
            None => match transport {
                Transport::Cli => "**Configured catalog**: none — configure one in \
                                   `~/.config/archetect/config.yaml` (`archetect config merged` \
                                   shows what resolved), or render sources directly: `archetect \
                                   render <git-url|path>`."
                    .into(),
                Transport::Mcp => "**Configured catalog**: none — `catalog_browse`/`catalog_render` \
                                   have nothing to serve; use `render { source, destination }` \
                                   with an explicit git URL or path."
                    .into(),
            },
        },
        Slot::Locals => {
            if env.locals_enabled && !env.locals_paths.is_empty() {
                format!(
                    "**Locals**: ENABLED — sources whose repo directory name exists under {} \
                     short-circuit the remote clone to that checkout (dev mode; `-l/--local` \
                     toggles per run).",
                    env.locals_paths.join(", ")
                )
            } else if env.locals_enabled {
                "**Locals**: enabled, but no paths configured.".into()
            } else {
                "**Locals**: disabled — every git source resolves through the cache \
                 (`-l/--local` enables per run when configured)."
                    .into()
            }
        }
        Slot::CacheState => match (&env.cache_dir, env.cache_trees) {
            (Some(dir), Some(trees)) => format!(
                "**Cache**: {trees} materialized tree(s) under `{dir}` — content-addressed by \
                 commit; tags/commits never re-probe, branches re-check after the configured \
                 interval. `archetect cache pull|invalidate|prune|clear` manage it (CLI only)."
            ),
            (Some(dir), None) => format!(
                "**Cache**: empty (nothing materialized yet) — will populate under `{dir}` on \
                 the first remote render."
            ),
            _ => "**Cache**: location unknown (system layout unavailable).".into(),
        },
        Slot::ProjectConfig => match &env.project_config {
            Some(path) => {
                let switches = if env.switches.is_empty() {
                    String::new()
                } else {
                    format!(" Active switches: {}.", env.switches.join(", "))
                };
                format!(
                    "**Project config**: `{path}` is in reach — its catalog/answers/switches \
                     overlay the user config for renders from here.{switches}"
                )
            }
            None => "**Project config**: no `.archetect.yaml` in reach of the working directory \
                     — the user config alone applies."
                .into(),
        },
        Slot::Annotations => match env.annotations_installed {
            Some(true) => "**IDE annotations**: installed — editors with LuaLS get completion \
                           and hover for the whole scripting API."
                .into(),
            Some(false) => "**IDE annotations**: NOT installed — run `archetect ide setup` \
                            (shell out for it; no MCP tool) so editors and LuaLS see the API."
                .into(),
            None => String::new(),
        },
    }
}

/// Render a topic for a transport, substituting every slot from the environment. An unknown slot
/// is a bug caught by the in-crate tests; at runtime it renders as an explicit marker rather
/// than vanishing silently.
///
/// Slots are `[[slot:name]]`, NOT `{{name}}` — the templates/cases topics teach ATL, whose
/// interpolation syntax IS `{{ }}`, so the learn engine must not collide with its own examples.
pub fn render(topic: Topic, env: &RenderEnv, transport: Transport) -> String {
    let mut out = String::new();
    let mut rest = topic.source();
    while let Some(open) = rest.find("[[slot:") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 7..];
        match after.find("]]") {
            Some(close) => {
                let name = after[..close].trim();
                match Slot::parse(name) {
                    Some(slot) => out.push_str(&render_slot(slot, env, transport)),
                    None => out.push_str(&format!("(unknown slot `{name}`)")),
                }
                rest = &after[close + 2..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    // Empty slot renders leave runs of blank lines behind — collapse them so a degraded topic
    // reads clean, not gappy.
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out
}

/// The catalog listing: `key  hook` rows and the transport-appropriate next move.
pub fn listing(transport: Transport) -> String {
    let width = Topic::ALL.iter().map(|t| t.key().len()).max().unwrap_or(0);
    let mut out = vec![
        "Topics — progressive disclosure, one screen each:".to_string(),
        String::new(),
    ];
    for topic in Topic::ALL {
        out.push(format!("  {:<width$}  {}", topic.key(), topic.hook()));
    }
    out.push(String::new());
    out.push(match transport {
        Transport::Cli => "Read one: `archetect learn <topic>`.".to_string(),
        Transport::Mcp => "Read one: `learn { topic = \"<topic>\" }`.".to_string(),
    });
    out.join("\n")
}

/// Answer a `learn` ask — the ONE path every surface (CLI, MCP tool, MCP resources) goes
/// through, so they cannot disagree. `Err` is the usage-error text (unknown topic); the caller
/// decides exit code vs error result.
pub fn answer(topic: Option<&str>, env: &RenderEnv, transport: Transport) -> Result<String, String> {
    let name = match topic.map(str::trim) {
        None | Some("") => return Ok(listing(transport)),
        Some(name) => name,
    };
    match Topic::resolve(name) {
        Some(topic) => Ok(render(topic, env, transport)),
        None => Err(format!(
            "unknown topic {name:?}\n\n{}",
            listing(transport)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enumerate every `[[slot:name]]` occurrence across all topics.
    fn slots_in(source: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = source;
        while let Some(open) = rest.find("[[slot:") {
            let after = &rest[open + 7..];
            let Some(close) = after.find("]]") else { break };
            out.push(after[..close].trim().to_string());
            rest = &after[close + 2..];
        }
        out
    }

    /// The slot vocabulary is CLOSED: every `{{name}}` a topic uses parses to a Slot variant.
    #[test]
    fn every_slot_in_every_topic_is_in_the_vocabulary() {
        for topic in Topic::ALL {
            for name in slots_in(topic.source()) {
                assert!(
                    Slot::parse(&name).is_some(),
                    "topic `{}` uses unknown slot `{{{{{name}}}}}`",
                    topic.key()
                );
            }
        }
    }

    /// Every topic titles itself `# <key> — <hook>`: the listing hook is parsed from the title,
    /// so it is written once and cannot drift from the content.
    #[test]
    fn every_topic_titles_itself_with_its_key_and_hook() {
        for topic in Topic::ALL {
            let first = topic.source().lines().next().unwrap_or("");
            assert!(
                first.starts_with(&format!("# {} — ", topic.key())),
                "topic `{}` must start `# {} — <hook>`, got {first:?}",
                topic.key(),
                topic.key()
            );
            assert!(!topic.hook().is_empty(), "topic `{}` has an empty hook", topic.key());
        }
    }

    /// Aliases resolve, never collide with a key or each other, and every key resolves to itself.
    #[test]
    fn aliases_resolve_and_never_collide() {
        for topic in Topic::ALL {
            assert_eq!(Topic::resolve(topic.key()), Some(*topic));
        }
        let mut seen = std::collections::BTreeSet::new();
        for (alias, target) in Topic::ALIASES {
            assert!(seen.insert(*alias), "alias {alias:?} appears twice");
            assert!(
                Topic::ALL.iter().all(|t| t.key() != *alias),
                "alias {alias:?} shadows a topic key"
            );
            assert_eq!(Topic::resolve(alias), Some(*target));
        }
        assert_eq!(Topic::resolve("aml"), Some(Topic::Model));
        assert_eq!(Topic::resolve("no-such-topic"), None);
    }

    /// Every topic renders with a default (unconfigured) environment and stays one-screen-ish.
    #[test]
    fn every_topic_renders_unconfigured_and_stays_terse() {
        let env = RenderEnv::default();
        for topic in Topic::ALL {
            for transport in [Transport::Cli, Transport::Mcp] {
                let text = render(*topic, &env, transport);
                assert!(!text.trim().is_empty(), "topic `{}` rendered empty", topic.key());
                assert!(
                    !text.contains("[[slot:"),
                    "topic `{}` leaked an unrendered slot",
                    topic.key()
                );
                assert!(
                    !text.contains("\n\n\n"),
                    "topic `{}` renders gappy (empty slots must collapse)",
                    topic.key()
                );
                let lines = text.lines().count();
                assert!(
                    lines <= 90,
                    "topic `{}` is {lines} lines — split it (one screen per topic)",
                    topic.key()
                );
            }
        }
    }

    /// The listing carries every key and the transport-appropriate next move.
    #[test]
    fn listing_names_every_topic_and_the_next_move() {
        for transport in [Transport::Cli, Transport::Mcp] {
            let text = listing(transport);
            for topic in Topic::ALL {
                assert!(text.contains(topic.key()));
            }
        }
        assert!(listing(Transport::Cli).contains("archetect learn <topic>"));
        assert!(listing(Transport::Mcp).contains("learn { topic"));
    }

    /// `answer` is the one path every surface shares: listing, topic, alias, unknown.
    #[test]
    fn answer_routes_listing_topic_and_unknown() {
        let env = RenderEnv::default();
        assert!(answer(None, &env, Transport::Cli).unwrap().contains("templates"));
        assert!(answer(Some("atl"), &env, Transport::Cli).unwrap().contains("filter"));
        let err = answer(Some("nope"), &env, Transport::Cli).unwrap_err();
        assert!(err.contains("unknown topic"));
        assert!(err.contains("generation"), "the listing rides the error");
    }

    /// The skill is embedded, non-empty, and teaches the learn rail.
    #[test]
    fn the_skill_teaches_the_learn_rail() {
        assert!(SKILL.contains("archetect learn"));
        assert!(SKILL.contains("introspect"));
    }
}

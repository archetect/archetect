use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};

use archetect_api::{ContextMap, ContextValue};

/// Shared recorder for `archetype.switches.is_enabled` queries. The
/// interface probe attaches one to observe which switch names a script
/// consults — switches are never prompted, so this is the only way a
/// derived interface can discover them. Cloned into child render
/// contexts, so composition records into one set.
pub type SwitchRecorder = Arc<Mutex<BTreeSet<String>>>;

#[derive(Clone, Debug)]
pub struct RenderContext {
    destination: Utf8PathBuf,
    answers: ContextMap,
    use_defaults: HashSet<String>,
    use_defaults_all: bool,
    switches: HashSet<String>,
    settings: ContextMap,
    switch_recorder: Option<SwitchRecorder>,
}

impl RenderContext {
    /// Construct a render context. The `destination` is absolutized
    /// (joined against the process CWD if relative) and lexically
    /// normalized (`.` / `..` segments collapsed) before being stored,
    /// so `self.destination()` always returns an absolute path — the
    /// contract downstream consumers (templates, Lua modules, the
    /// include resolver, scm-library) depend on.
    ///
    /// No filesystem access: normalization is purely lexical, so a
    /// destination that doesn't yet exist (the common case for new
    /// scaffolds) works fine. Paths that are already absolute pass
    /// through `normalize` only.
    pub fn new<T: Into<Utf8PathBuf>>(destination: T, answers: ContextMap) -> RenderContext {
        RenderContext {
            destination: absolutize(destination.into()),
            answers,
            use_defaults: Default::default(),
            use_defaults_all: false,
            switches: Default::default(),
            settings: Default::default(),
            switch_recorder: None,
        }
    }

    /// Attach a recorder observing `switches.is_enabled` queries — see
    /// [`SwitchRecorder`]. Used by the interface probe.
    pub fn with_switch_recorder(mut self, recorder: SwitchRecorder) -> Self {
        self.switch_recorder = Some(recorder);
        self
    }

    pub fn switch_recorder(&self) -> Option<&SwitchRecorder> {
        self.switch_recorder.as_ref()
    }

    pub fn answers(&self) -> &ContextMap {
        &self.answers
    }

    pub fn answers_mut(&mut self) -> &mut ContextMap {
        &mut self.answers
    }

    pub fn answers_owned(&self) -> ContextMap {
        self.answers.clone()
    }

    pub fn destination(&self) -> &Utf8Path {
        self.destination.as_path()
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.switches
    }

    pub fn with_switch<S: Into<String>>(mut self, switch: S) -> Self {
        self.switches.insert(switch.into());
        self
    }

    pub fn with_switches(mut self, switches: HashSet<String>) -> Self {
        self.set_switches(switches);
        self
    }

    pub fn set_switches(&mut self, switches: HashSet<String>) {
        self.switches = switches;
    }

    pub fn settings(&self) -> &ContextMap {
        &self.settings
    }

    pub fn with_settings(mut self, settings: ContextMap) -> Self {
        if let Some(ContextValue::Array(switches)) = settings.get("switches") {
            self.switches = switches
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        self.settings = settings;
        self
    }

    pub fn use_defaults(&self) -> &HashSet<String> {
        &self.use_defaults
    }

    pub fn with_use_default<D: Into<String>>(mut self, default: D) -> Self {
        self.use_defaults.insert(default.into());
        self
    }

    pub fn with_use_defaults(mut self, defaults: HashSet<String>) -> Self {
        self.set_use_defaults(defaults);
        self
    }

    pub fn set_use_defaults(&mut self, use_defaults: HashSet<String>) {
        self.use_defaults = use_defaults;
    }

    pub fn use_defaults_all(&self) -> bool {
        self.use_defaults_all
    }

    pub fn with_use_defaults_all(mut self, value: bool) -> Self {
        self.set_use_defaults_all(value);
        self
    }

    pub fn set_use_defaults_all(&mut self, value: bool) {
        self.use_defaults_all = value;
    }
}

// ── Destination normalization helpers ──────────────────────────────────
//
// archetect-bin defaults the destination to `"."` when the user doesn't
// pass `-d`, and `Utf8PathBuf::from(".")` happily stores it verbatim.
// Downstream consumers (the `archetype.destination` Lua global, the
// include resolver, libraries that want to reason about the output dir
// name) all expect an absolute path. Absorbing the resolution here
// means every `RenderContext::new` callsite — CLI, server, MCP,
// tests — gets the same guarantee for free.

/// Make `path` absolute by joining against the process CWD when
/// relative, then lexically normalizing. Pure — never touches the
/// filesystem, so paths that don't yet exist (the common new-scaffold
/// case) resolve fine.
///
/// Falls back to returning the path unchanged if `std::env::current_dir()`
/// fails or isn't valid UTF-8. Both are rare (CWD deleted out from
/// under us, non-UTF-8 filesystem names) and we prefer to ship a
/// best-effort path than refuse to render.
fn absolutize(path: Utf8PathBuf) -> Utf8PathBuf {
    if path.is_absolute() {
        return normalize(path);
    }
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(_) => return path,
    };
    let cwd = match Utf8PathBuf::from_path_buf(cwd) {
        Ok(c) => c,
        Err(_) => return path,
    };
    normalize(cwd.join(path))
}

/// Lexically collapse `.` and `..` segments. Pure — no fs access,
/// no symlink resolution. For absolute paths, `..` at the root is
/// dropped (can't escape the filesystem root). For relative paths
/// that start with `..`, the leading `..` segments are preserved.
fn normalize(path: Utf8PathBuf) -> Utf8PathBuf {
    let is_absolute = path.is_absolute();
    let mut out = Utf8PathBuf::new();
    for comp in path.components() {
        match comp {
            Utf8Component::CurDir => {}
            Utf8Component::ParentDir => {
                let can_pop = out
                    .components()
                    .last()
                    .map(|c| matches!(c, Utf8Component::Normal(_)))
                    .unwrap_or(false);
                if can_pop {
                    out.pop();
                } else if !is_absolute {
                    out.push("..");
                }
                // else: absolute path at root — drop the `..`
            }
            other => out.push(other.as_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_curdir() {
        assert_eq!(normalize(Utf8PathBuf::from("/a/./b")), Utf8PathBuf::from("/a/b"));
        assert_eq!(normalize(Utf8PathBuf::from("./a/b")), Utf8PathBuf::from("a/b"));
    }

    #[test]
    fn normalize_collapses_parentdir() {
        assert_eq!(normalize(Utf8PathBuf::from("/a/b/../c")), Utf8PathBuf::from("/a/c"));
        assert_eq!(normalize(Utf8PathBuf::from("a/b/../c")), Utf8PathBuf::from("a/c"));
    }

    #[test]
    fn normalize_drops_parentdir_at_absolute_root() {
        assert_eq!(normalize(Utf8PathBuf::from("/..")), Utf8PathBuf::from("/"));
        assert_eq!(normalize(Utf8PathBuf::from("/../../a")), Utf8PathBuf::from("/a"));
    }

    #[test]
    fn normalize_preserves_leading_parentdir_in_relative() {
        assert_eq!(normalize(Utf8PathBuf::from("../a")), Utf8PathBuf::from("../a"));
        assert_eq!(normalize(Utf8PathBuf::from("../../a/b")), Utf8PathBuf::from("../../a/b"));
    }

    #[test]
    fn absolutize_absolute_path_is_idempotent() {
        let p = Utf8PathBuf::from("/tmp/foo");
        assert_eq!(absolutize(p.clone()), p);
    }

    #[test]
    fn absolutize_dot_resolves_to_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let cwd = Utf8PathBuf::from_path_buf(cwd).unwrap();
        assert_eq!(absolutize(Utf8PathBuf::from(".")), cwd);
    }

    #[test]
    fn absolutize_relative_path_joins_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let cwd = Utf8PathBuf::from_path_buf(cwd).unwrap();
        assert_eq!(absolutize(Utf8PathBuf::from("foo/bar")), cwd.join("foo/bar"));
    }

    #[test]
    fn render_context_new_stores_absolute_destination() {
        // The key promise: `.destination()` is always absolute.
        let ctx = RenderContext::new(Utf8PathBuf::from("."), ContextMap::new());
        assert!(ctx.destination().is_absolute());
    }

    #[test]
    fn render_context_new_preserves_already_absolute() {
        let ctx = RenderContext::new(Utf8PathBuf::from("/tmp/test"), ContextMap::new());
        assert_eq!(ctx.destination(), Utf8Path::new("/tmp/test"));
    }
}

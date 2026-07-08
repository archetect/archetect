//! Flag-bag token parsing and overlay resolution for switches and
//! use-defaults.
//!
//! See `docs/specs/flag-resolution-semantics.md`. A token is `name`,
//! `name=true`, or `name=false`; anything else is an error. Layers are
//! folded lowest-precedence first with [`overlay_flag_tokens`]: each
//! layer's additions apply before its removals, and items a layer
//! doesn't mention are untouched.

use std::collections::HashSet;

use crate::errors::ArchetectError;

/// A parsed flag directive: enable or disable a named flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlagDirective {
    pub name: String,
    pub enabled: bool,
}

/// Parse a single flag token: `name`, `name=true`, or `name=false`.
///
/// `kind` names the flag family ("switch", "use-default") for error
/// messages; `source` names where the token came from (e.g. a file
/// path, "command line", "catalog entry 'services/grpc'").
pub fn parse_flag_token(token: &str, kind: &str, source: &str) -> Result<FlagDirective, ArchetectError> {
    let (name, enabled) = match token.split_once('=') {
        None => (token, true),
        Some((name, "true")) => (name, true),
        Some((name, "false")) => (name, false),
        Some((_, value)) => {
            return Err(ArchetectError::GeneralError(format!(
                "Invalid {kind} '{token}' (from {source}): expected '<name>', '<name>=true', or '<name>=false', got value '{value}'"
            )));
        }
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(ArchetectError::GeneralError(format!(
            "Invalid {kind} '{token}' (from {source}): name must not be empty"
        )));
    }
    Ok(FlagDirective {
        name: name.to_string(),
        enabled,
    })
}

/// Overlay one layer of flag tokens onto an accumulated set.
///
/// Within the layer, additions apply before removals, so ordering
/// inside a single layer never matters and `[x, x=false]` in one layer
/// deterministically resolves to "removed". Items the layer doesn't
/// mention are untouched.
pub fn overlay_flag_tokens<'a, I>(
    set: &mut HashSet<String>,
    tokens: I,
    kind: &str,
    source: &str,
) -> Result<(), ArchetectError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut removals = Vec::new();
    for token in tokens {
        let directive = parse_flag_token(token, kind, source)?;
        if directive.enabled {
            set.insert(directive.name);
        } else {
            removals.push(directive.name);
        }
    }
    for name in removals {
        set.remove(&name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse_bare_name_enables() {
        let d = parse_flag_token("github", "switch", "test").unwrap();
        assert_eq!(d, FlagDirective { name: "github".into(), enabled: true });
    }

    #[test]
    fn test_parse_explicit_true() {
        let d = parse_flag_token("github=true", "switch", "test").unwrap();
        assert!(d.enabled);
    }

    #[test]
    fn test_parse_false_disables() {
        let d = parse_flag_token("github=false", "switch", "test").unwrap();
        assert_eq!(d, FlagDirective { name: "github".into(), enabled: false });
    }

    #[test]
    fn test_parse_rejects_other_values() {
        let err = parse_flag_token("github=maybe", "switch", "cli").unwrap_err();
        assert!(err.to_string().contains("github=maybe"));
        assert!(err.to_string().contains("cli"));
    }

    #[test]
    fn test_parse_rejects_empty_name() {
        assert!(parse_flag_token("=true", "switch", "test").is_err());
        assert!(parse_flag_token("", "switch", "test").is_err());
    }

    #[test]
    fn test_overlay_adds_and_removes() {
        let mut s = set(&["github", "docker"]);
        overlay_flag_tokens(&mut s, ["postgres", "github=false"], "switch", "test").unwrap();
        assert_eq!(s, set(&["docker", "postgres"]));
    }

    #[test]
    fn test_overlay_untouched_items_survive() {
        let mut s = set(&["a", "b"]);
        overlay_flag_tokens(&mut s, ["c"], "switch", "test").unwrap();
        assert_eq!(s, set(&["a", "b", "c"]));
    }

    #[test]
    fn test_removals_apply_after_additions_within_layer() {
        // Order inside one layer must not matter.
        let mut s1 = HashSet::new();
        overlay_flag_tokens(&mut s1, ["x", "x=false"], "switch", "test").unwrap();
        let mut s2 = HashSet::new();
        overlay_flag_tokens(&mut s2, ["x=false", "x"], "switch", "test").unwrap();
        assert!(s1.is_empty());
        assert!(s2.is_empty());
    }

    #[test]
    fn test_later_layer_wins() {
        let mut s = HashSet::new();
        overlay_flag_tokens(&mut s, ["github=false"], "switch", "layer1").unwrap();
        overlay_flag_tokens(&mut s, ["github"], "switch", "layer2").unwrap();
        assert_eq!(s, set(&["github"]));
    }

    #[test]
    fn test_remove_absent_is_noop() {
        let mut s = HashSet::new();
        overlay_flag_tokens(&mut s, ["ghost=false"], "switch", "test").unwrap();
        assert!(s.is_empty());
    }
}

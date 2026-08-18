//! Segments — the author's grouping intent, as messages.
//!
//! A **page** and a **section** are the same container with a different
//! `kind`; archetect carries the distinction rather than deciding what
//! either looks like. A wizard turns pages into steps and sections into
//! fieldsets; a terminal turns both into headings.
//!
//! They travel as `ScriptMessage::BeginSegment` / `EndSegment`, which is
//! what makes one declaration serve every mode: the probe folds them into
//! a tree for form generation, and an interactive driver prints them as
//! it passes through. Neither expects a reply.
//!
//! See `docs/plans/interface-pages-and-sections.md`.

use serde::{Deserialize, Serialize};

/// Which kind of container this is. Renderers distinguish; archetect does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// A top-level break — a wizard step, a screen.
    Page,
    /// A subordinate grouping — a fieldset within a step.
    Section,
}

impl SegmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SegmentKind::Page => "page",
            SegmentKind::Section => "section",
        }
    }
}

impl std::fmt::Display for SegmentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A container being entered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentInfo {
    pub kind: SegmentKind,
    /// Stable identity a client routes on. Derived from the title unless
    /// the author pinned one — titles get reworded, keys should not.
    pub key: String,
    /// Display text. Required: a container a UI cannot label is not renderable.
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Opaque author-supplied UI metadata, passed through untouched — the
    /// same contract prompts already have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<serde_json::Value>,
}

/// A container being left. Self-describing so a client can close the right
/// one without tracking the open stack itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentEnd {
    pub kind: SegmentKind,
    pub key: String,
}

/// One rung of the breadcrumb a prompt carries: the containers it was asked
/// inside, outermost first. This is the interactive half of the feature —
/// a client receiving one envelope at a time still knows where it is.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentRef {
    pub kind: SegmentKind,
    pub key: String,
    pub title: String,
}

impl From<&SegmentInfo> for SegmentRef {
    fn from(info: &SegmentInfo) -> Self {
        SegmentRef {
            kind: info.kind,
            key: info.key.clone(),
            title: info.title.clone(),
        }
    }
}

/// Derive a segment key from its title: lowercased, every run of
/// non-alphanumeric characters collapsed to `_`, ends trimmed.
/// `"Service Identity"` → `"service_identity"`.
pub fn segment_key_from_title(title: &str) -> String {
    let mut key = String::with_capacity(title.len());
    let mut pending_separator = false;
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            if pending_separator && !key.is_empty() {
                key.push('_');
            }
            pending_separator = false;
            key.extend(ch.to_lowercase());
        } else {
            pending_separator = true;
        }
    }
    if key.is_empty() {
        // A title of pure punctuation is legal but unroutable; give the
        // client something stable rather than an empty key.
        key.push_str("segment");
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_snake_slug_from_a_title() {
        assert_eq!(segment_key_from_title("Service Identity"), "service_identity");
        assert_eq!(segment_key_from_title("Review"), "review");
        assert_eq!(segment_key_from_title("  Leading & trailing  "), "leading_trailing");
        assert_eq!(segment_key_from_title("CI/CD"), "ci_cd");
    }

    #[test]
    fn an_unsluggable_title_still_yields_a_key() {
        assert_eq!(segment_key_from_title("!!!"), "segment");
        assert_eq!(segment_key_from_title(""), "segment");
    }
}

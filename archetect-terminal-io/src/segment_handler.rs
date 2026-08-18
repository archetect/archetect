//! Rendering a page or a section on a terminal.
//!
//! The CLI has always had, in miniature, the problem the interface work hit
//! at scale: a long prompt sequence with no sense of place. Archetypes have
//! been faking it with banner lines. Because containers are IO messages, the
//! same declaration that paginates a wizard heads a terminal run for free.
//!
//! Headings go to **stderr**, alongside the prompts they introduce and away
//! from anything a caller pipes.

use archetect_api::{SegmentInfo, SegmentKind};

/// Announce a container being entered. A page gets a rule; a section gets a
/// quieter marker — the same distinction a wizard makes between a step and a
/// fieldset, in the vocabulary a terminal has.
pub fn handle_begin_segment(info: &SegmentInfo) {
    eprintln!();
    match info.kind {
        SegmentKind::Page => {
            let widest = info
                .help
                .as_deref()
                .map(|help| help.chars().count())
                .unwrap_or(0)
                .max(info.title.chars().count());
            let rule = "─".repeat(widest + 4);
            eprintln!("┌{}", rule);
            eprintln!("│  {}", info.title);
            if let Some(help) = info.help.as_deref() {
                eprintln!("│  {}", help);
            }
            eprintln!("└{}", rule);
        }
        SegmentKind::Section => {
            eprintln!("  ┄ {}", info.title);
            if let Some(help) = info.help.as_deref() {
                eprintln!("    {}", help);
            }
        }
    }
    eprintln!();
}

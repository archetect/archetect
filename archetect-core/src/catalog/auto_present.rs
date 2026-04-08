use archetect_api::ContextValue;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::catalog::dispatch;
use crate::errors::ArchetectError;

/// Auto-present catalog entries as an interactive select menu.
///
/// This is the runtime path for archetypes that have a `catalog` field but no script.
/// Delegates to the shared catalog dispatch module.
pub fn auto_present_catalog(
    archetype: &Archetype,
    render_context: RenderContext,
) -> Result<ContextValue, ArchetectError> {
    let catalog = archetype.manifest().catalog().ok_or_else(|| {
        ArchetectError::GeneralError("No catalog entries found in manifest".to_string())
    })?;

    dispatch::present_entries(archetype.archetect(), catalog, &render_context)?;
    Ok(ContextValue::Nil)
}

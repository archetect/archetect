use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_catalog_render_returns_child_context_with_value_semantics() -> Result<(), ArchetectError> {
    // The parent calls catalog.render("org-prompts", parent_local) twice.
    // The component sets a key on its own context and returns it.
    // - The returned context has the key the component set.
    // - The parent's original context is NOT mutated by the call.
    // - The replace pattern (assign-back) gives the parent a context that
    //   contains BOTH the parent's original keys AND the component's
    //   contributions, because the child started from a copy of the parent.
    let harness = TestHarnessBuilder::new(file!()).build()?;

    assert_eq!(harness.expect_print(), "child returned: yes");
    assert_eq!(harness.expect_print(), "parent isolated");
    assert_eq!(harness.expect_print(), "merged has both");

    assert!(harness.render_succeeded());
    Ok(())
}

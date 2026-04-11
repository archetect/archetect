use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_staged_library_require() -> Result<(), ArchetectError> {
    // The consumer archetype declares `test-lib` as a `library: true`
    // catalog entry pointing at the inline `test-library/` directory.
    // At archetype load, the library's lib/ is symlinked into the
    // consumer's staging dir under the catalog map key, so the consumer
    // script can `require("test-lib.shouter")` and find the file.
    let harness = TestHarnessBuilder::new(file!()).build()?;

    assert_eq!(harness.expect_print(), "HELLO!");

    assert!(harness.render_succeeded());
    Ok(())
}

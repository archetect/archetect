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

    // archetype.mount_key() called from inside a staged library returns
    // the catalog map-key under which the library was mounted —
    // independent of the library's own physical directory name. The
    // consumer chose `test-lib` as the catalog map-key here, so that's
    // what introspection returns.
    assert_eq!(harness.expect_print(), "from-library: test-lib");
    assert_eq!(harness.expect_print(), "from-library is_library: true");

    // Called from the parent's own script (not loaded from any staged
    // library), mount_key returns nil and is_standalone is true.
    assert_eq!(harness.expect_print(), "from-parent: nil");
    assert_eq!(harness.expect_print(), "from-parent is_standalone: true");

    // include_path() — sugar that auto-prefixes the mount key when
    // called from inside a staged library, and returns the path
    // unchanged when called from outside.
    assert_eq!(harness.expect_print(), "lib-include: test-lib/foo.atl");
    assert_eq!(harness.expect_print(), "parent-include: foo.atl");

    assert!(harness.render_succeeded());
    Ok(())
}

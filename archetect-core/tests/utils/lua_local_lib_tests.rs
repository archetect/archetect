use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_local_lib_directory_on_package_path() -> Result<(), ArchetectError> {
    // The consumer's own <root>/lib/ should be on package.path
    // automatically — no manifest declaration needed. The test fixture's
    // archetype.lua does require("greet") and require("nested.util")
    // and prints the results, which we verify here.
    let harness = TestHarnessBuilder::new(file!()).build()?;

    assert_eq!(harness.expect_print(), "hello, world");
    assert_eq!(harness.expect_print(), "HELLO, NESTED!");

    assert!(harness.render_succeeded());
    Ok(())
}

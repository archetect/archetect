use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_self_require_lib_init() -> Result<(), ArchetectError> {
    // A library's own shim should be able to reach `lib/init.lua` as
    // `require("lib")`. This works because the archetype root is on
    // package.path with a `?/init.lua` pattern — Lua substitutes `lib`
    // for `?` and finds the file. See docs/plans/self-requirable-lib.md.
    let harness = TestHarnessBuilder::new(file!()).build()?;

    assert_eq!(harness.expect_print(), "hi from self-lib");

    assert!(harness.render_succeeded());
    Ok(())
}

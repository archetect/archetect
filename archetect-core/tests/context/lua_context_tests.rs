use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_lua_context_operations() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!()).build()?;

    // ctx:set("name", "Alice") then ctx:get("name")
    assert_eq!(harness.expect_log_info(), "Alice");

    // ctx:has("name") -> true
    assert_eq!(harness.expect_log_info(), "true");

    // ctx:has("missing") -> false
    assert_eq!(harness.expect_log_info(), "false");

    // ctx:set("count", 42) then ctx:get("count")
    assert_eq!(harness.expect_log_info(), "42");

    // ctx:set("enabled", true) then ctx:get("enabled")
    assert_eq!(harness.expect_log_info(), "true");

    // ctx:get("nonexistent") -> nil
    assert_eq!(harness.expect_log_info(), "nil");

    assert!(harness.render_succeeded());
    Ok(())
}

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_lua_switches_enabled() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch("feature_a")
        .build()?;

    // feature_a is enabled, feature_b is not
    assert_eq!(harness.expect_log_info(), "feature_a_enabled");
    // feature_c is not enabled, so the "not" branch fires
    assert_eq!(harness.expect_log_info(), "feature_c_disabled");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
fn test_lua_switches_multiple() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch("feature_a")
        .with_switch("feature_b")
        .build()?;

    assert_eq!(harness.expect_log_info(), "feature_a_enabled");
    assert_eq!(harness.expect_log_info(), "feature_b_enabled");
    assert_eq!(harness.expect_log_info(), "feature_c_disabled");

    assert!(harness.render_succeeded());
    Ok(())
}

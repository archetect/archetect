use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_lua_logging() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!()).build()?;

    assert_eq!(harness.expect_log_trace(), "trace message");
    assert_eq!(harness.expect_log_debug(), "debug message");
    assert_eq!(harness.expect_log_info(), "info message");
    assert_eq!(harness.expect_log_warn(), "warn message");
    assert_eq!(harness.expect_log_error(), "error message");
    assert_eq!(harness.expect_print(), "print message");
    assert_eq!(harness.expect_display(), "banner message");

    assert!(harness.render_succeeded());
    Ok(())
}

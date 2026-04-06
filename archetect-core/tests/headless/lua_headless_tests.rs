use function_name::named;

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_headless_with_defaults() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .headless()
        .with_switch(function_name!())
        .build()?;

    // In headless mode with defaults, no prompts are sent
    assert_eq!(harness.expect_log_info(), "DefaultName");
    assert_eq!(harness.expect_log_info(), "8080");
    assert_eq!(harness.expect_log_info(), "true");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_headless_without_defaults() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .headless()
        .with_switch(function_name!())
        .build()?;

    // In headless mode without defaults, the Lua script errors
    let error = harness.expect_log_error();
    assert!(error.contains("Headless") || error.contains("headless") || error.contains("no answer"),
        "Expected headless error message: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

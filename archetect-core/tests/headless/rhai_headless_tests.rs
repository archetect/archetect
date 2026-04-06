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

    // In headless mode with defaults, no prompts are sent — defaults are used directly
    let output = harness.expect_display();
    assert!(output.contains("\"name\": \"DefaultName\""), "Expected DefaultName in output: {}", output);
    assert!(output.contains("\"port\": 8080"), "Expected port 8080 in output: {}", output);
    assert!(output.contains("\"enabled\": true"), "Expected enabled true in output: {}", output);

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

    // In headless mode without defaults, the script should error
    let error = harness.expect_log_error();
    assert!(error.contains("headless") || error.contains("Headless"),
        "Expected headless error message: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

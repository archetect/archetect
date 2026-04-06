use function_name::named;

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_runtime_error() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let error = harness.expect_log_error();
    assert!(!error.is_empty(), "Expected a runtime error message");

    assert!(!harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_undefined_function() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let error = harness.expect_log_error();
    assert!(error.contains("nonexistent_function") || error.contains("not found"),
        "Expected error about undefined function: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

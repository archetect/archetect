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
    assert!(error.contains("intentional error"),
        "Expected 'intentional error' in message: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_nil_index() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let error = harness.expect_log_error();
    assert!(!error.is_empty(), "Expected a runtime error for nil indexing");

    assert!(!harness.render_succeeded());
    Ok(())
}

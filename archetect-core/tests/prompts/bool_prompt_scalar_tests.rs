use function_name::named;

use archetect_api::PromptInfo;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_scalar_bool_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_bool_prompt();
    assert_eq!(prompt_info.message(), "Enabled:");
    assert_eq!(prompt_info.default(), None);
    assert_eq!(prompt_info.help(), None);
    assert_eq!(prompt_info.placeholder(), None);
    assert!(!prompt_info.optional());

    harness.respond_bool(true);

    let output = harness.expect_display();
    assert!(output.contains("\"enabled\": true"), "Expected 'enabled: true' in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_scalar_bool_prompt_with_default() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_bool_prompt();
    assert_eq!(prompt_info.message(), "Verbose:");
    assert_eq!(prompt_info.default(), Some(true));
    assert_eq!(prompt_info.help(), Some("Enable verbose output"));
    assert_eq!(prompt_info.placeholder(), Some("true/false"));

    harness.respond_bool(false);

    let output = harness.expect_display();
    assert!(output.contains("\"verbose\": false"), "Expected 'verbose: false' in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_scalar_bool_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_bool_prompt();
    harness.respond_none();

    let error = harness.expect_log_error();
    assert!(error.contains("not optional"), "Expected 'not optional' in error: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_scalar_bool_prompt_unexpected() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_bool_prompt();
    harness.respond_text("not a bool");

    let error = harness.expect_log_error();
    assert!(error.contains("expects a Boolean"), "Expected 'expects a Boolean' in error: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_scalar_bool_prompt_with_answer() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    // Answer is provided by answer_source, so no prompt expected
    let output = harness.expect_display();
    assert!(output.contains("\"enable_logging\": true"), "Expected 'enable_logging: true' in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

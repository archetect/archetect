use function_name::named;

use archetect_api::{PromptInfo, PromptInfoPageable};
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_select_basic() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_select_prompt();
    assert_eq!(prompt_info.message(), "Language:");
    assert_eq!(prompt_info.options(), &["Rust", "Java", "Go"]);
    assert_eq!(prompt_info.default(), None);
    assert_eq!(prompt_info.help(), None);
    assert!(!prompt_info.optional());
    assert_eq!(prompt_info.page_size(), Some(10));

    harness.respond_text("Rust");

    let output = harness.expect_display();
    assert!(output.contains("\"language\": \"Rust\""), "Expected language: Rust in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_select_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_select_prompt();
    assert_eq!(prompt_info.message(), "Language:");
    assert_eq!(prompt_info.default(), Some("Rust".to_string()));
    assert_eq!(prompt_info.help(), Some("Choose your primary language"));
    assert_eq!(prompt_info.page_size(), Some(5));

    harness.respond_text("Go");

    let output = harness.expect_display();
    assert!(output.contains("\"language\": \"Go\""), "Expected language: Go in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_select_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_select_prompt();
    harness.respond_none();

    let error = harness.expect_log_error();
    assert!(error.contains("not optional"), "Expected 'not optional' in error: {}", error);

    assert!(!harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_select_with_answer() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .with_answer("language", "Java")
        .build()?;

    // Answer is provided by RenderContext, so no prompt expected
    let output = harness.expect_display();
    assert!(output.contains("\"language\": \"Java\""), "Expected language: Java in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

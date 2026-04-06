use function_name::named;

use archetect_api::PromptInfo;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_select_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_select_prompt();
    assert_eq!(prompt_info.message(), "Language:");
    assert_eq!(prompt_info.key(), Some("language"));
    assert_eq!(prompt_info.options(), &["Rust", "Java", "Go"]);
    assert_eq!(prompt_info.default(), None);
    assert!(!prompt_info.optional());

    harness.respond_text("Rust");

    let output = harness.expect_log_info();
    assert_eq!(output, "Rust");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_select_prompt_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_select_prompt();
    assert_eq!(prompt_info.message(), "Language:");
    assert_eq!(prompt_info.default(), Some("Rust".to_string()));
    assert_eq!(prompt_info.help(), Some("Choose your primary language"));

    harness.respond_text("Go");

    let output = harness.expect_log_info();
    assert_eq!(output, "Go");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_select_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_select_prompt();
    harness.respond_none();

    // Lua doesn't store None — ctx:get returns nil
    let output = harness.expect_log_info();
    assert_eq!(output, "nil");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_select_prompt_with_answer() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .with_answer("language", "Java")
        .build()?;

    // Answer is pre-supplied, so no prompt expected
    let output = harness.expect_log_info();
    assert_eq!(output, "Java");

    assert!(harness.render_succeeded());
    Ok(())
}

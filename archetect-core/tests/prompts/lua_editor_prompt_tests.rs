use function_name::named;

use archetect_api::PromptInfo;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_editor_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_editor_prompt();
    assert_eq!(prompt_info.message(), "Description:");
    assert_eq!(prompt_info.key(), Some("description"));
    assert_eq!(prompt_info.default(), None);

    harness.respond_text("A multi-line description");

    let output = harness.expect_log_info();
    assert_eq!(output, "A multi-line description");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_editor_prompt_with_default() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_editor_prompt();
    assert_eq!(prompt_info.message(), "Description:");
    assert_eq!(prompt_info.default(), Some("Default description".to_string()));
    assert_eq!(prompt_info.help(), Some("Enter a description"));

    harness.respond_text("Custom description");

    let output = harness.expect_log_info();
    assert_eq!(output, "Custom description");

    assert!(harness.render_succeeded());
    Ok(())
}

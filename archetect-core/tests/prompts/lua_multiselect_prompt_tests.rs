use function_name::named;

use archetect_api::{PromptInfo, PromptInfoItemsRestrictions};
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_multiselect_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_multiselect_prompt();
    assert_eq!(prompt_info.message(), "Languages:");
    assert_eq!(prompt_info.key(), Some("languages"));
    assert_eq!(prompt_info.options(), &["Rust", "Java", "Go"]);
    assert!(!prompt_info.optional());

    harness.respond_array(vec!["Rust", "Go"]);

    assert_eq!(harness.expect_log_info(), "Rust");
    assert_eq!(harness.expect_log_info(), "Go");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_multiselect_prompt_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_multiselect_prompt();
    assert_eq!(prompt_info.message(), "Languages:");
    assert_eq!(prompt_info.help(), Some("Select your languages"));
    assert_eq!(prompt_info.min_items(), Some(1));
    assert_eq!(prompt_info.max_items(), Some(2));

    harness.respond_array(vec!["Java"]);

    assert_eq!(harness.expect_log_info(), "Java");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_multiselect_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_multiselect_prompt();
    harness.respond_none();

    // Lua doesn't store None — no items logged
    assert!(harness.render_succeeded());
    Ok(())
}

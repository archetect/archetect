use function_name::named;

use archetect_api::{PromptInfo, PromptInfoItemsRestrictions};
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_list_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_list_prompt();
    assert_eq!(prompt_info.message(), "Dependencies:");
    assert_eq!(prompt_info.key(), Some("dependencies"));
    assert!(!prompt_info.optional());

    harness.respond_array(vec!["serde", "tokio"]);

    assert_eq!(harness.expect_log_info(), "serde");
    assert_eq!(harness.expect_log_info(), "tokio");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_list_prompt_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_list_prompt();
    assert_eq!(prompt_info.message(), "Dependencies:");
    assert_eq!(prompt_info.help(), Some("Enter dependencies one at a time"));
    assert_eq!(prompt_info.min_items(), Some(1));
    assert_eq!(prompt_info.max_items(), Some(5));

    harness.respond_array(vec!["clap", "anyhow", "tracing"]);

    assert_eq!(harness.expect_log_info(), "clap");
    assert_eq!(harness.expect_log_info(), "anyhow");
    assert_eq!(harness.expect_log_info(), "tracing");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_list_prompt_with_default() -> Result<(), ArchetectError> {
    // In headless mode with a default list provided in opts, no prompt
    // is sent — the default is applied directly and stored in context.
    let harness = TestHarnessBuilder::new(file!())
        .headless()
        .with_switch(function_name!())
        .build()?;

    assert_eq!(harness.expect_log_info(), "serde");
    assert_eq!(harness.expect_log_info(), "tokio");

    assert!(harness.render_succeeded());
    Ok(())
}

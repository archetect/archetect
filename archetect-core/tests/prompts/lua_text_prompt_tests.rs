use function_name::named;

use archetect_api::{PromptInfo, PromptInfoLengthRestrictions};
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_text_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_text_prompt();
    assert_eq!(prompt_info.message(), "Service Name:");
    assert_eq!(prompt_info.key(), Some("service_name"));
    assert_eq!(prompt_info.default(), None);
    assert!(!prompt_info.optional());

    harness.respond_text("CustomerService");

    let output = harness.expect_log_info();
    assert_eq!(output, "CustomerService");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_text_prompt_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_text_prompt();
    assert_eq!(prompt_info.message(), "Service Name:");
    assert_eq!(prompt_info.default(), Some("MyService".to_string()));
    assert_eq!(prompt_info.min(), Some(2));
    assert_eq!(prompt_info.max(), Some(20));
    assert_eq!(prompt_info.help(), Some("Enter a service name"));
    assert_eq!(prompt_info.placeholder(), Some("ServiceName"));

    harness.respond_text("OrderService");

    let output = harness.expect_log_info();
    assert_eq!(output, "OrderService");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_text_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_text_prompt();
    harness.respond_none();

    // Lua stores None as no entry — script continues without storing
    // The log.info(tostring(ctx:get("service_name"))) will log "nil"
    let output = harness.expect_log_info();
    assert_eq!(output, "nil");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_text_prompt_with_answer() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .with_answer("service_name", "PreSupplied")
        .build()?;

    // Answer is pre-supplied, so no prompt expected
    let output = harness.expect_log_info();
    assert_eq!(output, "PreSupplied");

    assert!(harness.render_succeeded());
    Ok(())
}

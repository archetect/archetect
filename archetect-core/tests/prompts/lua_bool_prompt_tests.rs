use function_name::named;

use archetect_api::PromptInfo;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_bool_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_bool_prompt();
    assert_eq!(prompt_info.message(), "Enable logging:");
    assert_eq!(prompt_info.key(), Some("enable_logging"));
    assert_eq!(prompt_info.default(), None);
    assert!(!prompt_info.optional());

    harness.respond_bool(true);

    let output = harness.expect_log_info();
    assert_eq!(output, "true");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_bool_prompt_with_default() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_bool_prompt();
    assert_eq!(prompt_info.message(), "Verbose:");
    assert_eq!(prompt_info.default(), Some(true));
    assert_eq!(prompt_info.help(), Some("Enable verbose output"));

    harness.respond_bool(false);

    let output = harness.expect_log_info();
    assert_eq!(output, "false");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_bool_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_bool_prompt();
    harness.respond_none();

    // Lua doesn't store None — ctx:get returns nil
    let output = harness.expect_log_info();
    assert_eq!(output, "nil");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_bool_prompt_with_answer() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .with_answer("enable_logging", true)
        .build()?;

    // Answer is pre-supplied, so no prompt expected
    let output = harness.expect_log_info();
    assert_eq!(output, "true");

    assert!(harness.render_succeeded());
    Ok(())
}

use function_name::named;

use archetect_api::{PromptInfo, PromptInfoLengthRestrictions};
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_int_prompt() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_int_prompt();
    assert_eq!(prompt_info.message(), "Port:");
    assert_eq!(prompt_info.key(), Some("port"));
    assert_eq!(prompt_info.default(), None);
    assert!(!prompt_info.optional());

    harness.respond_int(8080);

    let output = harness.expect_log_info();
    assert_eq!(output, "8080");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_int_prompt_with_options() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let prompt_info = harness.expect_int_prompt();
    assert_eq!(prompt_info.message(), "Port:");
    assert_eq!(prompt_info.default(), Some(8080));
    assert_eq!(prompt_info.min(), Some(1024));
    assert_eq!(prompt_info.max(), Some(65535));
    assert_eq!(prompt_info.help(), Some("Enter a port number"));
    assert_eq!(prompt_info.placeholder(), Some("8080"));

    harness.respond_int(3000);

    let output = harness.expect_log_info();
    assert_eq!(output, "3000");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_int_prompt_non_optional() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let _ = harness.expect_int_prompt();
    harness.respond_none();

    // Lua doesn't store None — ctx:get returns nil
    let output = harness.expect_log_info();
    assert_eq!(output, "nil");

    assert!(harness.render_succeeded());
    Ok(())
}

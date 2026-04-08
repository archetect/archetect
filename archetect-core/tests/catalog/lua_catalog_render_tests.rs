use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

/// Test that catalog.render() resolves the path, applies pre-configured answers,
/// and renders the child archetype without prompting.
#[test]
fn test_catalog_render_with_pre_answers() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!()).build()?;

    // The child archetype prompts for service_name, but the catalog entry
    // pre-configures it as "grpc-service" via answers. So we should get
    // the write events without needing to respond to a prompt.

    // Expect the directory write for the child archetype's output
    let _dir = harness.expect_write_directory();

    // Expect the rendered service.txt file
    let file_info = harness.expect_write_file();
    let contents = String::from_utf8(file_info.contents).expect("Valid UTF-8");
    assert!(
        contents.contains("Service: grpc-service"),
        "Expected pre-answered service name, got: {}",
        contents
    );

    // Expect the log message confirming catalog.render() returned
    let msg = harness.expect_log_info();
    assert_eq!(msg, "catalog render completed");

    assert!(harness.render_succeeded());
    Ok(())
}

/// Test that catalog.render() works for entries without pre-answers,
/// falling through to interactive prompting.
#[test]
fn test_catalog_render_interactive() -> Result<(), ArchetectError> {
    // Construct path in the same format file!() would produce for a sibling test file
    let test_path = file!().replace("lua_catalog_render_tests.rs", "lua_catalog_render_interactive_tests.rs");
    let harness = TestHarnessBuilder::new(&test_path).build()?;

    // The "rest" entry has no pre-configured answers, so the child
    // archetype will prompt for service_name interactively.
    let prompt = harness.expect_text_prompt();
    assert_eq!(prompt.message, "Service Name:");
    harness.respond_text("my-rest-api");

    let _dir = harness.expect_write_directory();

    let file_info = harness.expect_write_file();
    let contents = String::from_utf8(file_info.contents).expect("Valid UTF-8");
    assert!(
        contents.contains("Service: my-rest-api"),
        "Expected interactively-provided name, got: {}",
        contents
    );

    let msg = harness.expect_log_info();
    assert_eq!(msg, "interactive catalog render completed");

    assert!(harness.render_succeeded());
    Ok(())
}

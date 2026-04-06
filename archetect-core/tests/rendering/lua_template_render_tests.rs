use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_lua_template_render() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!()).build()?;

    let _ = harness.expect_text_prompt();
    harness.respond_text("World");

    let output = harness.expect_log_info();
    assert_eq!(output, "Hello, World!");

    assert!(harness.render_succeeded());
    Ok(())
}

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_rhai_render_directory() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!()).build()?;

    // Respond to the project name prompt
    let _ = harness.expect_text_prompt();
    harness.respond_text("MyProject");

    // Expect the destination directory to be created
    let dir_path = harness.expect_write_directory();
    assert!(dir_path.is_empty() || dir_path == "", "Expected root destination, got: {}", dir_path);

    // Expect the rendered README.md file
    let file_info = harness.expect_write_file();
    assert!(file_info.destination.contains("README.md"), "Expected README.md, got: {}", file_info.destination);

    let contents = String::from_utf8(file_info.contents).expect("Valid UTF-8");
    assert!(contents.contains("# MyProject"), "Expected rendered project name in contents: {}", contents);
    assert!(contents.contains("Welcome to MyProject"), "Expected welcome text in contents: {}", contents);

    assert!(harness.render_succeeded());
    Ok(())
}

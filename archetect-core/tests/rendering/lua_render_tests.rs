use archetect_core::errors::ArchetectError;
use camino::Utf8PathBuf;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_lua_render_directory() -> Result<(), ArchetectError> {
    // Pin the destination explicitly — `RenderContext::new` absolutizes
    // any relative/empty input to the process CWD, which would make the
    // emitted write-directory path depend on where `cargo test` is run.
    let dest = Utf8PathBuf::from("/tmp/archetect-test-lua-render-directory");
    let harness = TestHarnessBuilder::new(file!())
        .with_destination(dest.clone())
        .build()?;

    // Respond to the project name prompt
    let _ = harness.expect_text_prompt();
    harness.respond_text("MyProject");

    // Expect the destination directory to be created (at the root — not
    // a nested subdir — hence matching the destination exactly).
    let dir_path = harness.expect_write_directory();
    assert_eq!(dir_path, dest.as_str(), "Expected root destination, got: {}", dir_path);

    // Expect the rendered README.md file
    let file_info = harness.expect_write_file();
    assert!(file_info.destination.contains("README.md"), "Expected README.md, got: {}", file_info.destination);

    let contents = String::from_utf8(file_info.contents).expect("Valid UTF-8");
    assert!(contents.contains("# MyProject"), "Expected rendered project name in contents: {}", contents);
    assert!(contents.contains("Welcome to MyProject"), "Expected welcome text in contents: {}", contents);

    assert!(harness.render_succeeded());
    Ok(())
}

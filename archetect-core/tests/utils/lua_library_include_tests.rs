use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_library_include_resolves_through_staging() -> Result<(), ArchetectError> {
    // The consumer declares `test-lib` as a library: true catalog entry.
    // At load time, the LibraryStager symlinks the library's includes/
    // directory under <staging>/includes/test-lib/. The IncludeResolver
    // search list contains <staging>/includes/, so the consumer's
    // template can do `{% include "test-lib/banner.atl" %}` and it
    // resolves through the namespace prefix.
    let harness = TestHarnessBuilder::new(file!()).build()?;

    let _dir = harness.expect_write_directory();
    let file_info = harness.expect_write_file();
    let contents = String::from_utf8(file_info.contents).expect("Valid UTF-8");

    // The banner came from the staged library's includes/banner.atl,
    // and the project_name interpolation was resolved by the consumer's
    // own context. Both pieces working confirms the round-trip.
    assert!(
        contents.contains("=== smoke-test ==="),
        "expected library banner with interpolated name, got: {}",
        contents
    );
    assert!(
        contents.contains("project: smoke-test"),
        "expected consumer's own template content, got: {}",
        contents
    );

    assert!(harness.render_succeeded());
    Ok(())
}

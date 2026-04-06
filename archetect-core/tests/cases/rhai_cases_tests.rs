use function_name::named;

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_single_case_style() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    // With a single CaseStyle (SnakeCase), the value is cased but the key stays the same
    let output = harness.expect_display();
    assert!(output.contains("\"project_name\": \"my_cool_project\""),
        "Expected snake_cased value in output: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

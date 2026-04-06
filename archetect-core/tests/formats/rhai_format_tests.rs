use function_name::named;

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_as_json() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let output = harness.expect_display();
    assert!(output.contains("\"name\"") && output.contains("Alice"),
        "Expected JSON with name Alice: {}", output);
    assert!(output.contains("30"), "Expected age 30 in JSON: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_as_yaml() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let output = harness.expect_display();
    assert!(output.contains("name") && output.contains("Alice"),
        "Expected YAML with name Alice: {}", output);

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_from_json() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let name = harness.expect_display();
    assert_eq!(name, "Bob");

    let age = harness.expect_display();
    assert_eq!(age, "25");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_from_yaml() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    let name = harness.expect_display();
    assert_eq!(name, "Charlie");

    let age = harness.expect_display();
    assert_eq!(age, "35");

    assert!(harness.render_succeeded());
    Ok(())
}

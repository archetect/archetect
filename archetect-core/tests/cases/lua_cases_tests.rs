use function_name::named;

use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
#[named]
fn test_lua_cases_programming() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    // Snake case overwrites original key since snake_case("project_name") == "project_name"
    assert_eq!(harness.expect_log_info(), "my_cool_project");

    // Pascal case: key=ProjectName, value=MyCoolProject
    assert_eq!(harness.expect_log_info(), "MyCoolProject");

    // Camel case: key=projectName, value=myCoolProject
    assert_eq!(harness.expect_log_info(), "myCoolProject");

    // Kebab case: key=project-name, value=my-cool-project
    assert_eq!(harness.expect_log_info(), "my-cool-project");

    // Train case: key=Project-Name, value=My-Cool-Project
    assert_eq!(harness.expect_log_info(), "My-Cool-Project");

    // Constant case: key=PROJECT_NAME, value=MY_COOL_PROJECT
    assert_eq!(harness.expect_log_info(), "MY_COOL_PROJECT");

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_cases_enum_set() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    // Cases.set(Case.Snake, Case.Kebab, Case.Constant)
    assert_eq!(harness.expect_log_info(), "my_app");       // snake
    assert_eq!(harness.expect_log_info(), "my-app");        // kebab
    assert_eq!(harness.expect_log_info(), "MY_APP");        // constant

    assert!(harness.render_succeeded());
    Ok(())
}

#[test]
#[named]
fn test_lua_cases_enum_fixed() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .with_switch(function_name!())
        .build()?;

    // Original value stored as-is
    assert_eq!(harness.expect_log_info(), "Hello World");

    // Cases.fixed("display_name", Case.Title) → title-cased value at fixed key
    assert_eq!(harness.expect_log_info(), "Hello World");

    assert!(harness.render_succeeded());
    Ok(())
}

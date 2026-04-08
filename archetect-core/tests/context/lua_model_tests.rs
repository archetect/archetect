use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarnessBuilder;

#[test]
fn test_model_builder_api() -> Result<(), ArchetectError> {
    let harness = TestHarnessBuilder::new(file!())
        .headless()
        .build()?;

    // Verify the log output from the archetype
    assert_eq!(harness.expect_log_info(), "org=test-org");
    assert_eq!(harness.expect_log_info(), "sol=test-sol");
    assert_eq!(harness.expect_log_info(), "org_solution.kebab=test-org-test-sol");
    assert_eq!(harness.expect_log_info(), "org_solution.pascal=TestOrgTestSol");
    assert_eq!(harness.expect_log_info(), "order-service entities=1");
    assert_eq!(harness.expect_log_info(), "entity_name=Order");
    assert_eq!(harness.expect_log_info(), "slice.boundary=widget-service");
    assert_eq!(harness.expect_log_info(), "slice.entities=1");
    assert_eq!(harness.expect_log_info(), "deps=1");
    assert_eq!(harness.expect_log_info(), "dep=widget-service");
    assert_eq!(harness.expect_log_info(), "remote_refs=1");
    assert_eq!(harness.expect_log_info(), "ref_target=Widget");

    assert!(harness.render_succeeded());
    Ok(())
}

use assert_matches::assert_matches;
use camino::Utf8Path;
use uuid::Uuid;

use archetect_api::{api_driver_and_handle, CommandRequest};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use archetect_core::system::RootedSystemLayout;

#[test]
fn test_utils() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let (driver, handle) = api_driver_and_handle();
    let destination = tempfile::TempDir::new()?;
    let runtime_context = RuntimeContext::new(&configuration, driver, RootedSystemLayout::temp().unwrap());
    let archetype = runtime_context.new_archetype("tests/archetypes/scripting/utils")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8Path::from_path(destination.as_ref()).unwrap(), Default::default())
            .with_switch("build")
            ;
        archetype.render(runtime_context, render_context).unwrap();
    });


    // Commands coming from utils/
    assert_matches!(handle.request(), CommandRequest::LogTrace(message) => {
        assert_eq!(message, "Trace Level");
    });

    assert_matches!(handle.request(), CommandRequest::LogDebug(message) => {
        assert_eq!(message, "Debug Level");
    });

    assert_matches!(handle.request(), CommandRequest::LogInfo(message) => {
        assert_eq!(message, "Info Level");
    });

    assert_matches!(handle.request(), CommandRequest::LogWarn(message) => {
        assert_eq!(message, "Warn Level");
    });

    assert_matches!(handle.request(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Error Level");
    });

    assert_matches!(handle.request(), CommandRequest::Display(message) => {
        assert_eq!(message, "Display Message".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Display(message) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Print(message) => {
        assert_eq!(message, "Print Message".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Print(message) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Display(message) => {
        assert_eq!(message, "13:1 | \"Debug Message\"".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Display(message) => {
        assert_eq!(message, "14:1 | [\"Debug\", \"Message\"]".to_string());
    });

    assert_matches!(handle.request(), CommandRequest::Print(uuid) => {
        use std::str::FromStr;
        assert!(Uuid::from_str(&uuid).is_ok());
    });

    assert_matches!(handle.request(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(handle.request(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from utils_child1/
    assert_matches!(handle.request(), CommandRequest::Print(child_message) => {
        assert_eq!(child_message, "Hello Child!");
    });

    assert_matches!(handle.request(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "false");
    });

    assert_matches!(handle.request(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from utils_child2/
    assert_matches!(handle.request(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(handle.request(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    assert_matches!(handle.request(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "true");
    });

    Ok(())
}
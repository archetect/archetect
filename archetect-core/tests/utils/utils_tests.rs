use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use uuid::Uuid;

use archetect_api::{api_driver_and_handle, CommandRequest};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;

#[test]
fn test_utils() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/utils/utils_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());

        archetype.render(runtime_context, render_context).unwrap();
    });

    // Commands coming from utils/
    assert_matches!(handle.receive(), CommandRequest::LogTrace(message) => {
        assert_eq!(message, "Trace Level");
    });

    assert_matches!(handle.receive(), CommandRequest::LogDebug(message) => {
        assert_eq!(message, "Debug Level");
    });

    assert_matches!(handle.receive(), CommandRequest::LogInfo(message) => {
        assert_eq!(message, "Info Level");
    });

    assert_matches!(handle.receive(), CommandRequest::LogWarn(message) => {
        assert_eq!(message, "Warn Level");
    });

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Error Level");
    });

    assert_matches!(handle.receive(), CommandRequest::Display(message) => {
        assert_eq!(message, "Display Message".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Display(message) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, "Print Message".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Display(message) => {
        assert_eq!(message, "13:1 | \"Debug Message\": tests/utils/utils_tests/archetype.rhai".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Display(message) => {
        assert_eq!(message, "14:1 | [\"Debug\", \"Message\"]: tests/utils/utils_tests/archetype.rhai".to_string());
    });

    assert_matches!(handle.receive(), CommandRequest::Print(uuid) => {
        use std::str::FromStr;
        assert!(Uuid::from_str(&uuid).is_ok());
    });

    Ok(())
}

#[test]
fn test_switches() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/utils/switches")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default()).with_switch("build");
        archetype.render(runtime_context, render_context).unwrap();
    });

    assert_matches!(handle.receive(), CommandRequest::Display(message) => {
        assert_eq!(message, "1:1 | [\"build\"]: tests/utils/switches/archetype.rhai");
    });

    assert_matches!(handle.receive(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(handle.receive(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from switches_child1/
    assert_matches!(handle.receive(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "false");
    });

    assert_matches!(handle.receive(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from switches_child2/
    assert_matches!(handle.receive(), CommandRequest::Print(build_switch_enabled) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(handle.receive(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "false");
    });

    assert_matches!(handle.receive(), CommandRequest::Print(test_switch_enabled) => {
        assert_eq!(test_switch_enabled, "true");
    });

    Ok(())
}
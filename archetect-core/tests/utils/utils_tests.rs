use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use uuid::Uuid;

use archetect_api::{ClientIoHandle, ScriptMessage, SyncIoDriver};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;
use archetect_core::Archetect;

#[test]
fn test_utils() -> Result<(), ArchetectError> {
    let (script_handle, client_handle) = SyncIoDriver::new().split();

    let archetect = Archetect::builder()
        .with_driver(script_handle)
        .with_temp_layout()?
        .build()?;
    let archetype = archetect.new_archetype("tests/utils/utils_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());

        assert!(archetype.render(render_context).is_ok());
    });

    // Commands coming from utils/
    assert_matches!(client_handle.receive(), Some(ScriptMessage::LogTrace(message)) => {
        assert_eq!(message, "Trace Level");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::LogDebug(message)) => {
        assert_eq!(message, "Debug Level");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::LogInfo(message)) => {
        assert_eq!(message, "Info Level");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::LogWarn(message)) => {
        assert_eq!(message, "Warn Level");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::LogError(message)) => {
        assert_eq!(message, "Error Level");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Display(message)) => {
        assert_eq!(message, "Display Message".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Display(message)) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(message)) => {
        assert_eq!(message, "Print Message".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(message)) => {
        assert_eq!(message, "".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Display(message)) => {
        assert_eq!(message, "13:1 | \"Debug Message\": tests/utils/utils_tests/archetype.rhai".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Display(message)) => {
        assert_eq!(message, "14:1 | [\"Debug\", \"Message\"]: tests/utils/utils_tests/archetype.rhai".to_string());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(uuid)) => {
        use std::str::FromStr;
        assert!(Uuid::from_str(&uuid).is_ok());
    });

    Ok(())
}

#[test]
fn test_switches() -> Result<(), ArchetectError> {
    let (script_handle, client_handle) = SyncIoDriver::new().split();
    let archetect = Archetect::builder()
        .with_driver(script_handle)
        .with_temp_layout()?
        .build()?;
    let archetype = archetect.new_archetype("tests/utils/switches")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default()).with_switch("build");
        assert!(archetype.render(render_context).is_ok());
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Display(message)) => {
        assert_eq!(message, "1:1 | [\"build\"]: tests/utils/switches/archetype.rhai");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(build_switch_enabled)) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(test_switch_enabled)) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from switches_child1/
    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(build_switch_enabled)) => {
        assert_eq!(build_switch_enabled, "false");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(test_switch_enabled)) => {
        assert_eq!(test_switch_enabled, "false");
    });

    // Commands coming from switches_child2/
    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(build_switch_enabled)) => {
        assert_eq!(build_switch_enabled, "true");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(test_switch_enabled)) => {
        assert_eq!(test_switch_enabled, "false");
    });

    assert_matches!(client_handle.receive(), Some(ScriptMessage::Print(test_switch_enabled)) => {
        assert_eq!(test_switch_enabled, "true");
    });

    Ok(())
}

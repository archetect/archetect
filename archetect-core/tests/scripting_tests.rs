use assert_matches::assert_matches;
use camino::Utf8Path;
use uuid::Uuid;

use archetect_api::{api_driver_and_handle, CommandRequest};
use archetect_core::Archetect;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use archetect_core::source::Source;
use archetect_core::system::LayoutType;

#[test]
fn test_utils() -> Result<(), ArchetectError> {
    let archetect = Archetect::builder()
        .with_layout_type(LayoutType::Temp)?
        .build()?;

    let configuration = Configuration::default();

    let (driver, handle) = api_driver_and_handle();
    let destination = tempfile::TempDir::new()?;
    let context = RuntimeContext::new(&configuration, driver);

    let source = Source::detect(&archetect, &context, "tests/archetypes/scripting/utils")?;

    let archetype = Archetype::new(&source)?;
    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8Path::from_path(destination.as_ref()).unwrap(), Default::default());
        archetype.render(context, render_context).unwrap();
    });

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

    Ok(())

}
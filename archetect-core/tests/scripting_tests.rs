use std::collections::HashSet;

use assert_matches::assert_matches;
use camino::Utf8PathBuf;

use archetect_api::{api_driver_and_handle, CommandRequest};
use archetect_core::Archetect;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use archetect_core::source::Source;
use archetect_core::system::LayoutType;

#[test]
fn test_log() -> Result<(), ArchetectError> {
    let archetect = Archetect::builder()
        .with_layout_type(LayoutType::Temp)?
        .build()?;

    let configuration = Configuration::default();

    let (driver, handle) = api_driver_and_handle();
    let destination = tempfile::TempDir::new()?;
    let context = RuntimeContext::new(&configuration, HashSet::new(), Utf8PathBuf::from_path_buf(destination.into_path()).unwrap(), driver);

    let source = Source::detect(&archetect, &context, "tests/archetypes/scripting/outputs")?;

    let archetype = Archetype::new(&source)?;
    std::thread::spawn(move || {
        archetype.render(context, Default::default()).unwrap();

    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::LogTrace(message) => {
        assert_eq!(message, "Trace Level");
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::LogDebug(message) => {
        assert_eq!(message, "Debug Level");
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::LogInfo(message) => {
        assert_eq!(message, "Info Level");
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::LogWarn(message) => {
        assert_eq!(message, "Warn Level");
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Error Level");
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::EPrint(message) => {
        assert_eq!(message, Some("Display Message".to_string()));
    });

    assert_matches!(handle.requests().recv().unwrap(), CommandRequest::EPrint(message) => {
        assert_eq!(message, None);
    });

    Ok(())

}
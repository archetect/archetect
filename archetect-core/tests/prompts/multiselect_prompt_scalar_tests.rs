use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use function_name::named;
use archetect_api::{ScriptMessage, ClientMessage, PromptInfo, PromptInfoItemsRestrictions, PromptInfoPageable};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use crate::test_utils::TestHarness;
use rhai::Map;

#[test]
#[named]
fn test_scalar_defaults() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), ScriptMessage::PromptForMultiSelect(prompt_info) => {
        assert_eq!(prompt_info.message(), "Languages:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), None);
        assert_matches!(prompt_info.optional(), false);
        assert_matches!(prompt_info.page_size(), Some(10));
    });

    harness.respond(ClientMessage::Array(vec!["Rust".into(), "Java".into()]));

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "11:5 | #{\"languages\": [\"Rust\", \"Java\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn test_map_defaults() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), ScriptMessage::PromptForMultiSelect(prompt_info) => {
        assert_eq!(prompt_info.message(), "Languages:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), None);
        assert_matches!(prompt_info.optional(), false);
        assert_matches!(prompt_info.page_size(), Some(10));
    });

    harness.respond(ClientMessage::Array(vec!["Rust".into(), "Java".into()]));

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "22:5 | #{\"languages\": [\"Rust\", \"Java\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn test_scalar_cased_as() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), ScriptMessage::PromptForMultiSelect(prompt_info) => {
        assert_eq!(prompt_info.message(), "Languages:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), None);
        assert_matches!(prompt_info.optional(), false);
        assert_matches!(prompt_info.page_size(), Some(10));
    });

    harness.respond(ClientMessage::Array(vec!["Rust".into(), "JavaScript".into()]));

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "34:5 | #{\"languages\": [\"rust\", \"java_script\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn test_scalar_headless_defaults_headless() -> Result<(), ArchetectError> {
    let configuration = Configuration::default()
        .with_headless(true)
        ;

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "46:5 | #{\"languages\": [\"Rust\", \"JavaScript\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn test_scalar_headless_defaults_cased_as_headless() -> Result<(), ArchetectError> {
    let configuration = Configuration::default()
        .with_headless(true)
        ;

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "59:5 | #{\"languages\": [\"rust\", \"javaScript\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn test_scalar_with_defaults() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
        .with_switch(function_name!())
        ;

    let harness = TestHarness::execute(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), ScriptMessage::PromptForMultiSelect(prompt_info) => {
        assert_eq!(prompt_info.message(), "Languages:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), Some(defaults) => {
            assert_eq!(defaults, vec!["Rust", "Java"]);
        });
        assert_matches!(prompt_info.optional(), false);
        assert_matches!(prompt_info.page_size(), Some(10));
    });

    harness.respond(ClientMessage::Array(vec!["Rust".into(), "JavaScript".into()]));

    assert_matches!(harness.receive(), ScriptMessage::Display(output) => {
        assert_eq!(output, "71:5 | #{\"languages\": [\"Rust\", \"JavaScript\"]}: tests/prompts/multiselect_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

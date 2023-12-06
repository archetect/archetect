use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use rhai::Map;

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, PromptInfoLengthRestrictions};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarness;

#[test]
fn test_scalar_text_prompt() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let mut answers = Map::new();
    answers.insert("description".into(), "Customer Service".into());
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers);

    let harness = TestHarness::new(file!(), &configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), CommandRequest::PromptForText(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Prefix:");
        assert_matches!(prompt_info.min(), Some(value) if value == 1);
        assert_matches!(prompt_info.max(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.default(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    harness.respond(CommandResponse::String("Customer".to_string()));

    assert_matches!(harness.receive(), CommandRequest::PromptForText(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Suffix:");
        assert_matches!(prompt_info.min(), Some(value) if value == 2);
        assert_matches!(prompt_info.max(), Some(value) if value == 15);
        assert_matches!(prompt_info.help(), Some(message) if "Enter a Service Suffix" == message);
        assert_matches!(prompt_info.placeholder(), Some(message) if "Service" == message);
        assert_matches!(prompt_info.default(), Some(message) if "Orchestrator" == message);
        assert_matches!(prompt_info.optional(), false);
    });

    harness.respond(CommandResponse::String("Service".to_string()));

    assert_matches!(harness.receive(), CommandRequest::Display(output) => {
        assert_eq!(output, "31:1 | #{\"description\": \"Customer Service\", \"service_prefix\": \
        \"Customer\", \"service_suffix\": \"Service\", \"summary\": \
        \"Extended Summary\"}: tests/prompts/text_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_text_prompt_non_optional() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), &configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::None);

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Service Prefix:' is not optional\nin call to function \
        'prompt' @ 'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_text_prompt_invalid() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), &configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::String("".to_string()));

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Answer Invalid: '' was provided as an answer to 'Service Prefix:', \
        but Answer must have greater than 1 characters.\nin call to function 'prompt' @ \
        'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_text_prompt_unexpected() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), &configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::Integer(1));

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Unexpected Response: 'Service Prefix:' expects a String, but received \
        Integer(1)\nin call to function 'prompt' @ \
        'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    Ok(())
}

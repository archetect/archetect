use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use rhai::Map;

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, PromptInfoLengthRestrictions};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;

use crate::test_utils::TestHarness;

#[test]
fn test_scalar_int_prompt() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let mut answers = Map::new();
    answers.insert("debug_port".into(), 8070.into());
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers);

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    assert_matches!(harness.receive(), CommandRequest::PromptForInt(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Port:");
        assert_matches!(prompt_info.min(), None);
        assert_matches!(prompt_info.max(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.default(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    harness.respond(CommandResponse::Integer(8080));

    assert_matches!(harness.receive(), CommandRequest::PromptForInt(prompt_info) => {
        assert_eq!(prompt_info.message(), "Management Port:");
        assert_matches!(prompt_info.min(), Some(min) if min == 1024);
        assert_matches!(prompt_info.max(), Some(max) if max == 65535);
        assert_matches!(prompt_info.help(), Some("Enter an integer between 1024 and 65535"));
        assert_matches!(prompt_info.placeholder(), Some("Management Port Number"));
        assert_matches!(prompt_info.default(), Some(port) if port == 8081 );
        assert_matches!(prompt_info.optional(), true);
    });

    harness.respond(CommandResponse::Integer(8090));

    assert_matches!(harness.receive(), CommandRequest::Display(output) => {
        assert_eq!(output, "34:1 | #{\"debug_port\": 8070, \"management_port\": 8090, \
        \"rest_port\": 8060, \"service_port\": 8080}: tests/prompts/int_prompt_scalar_tests/archetype.rhai");
    });

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_int_prompt_non_optional() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::None);

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Service Port:' is not optional @ 'tests/prompts/int_prompt_scalar_tests/archetype.rhai'\nin call to function \
        'prompt' (from 'tests/prompts/int_prompt_scalar_tests/archetype.rhai') (line 7, position 24)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_int_prompt_invalid() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();

    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::Integer(8080));

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::Integer(5));

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Answer Invalid: '5' was provided as an answer to 'Management Port:', \
        but Answer must be between 1024 and 65535. @ 'tests/prompts/int_prompt_scalar_tests/archetype.rhai'\nin call to function 'prompt' (from \
        'tests/prompts/int_prompt_scalar_tests/archetype.rhai') (line 11, position 27)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_scalar_int_prompt_unexpected() -> Result<(), ArchetectError> {
    let configuration = Configuration::default();
    let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
    let harness = TestHarness::new(file!(), configuration, render_context)?;

    let _ = harness.receive(); // Swallow Prompt

    harness.respond(CommandResponse::String("8080".to_string()));

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message,"Unexpected Response: The 'Service Port:' prompt expects Int, but received \
        String(\"8080\") @ 'tests/prompts/int_prompt_scalar_tests/archetype.rhai'\nin call to function 'prompt' (from 'tests/prompts/int_prompt_scalar_tests/archetype.rhai') \
        (line 7, position 24)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

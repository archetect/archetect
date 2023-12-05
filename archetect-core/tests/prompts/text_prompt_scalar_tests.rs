use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use rhai::Map;

use archetect_api::{api_driver_and_handle, CommandRequest, CommandResponse, PromptInfo};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;


#[test]
fn test_scalar_int_prompt() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/text_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let mut answers = Map::new();
        answers.insert("description".into(), "Customer Service".into());
        let render_context = RenderContext::new(Utf8PathBuf::new(), answers);

        assert!(archetype.render(runtime_context, render_context).is_ok());
    });

    // Test for defaults
    assert_matches!(handle.receive(), CommandRequest::PromptForText(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Prefix:");
        assert_matches!(prompt_info.min(), Some(value) if value == 1);
        assert_matches!(prompt_info.max(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.default(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    handle.respond(CommandResponse::String("Customer".to_string()));

    assert_matches!(handle.receive(), CommandRequest::PromptForText(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Suffix:");
        assert_matches!(prompt_info.min(), Some(value) if value == 2);
        assert_matches!(prompt_info.max(), Some(value) if value == 15);
        assert_matches!(prompt_info.help(), Some(message) if "Enter a Service Suffix" == message);
        assert_matches!(prompt_info.placeholder(), Some(message) if "Service" == message);
        assert_matches!(prompt_info.default(), Some(message) if "Orchestrator" == message);
        assert_matches!(prompt_info.optional(), false);
    });

    handle.respond(CommandResponse::String("Service".to_string()));

    assert_matches!(handle.receive(), CommandRequest::Display(output) => {
        assert_eq!(output, "31:1 | #{\"description\": \"Customer Service\", \"service_prefix\": \
        \"Customer\", \"service_suffix\": \"Service\", \"summary\": \
        \"Extended Summary\"}: tests/prompts/text_prompt_scalar_tests/archetype.rhai");
    });

    Ok(())
}

#[test]
fn test_scalar_text_prompt_non_optional() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/text_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::None);

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Service Prefix:' is not optional\nin call to function \
        'prompt' @ 'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    Ok(())
}

#[test]
fn test_scalar_text_prompt_invalid() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/text_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::String("".to_string()));

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Answer Invalid: '' was provided as an answer to 'Service Prefix:', \
        but Answer must have greater than 1 characters.\nin call to function 'prompt' @ \
        'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    Ok(())
}

#[test]
fn test_scalar_text_prompt_unexpected() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/text_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::Integer(1));

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Unexpected Response: 'Service Prefix:' expects a String, but received \
        Integer(1)\nin call to function 'prompt' @ \
        'tests/prompts/text_prompt_scalar_tests/archetype.rhai' (line 7, position 26)");
    });

    Ok(())
}

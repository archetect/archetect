use archetect_api::{api_driver_and_handle, CommandRequest, CommandResponse, PromptInfo};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use rhai::Map;

#[test]
fn test_scalar_int_prompt() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/int_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let mut answers = Map::new();
        answers.insert("debug_port".into(), 8070.into());
        let render_context = RenderContext::new(Utf8PathBuf::new(), answers);

        assert!(archetype.render(runtime_context, render_context).is_ok());
    });

    assert_matches!(handle.receive(), CommandRequest::PromptForInt(prompt_info) => {
        assert_eq!(prompt_info.message(), "Service Port:");
        assert_matches!(prompt_info.min(), None);
        assert_matches!(prompt_info.max(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.default(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    handle.respond(CommandResponse::Integer(8080));

    assert_matches!(handle.receive(), CommandRequest::PromptForInt(prompt_info) => {
        assert_eq!(prompt_info.message(), "Management Port:");
        assert_matches!(prompt_info.min(), Some(min) if min == 1024);
        assert_matches!(prompt_info.max(), Some(max) if max == 65535);
        assert_matches!(prompt_info.help(), Some("Enter an integer between 1024 and 65535"));
        assert_matches!(prompt_info.placeholder(), Some("Management Port Number"));
        assert_matches!(prompt_info.default(), Some(port) if port == 8081 );
        assert_matches!(prompt_info.optional(), true);
    });

    handle.respond(CommandResponse::Integer(8090));

    assert_matches!(handle.receive(), CommandRequest::Display(output) => {
        assert_eq!(output, "34:1 | #{\"debug_port\": 8070, \"management_port\": 8090, \
        \"rest_port\": 8060, \"service_port\": 8080}: tests/prompts/int_prompt_scalar_tests/archetype.rhai");
    });

    Ok(())
}

#[test]
fn test_scalar_int_prompt_non_optional() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/int_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::None);

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Service Port:' is not optional\nin call to function \
        'prompt' @ 'tests/prompts/int_prompt_scalar_tests/archetype.rhai' (line 7, position 24)");
    });

    Ok(())
}

#[test]
fn test_scalar_int_prompt_invalid() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/int_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::Integer(8080));

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::Integer(5));

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Answer Invalid: '5' was provided as an answer to 'Management Port:', \
        but Answer must be between 1024 and 65535.\nin call to function 'prompt' @ \
        'tests/prompts/int_prompt_scalar_tests/archetype.rhai' (line 11, position 27)");
    });

    Ok(())
}

#[test]
fn test_scalar_int_prompt_unexpected() -> Result<(), ArchetectError> {
    let (driver, handle) = api_driver_and_handle();
    let runtime_context = RuntimeContext::builder()
        .with_driver(driver)
        .with_temp_layout()?
        .build()?;
    let archetype = runtime_context.new_archetype("tests/prompts/int_prompt_scalar_tests")?;

    std::thread::spawn(move || {
        let render_context = RenderContext::new(Utf8PathBuf::new(), Default::default());
        assert!(archetype.render(runtime_context, render_context).is_err());
    });

    let _ = handle.receive(); // Swallow Prompt

    handle.respond(CommandResponse::String("8080".to_string()));

    assert_matches!(handle.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Unexpected Response: 'Service Port:' expects Int, but received \
        String(\"8080\")\nin call to function 'prompt' @ \
        'tests/prompts/int_prompt_scalar_tests/archetype.rhai' (line 7, position 24)");
    });

    Ok(())
}

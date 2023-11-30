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
    let archetype = runtime_context.new_archetype("tests/archetypes/scripting/prompts/int")?;

    std::thread::spawn(move || {
        let mut answers = Map::new();
        answers.insert("debug_port".into(), 8070.into());
        let render_context = RenderContext::new(Utf8PathBuf::new(), answers)
            ;

        archetype.render(runtime_context, render_context).unwrap();
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

    handle.respond(CommandResponse::IntAnswer(8080));

    assert_matches!(handle.receive(), CommandRequest::PromptForInt(prompt_info) => {
        assert_eq!(prompt_info.message(), "Management Port:");
        assert_matches!(prompt_info.min(), Some(min) if min == 1024);
        assert_matches!(prompt_info.max(), Some(max) if max == 65535);
        assert_matches!(prompt_info.help(), Some("Enter an integer between 1024 and 65535"));
        assert_matches!(prompt_info.placeholder(), Some("Management Port Number"));
        assert_matches!(prompt_info.default(), Some(port) if port == 8081 );
        assert_matches!(prompt_info.optional(), true);
    });

    handle.respond(CommandResponse::IntAnswer(8090));

    assert_matches!(handle.receive(), CommandRequest::Display(output) => {
        assert_eq!(output, "22:1 | #{\"debug_port\": 8070, \"management_port\": 8090, \"service_port\": 8080}");
    });

    Ok(())
}


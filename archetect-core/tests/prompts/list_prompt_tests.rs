use assert_matches::assert_matches;
use camino::Utf8PathBuf;
use function_name::named;
use indoc::indoc;
use rhai::Map;

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, PromptInfoItemsRestrictions};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;

use crate::test_utils::TestHarness;

#[test]
fn test_simple_defaults() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch("test_simple");

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    harness.respond(CommandResponse::None);

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Services:' is not optional\nin call to function 'prompt' @ \
        'tests/prompts/list_prompt_tests/archetype.rhai' (line 4, position 24)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_map_defaults() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch("test_map");

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    // Test for defaults
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
        assert_matches!(prompt_info.min_items(), None);
        assert_matches!(prompt_info.max_items(), None);
        assert_matches!(prompt_info.help(), None);
        assert_matches!(prompt_info.placeholder(), None);
        assert_matches!(prompt_info.defaults(), None);
        assert_matches!(prompt_info.optional(), false);
    });

    harness.respond(CommandResponse::None);

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Required: 'Services:' (key: 'services') is not optional\nin call to \
        function 'prompt' @ 'tests/prompts/list_prompt_tests/archetype.rhai' (line 11, position 16)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

#[test]
fn test_simple_non_cased_results() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch("test_simple");

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    // Test Prompt
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
    });

    harness.respond(CommandResponse::Array(vec!["Cart".to_string(), "customer".to_string(), "transactionProcessing".to_string()]));

    assert_matches!(harness.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, indoc! {"
           services:
           - Cart
           - customer
           - transactionProcessing
        "
    })});

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
fn test_simple_cased_results() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch("test_simple_cased");

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    // Test Prompt
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
    });

    harness.respond(CommandResponse::Array(vec!["Cart".to_string(), "customer".to_string(), "transactionProcessing".to_string()]));

    assert_matches!(harness.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, indoc! {"
           services:
           - cart
           - customer
           - transaction-processing
        "
    })});

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
fn test_map_cased_results() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch("test_map_cased");

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    // Test Prompt
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
    });

    harness.respond(CommandResponse::Array(vec!["Cart".to_string(), "customer".to_string(), "transactionProcessing".to_string()]));

    assert_matches!(harness.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, indoc! {"
            services:
            - item-name: cart
              item_name: Cart
            - item-name: customer
              item_name: customer
            - item-name: transaction-processing
              item_name: transactionProcessing
        "
    })});

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn  test_map_cased_as_with_single_strategy() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch(function_name!());

    let harness = TestHarness::new(file!(), configuration, render_context)?;
    // Test Prompt
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
    });

    harness.respond(CommandResponse::Array(vec!["Cart".to_string(), "customer".to_string(), "transactionProcessing".to_string()]));

    assert_matches!(harness.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, indoc! {"
            services:
            - item-name: cart
              item_name: Cart
            - item-name: customer
              item_name: customer
            - item-name: transaction-processing
              item_name: transactionProcessing
        "
    })});

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn  test_simple_cased_as_with_single_style() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch(function_name!());

    let harness = TestHarness::new(file!(), configuration, render_context)?;
    // Test Prompt
    assert_matches!(harness.receive(), CommandRequest::PromptForList(prompt_info) => {
        assert_eq!(prompt_info.message(), "Services:");
    });

    harness.respond(CommandResponse::Array(vec!["Cart".to_string(), "customer".to_string(), "transactionProcessing".to_string()]));

    assert_matches!(harness.receive(), CommandRequest::Print(message) => {
        assert_eq!(message, indoc! {"
           services:
           - cart
           - customer
           - transaction-processing
        "
    })});

    assert!(harness.render_succeeded());

    Ok(())
}

#[test]
#[named]
fn  test_map_cased_as_with_string_strategy() -> anyhow::Result<()> {
    let configuration = Configuration::default();

    let answers = Map::new();
    let render_context = RenderContext::new(Utf8PathBuf::new(), answers).with_switch(function_name!());

    let harness = TestHarness::new(file!(), configuration, render_context)?;

    assert_matches!(harness.receive(), CommandRequest::LogError(message) => {
        assert_eq!(message, "Invalid Setting: For the 'Services:' prompt (key: 'services'), the 'cased_as' setting must \
        be an array of CaseStrategy elements, but contains \"CamelCase\" (string)\nin call to function 'prompt' @ \
        'tests/prompts/list_prompt_tests/archetype.rhai' (line 52, position 16)");
    });

    assert!(!harness.render_succeeded());

    Ok(())
}

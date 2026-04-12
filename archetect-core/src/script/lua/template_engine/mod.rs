pub mod builtins;
mod compiler;
mod error;
pub mod include_resolver;
pub mod render;
mod tokenizer;

pub use compiler::{CompileOptions, Compiler};
pub use error::TemplateCompileError;
pub use include_resolver::IncludeResolver;
use tokenizer::Tokenizer;

/// A compiled template: Lua source code ready to be loaded into an mlua VM.
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    /// The Lua source code (a function definition).
    pub source: String,
}

/// Compiles Archetect Template Language (ATL) templates into Lua functions.
///
/// Templates use `{{ expr | filter }}` for interpolation and `{% lua_code %}`
/// for logic blocks. The compiled Lua function receives a context table and
/// a filters table, and returns the rendered string.
pub struct TemplateCompiler;

impl TemplateCompiler {
    /// Compile a template string into Lua source code.
    ///
    /// Convenience entry point for callers that don't have an
    /// [`IncludeResolver`] or custom [`CompileOptions`] — uses a disabled
    /// resolver and default options, so any `{% include %}` directive will
    /// fail with `IncludeNotFound` and strict mode is off.
    /// The `_name` parameter is reserved for future error-reporting use.
    pub fn compile(template: &str, _name: &str) -> Result<CompiledTemplate, TemplateCompileError> {
        let mut resolver = IncludeResolver::disabled();
        Self::compile_with(template, _name, &mut resolver, CompileOptions::default())
    }

    /// Compile a template string with a configured [`IncludeResolver`] and
    /// [`CompileOptions`]. `{% include "..." %}` directives are resolved
    /// against the resolver's includes directory and inlined at compile
    /// time. Options control strict-mode resolution and whitespace controls.
    pub fn compile_with(
        template: &str,
        name: &str,
        resolver: &mut IncludeResolver,
        opts: CompileOptions,
    ) -> Result<CompiledTemplate, TemplateCompileError> {
        let tokens = Tokenizer::tokenize(template)?;
        let source = Compiler::compile(&tokens, resolver, opts)?;
        validate_lua_syntax(&source, name)?;
        Ok(CompiledTemplate { source })
    }
}

/// Try to parse the generated Lua source so that malformed `{% ... %}` blocks
/// surface as compile-time errors instead of being deferred to render-time.
///
/// This spins up a short-lived `mlua::Lua` purely for parsing — `into_function`
/// loads but does not execute the chunk. Templates are compile-cached, so the
/// cost is paid once per unique template per render.
///
/// Phase 8.4: `template_name` is the human-readable identifier of the
/// template being compiled (file path or `<inline>`). It's embedded in the
/// `InvalidLuaSyntax` error so the user can tell which template a Lua
/// parse error came from — especially useful when an error originates
/// inside a transitively included partial.
fn validate_lua_syntax(source: &str, template_name: &str) -> Result<(), TemplateCompileError> {
    let lua = mlua::Lua::new();
    lua.load(source)
        .into_function()
        .map(|_| ())
        .map_err(|err| TemplateCompileError::InvalidLuaSyntax {
            template: template_name.to_string(),
            detail: err.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let result = TemplateCompiler::compile("Hello {{ name }}!", "test");
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert!(compiled.source.contains("__w(name)"));
    }

    #[test]
    fn test_compile_error_unterminated() {
        let result = TemplateCompiler::compile("Hello {{ name", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_and_execute_simple() {
        let compiled = TemplateCompiler::compile("Hello {{ name }}!", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "World").unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_compile_and_execute_dotted_access() {
        let compiled = TemplateCompiler::compile("{{ entity.name.pascal }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let name_table = lua.create_table().unwrap();
        name_table.set("pascal", "OrderItem").unwrap();
        let entity = lua.create_table().unwrap();
        entity.set("name", name_table).unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("entity", entity).unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "OrderItem");
    }

    #[test]
    fn test_compile_and_execute_filter() {
        let compiled = TemplateCompiler::compile("{{ name | shout }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "hello").unwrap();

        let filters = lua.create_table().unwrap();
        let shout_fn = lua.create_function(|_, s: String| {
            Ok(s.to_uppercase() + "!")
        }).unwrap();
        filters.set("shout", shout_fn).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "HELLO!");
    }

    #[test]
    fn test_compile_and_execute_for_loop() {
        let template = "{% for _, item in ipairs(items) do %}{{ item }}\n{% end %}";
        let compiled = TemplateCompiler::compile(template, "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let items = lua.create_sequence_from(["apple", "banana", "cherry"]).unwrap();
        let ctx = lua.create_table().unwrap();
        ctx.set("items", items).unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "apple\nbanana\ncherry\n");
    }

    #[test]
    fn test_compile_and_execute_if_else() {
        let template = "{% if show then %}visible{% else %}hidden{% end %}";
        let compiled = TemplateCompiler::compile(template, "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let filters = lua.create_table().unwrap();

        // Test with show = true
        let ctx = lua.create_table().unwrap();
        ctx.set("show", true).unwrap();
        let result: String = func.call::<String>((ctx.clone(), filters.clone())).unwrap();
        assert_eq!(result, "visible");

        // Test with show = false
        let ctx = lua.create_table().unwrap();
        ctx.set("show", false).unwrap();
        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "hidden");
    }

    #[test]
    fn test_compile_and_execute_nested_iteration() {
        let template = r#"{% for _, entity in ipairs(entities) do %}
Entity: {{ entity.name }}
{% for _, field in ipairs(entity.fields) do %}
  - {{ field.name }}: {{ field.field_type }}
{% end %}
{% end %}"#;
        let compiled = TemplateCompiler::compile(template, "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        // Build nested data: entities with fields
        let field1 = lua.create_table().unwrap();
        field1.set("name", "id").unwrap();
        field1.set("field_type", "UUID").unwrap();

        let field2 = lua.create_table().unwrap();
        field2.set("name", "email").unwrap();
        field2.set("field_type", "String").unwrap();

        let entity = lua.create_table().unwrap();
        entity.set("name", "Customer").unwrap();
        let fields = lua.create_sequence_from([field1, field2]).unwrap();
        entity.set("fields", fields).unwrap();

        let entities = lua.create_sequence_from([entity]).unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("entities", entities).unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert!(result.contains("Entity: Customer"));
        assert!(result.contains("- id: UUID"));
        assert!(result.contains("- email: String"));
    }

    #[test]
    fn test_compile_and_execute_filter_chain() {
        let compiled = TemplateCompiler::compile("{{ name | lower | reverse }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "Hello").unwrap();

        let filters = lua.create_table().unwrap();
        let lower_fn = lua.create_function(|_, s: String| Ok(s.to_lowercase())).unwrap();
        let reverse_fn = lua.create_function(|_, s: String| {
            Ok(s.chars().rev().collect::<String>())
        }).unwrap();
        filters.set("lower", lower_fn).unwrap();
        filters.set("reverse", reverse_fn).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "olleh");
    }

    /// The killer test: reproduce the exact model-driven generation scenario
    /// that MiniJinja can't handle — nested entity data with field iteration.
    #[test]
    fn test_model_driven_proto_template() {
        let template = r#"syntax = "proto3";

message {{ entity.name.pascal }}Response {
{% for i, field in ipairs(entity.local_fields) do %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{% end %}
}

service {{ entity.name.pascal }}Service {
    rpc Get{{ entity.name.pascal }} (Get{{ entity.name.pascal }}Request) returns ({{ entity.name.pascal }}Response);
}"#;

        let compiled = TemplateCompiler::compile(template, "entity.proto").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        // Build entity with case-expanded name and typed fields
        let name = lua.create_table().unwrap();
        name.set("pascal", "Customer").unwrap();
        name.set("snake", "customer").unwrap();

        let id_name = lua.create_table().unwrap();
        id_name.set("snake", "id").unwrap();
        let id_field = lua.create_table().unwrap();
        id_field.set("name", id_name).unwrap();
        id_field.set("field_type", "UUID").unwrap();

        let email_name = lua.create_table().unwrap();
        email_name.set("snake", "email").unwrap();
        let email_field = lua.create_table().unwrap();
        email_field.set("name", email_name).unwrap();
        email_field.set("field_type", "String").unwrap();

        let local_fields = lua.create_sequence_from([id_field, email_field]).unwrap();

        let entity = lua.create_table().unwrap();
        entity.set("name", name).unwrap();
        entity.set("local_fields", local_fields).unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("entity", entity).unwrap();

        // Register proto_type filter
        let filters = lua.create_table().unwrap();
        let proto_type_fn = lua.create_function(|_, field: mlua::Table| {
            let ft: String = field.get("field_type")?;
            let proto = match ft.as_str() {
                "UUID" => "string",
                "String" => "string",
                "Integer" => "int64",
                "Boolean" => "bool",
                _ => "string",
            };
            Ok(proto.to_string())
        }).unwrap();
        filters.set("proto_type", proto_type_fn).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();

        assert!(result.contains("syntax = \"proto3\";"));
        assert!(result.contains("message CustomerResponse {"));
        assert!(result.contains("string id = 1;"));
        assert!(result.contains("string email = 2;"));
        assert!(result.contains("service CustomerService {"));
        assert!(result.contains("rpc GetCustomer (GetCustomerRequest) returns (CustomerResponse);"));
    }

    // ---------- Error-case coverage ----------

    #[test]
    fn test_missing_context_var_renders_empty() {
        // Undefined context vars resolve to nil; the writer drops nil silently
        // rather than emitting the literal "nil" into the generated output.
        let compiled = TemplateCompiler::compile("Hello {{ name }}!", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello !");
    }

    // ---------- Phase 3.0: filter/function symmetry ----------

    #[test]
    fn test_bare_function_call_resolves_through_filters() {
        // {{ upper_case(name) }} — `upper_case` is a bare identifier inside the
        // expression. It should resolve via _ENV → __filters at render time.
        let compiled = TemplateCompiler::compile("{{ upper_case(name) }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "hello").unwrap();

        let filters = lua.create_table().unwrap();
        let upper_case = lua
            .create_function(|_, s: String| Ok(s.to_uppercase()))
            .unwrap();
        filters.set("upper_case", upper_case).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_pipe_and_function_forms_produce_same_output() {
        // The principle: every filter is also a callable function. Both forms
        // should yield identical output for the same inputs.
        let pipe_form = TemplateCompiler::compile("{{ name | upper_case }}", "test").unwrap();
        let fn_form = TemplateCompiler::compile("{{ upper_case(name) }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let pipe_fn: mlua::Function = lua.load(&pipe_form.source).eval().unwrap();
        let call_fn: mlua::Function = lua.load(&fn_form.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "world").unwrap();

        let filters = lua.create_table().unwrap();
        let upper_case = lua
            .create_function(|_, s: String| Ok(s.to_uppercase()))
            .unwrap();
        filters.set("upper_case", upper_case).unwrap();

        let pipe_result: String = pipe_fn.call::<String>((ctx.clone(), filters.clone())).unwrap();
        let call_result: String = call_fn.call::<String>((ctx, filters)).unwrap();
        assert_eq!(pipe_result, "WORLD");
        assert_eq!(call_result, "WORLD");
        assert_eq!(pipe_result, call_result);
    }

    #[test]
    fn test_nested_function_call_resolves_through_filters() {
        // {{ upper_case(snake_case(name)) }} — both names resolve via _ENV.
        let compiled = TemplateCompiler::compile(
            "{{ upper_case(snake_case(name)) }}",
            "test",
        )
        .unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "HelloWorld").unwrap();

        let filters = lua.create_table().unwrap();
        filters
            .set(
                "snake_case",
                lua.create_function(|_, s: String| {
                    Ok(s.chars()
                        .enumerate()
                        .flat_map(|(i, c)| {
                            if c.is_uppercase() && i > 0 {
                                vec!['_', c.to_ascii_lowercase()]
                            } else {
                                vec![c.to_ascii_lowercase()]
                            }
                        })
                        .collect::<String>())
                })
                .unwrap(),
            )
            .unwrap();
        filters
            .set(
                "upper_case",
                lua.create_function(|_, s: String| Ok(s.to_uppercase())).unwrap(),
            )
            .unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "HELLO_WORLD");
    }

    #[test]
    fn test_filter_takes_precedence_over_context() {
        // If a context key shadows a filter name, the filter wins. This is the
        // documented behavior — built-ins are precedence-protected so that
        // `ctx:set("now", ...)` cannot accidentally break templates that call
        // the `now()` builtin.
        let compiled = TemplateCompiler::compile("{{ greet(name) }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "World").unwrap();
        ctx.set("greet", "I am a string, not a function").unwrap();

        let filters = lua.create_table().unwrap();
        let greet = lua
            .create_function(|_, name: String| Ok(format!("Hello, {}!", name)))
            .unwrap();
        filters.set("greet", greet).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    // ---------- Phase 2: filter argument coverage ----------

    #[test]
    fn test_filter_with_single_arg_renders() {
        // {{ name | truncate(5) }} should call __filters.truncate(name, 5).
        let compiled = TemplateCompiler::compile("{{ name | truncate(5) }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "abcdefghij").unwrap();

        let filters = lua.create_table().unwrap();
        let truncate = lua
            .create_function(|_, (s, n): (String, usize)| {
                Ok(s.chars().take(n).collect::<String>())
            })
            .unwrap();
        filters.set("truncate", truncate).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "abcde");
    }

    #[test]
    fn test_filter_with_multiple_args_renders() {
        // {{ name | replace("a", "b") }}
        let compiled = TemplateCompiler::compile(
            r#"{{ name | replace("a", "b") }}"#,
            "test",
        )
        .unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "banana").unwrap();

        let filters = lua.create_table().unwrap();
        let replace = lua
            .create_function(|_, (s, from, to): (String, String, String)| {
                Ok(s.replace(&from, &to))
            })
            .unwrap();
        filters.set("replace", replace).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "bbnbnb");
    }

    #[test]
    fn test_filter_arg_resolves_context_var() {
        // {{ greeting | with_name(name) }} — the arg is a bare identifier that
        // must resolve through _ENV → __ctx at render time.
        let compiled = TemplateCompiler::compile(
            "{{ greeting | with_name(name) }}",
            "test",
        )
        .unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("greeting", "Hello").unwrap();
        ctx.set("name", "World").unwrap();

        let filters = lua.create_table().unwrap();
        let with_name = lua
            .create_function(|_, (greeting, name): (String, String)| {
                Ok(format!("{}, {}!", greeting, name))
            })
            .unwrap();
        filters.set("with_name", with_name).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_filter_chain_with_args_renders() {
        // {{ name | truncate(3) | upper_case }}
        let compiled = TemplateCompiler::compile(
            "{{ name | truncate(3) | upper_case }}",
            "test",
        )
        .unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "hello world").unwrap();

        let filters = lua.create_table().unwrap();
        let truncate = lua
            .create_function(|_, (s, n): (String, usize)| {
                Ok(s.chars().take(n).collect::<String>())
            })
            .unwrap();
        filters.set("truncate", truncate).unwrap();
        let upper_case = lua
            .create_function(|_, s: String| Ok(s.to_uppercase()))
            .unwrap();
        filters.set("upper_case", upper_case).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "HEL");
    }

    // ---------- Original Phase 1 tests ----------

    #[test]
    fn test_explicit_nil_renders_empty() {
        // An explicit nil in the context behaves the same as a missing key:
        // it produces an empty interpolation rather than the string "nil".
        let compiled = TemplateCompiler::compile("Hello {{ name }}!", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", mlua::Value::Nil).unwrap();
        let filters = lua.create_table().unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello !");
    }

    #[test]
    fn test_filter_runtime_failure_propagates() {
        // A filter that calls error() should produce a render-time mlua error
        // (not a panic) — render.rs maps this to RenderError::LuaTemplateRuntimeError.
        let compiled = TemplateCompiler::compile("{{ name | boom }}", "test").unwrap();

        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("name", "anything").unwrap();

        let filters = lua.create_table().unwrap();
        let boom = lua
            .create_function(|_, _: String| -> mlua::Result<String> {
                Err(mlua::Error::RuntimeError("boom".to_string()))
            })
            .unwrap();
        filters.set("boom", boom).unwrap();

        let result = func.call::<String>((ctx, filters));
        assert!(result.is_err(), "expected runtime error, got {:?}", result);
    }

    #[test]
    fn test_invalid_lua_in_logic_block_caught_at_compile() {
        // A `{% ... %}` block containing malformed Lua should now be caught
        // by the compile-time syntax validator, not deferred to render time.
        let result = TemplateCompiler::compile("{% if then %}oops{% end %}", "test");
        match result {
            Err(TemplateCompileError::InvalidLuaSyntax { .. }) => {}
            other => panic!(
                "expected InvalidLuaSyntax compile error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_invalid_lua_unclosed_block_caught_at_compile() {
        // Missing `end` should also be a compile-time failure.
        let result = TemplateCompiler::compile(
            "{% for i = 1, 3 do %}{{ i }}",
            "test",
        );
        assert!(
            matches!(result, Err(TemplateCompileError::InvalidLuaSyntax { .. })),
            "expected InvalidLuaSyntax, got {:?}",
            result
        );
    }

    // ---------- Phase 4: includes ----------
    //
    // Each test sets up a temp includes directory, drops in some `.atl`
    // partials, and confirms that compile_with inlines them correctly.

    fn temp_includes_dir() -> (tempfile::TempDir, camino::Utf8PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).unwrap();
        (tmp, dir)
    }

    fn write_include(dir: &camino::Utf8Path, name: &str, contents: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    fn render_with_includes(
        template: &str,
        includes_dir: camino::Utf8PathBuf,
        ctx_setup: impl FnOnce(&mlua::Lua, &mlua::Table),
    ) -> Result<String, TemplateCompileError> {
        let mut resolver = IncludeResolver::single(includes_dir);
        let compiled = TemplateCompiler::compile_with(
            template,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        )?;
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        ctx_setup(&lua, &ctx);
        let filters = lua.create_table().unwrap();
        Ok(func.call::<String>((ctx, filters)).unwrap())
    }

    #[test]
    fn test_include_basic() {
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "header.atl", "Hello world");

        let result = render_with_includes(
            r#"prefix: {% include "header.atl" %} :suffix"#,
            dir,
            |_, _| {},
        )
        .unwrap();
        assert_eq!(result, "prefix: Hello world :suffix");
    }

    #[test]
    fn test_include_uses_outer_context() {
        // The included file references `name` — it must resolve through the
        // outer template's __ctx because the include is inlined and shares
        // the same _ENV chain.
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "greet.atl", "Hello {{ name }}!");

        let result = render_with_includes(
            r#"{% include "greet.atl" %}"#,
            dir,
            |_, ctx| {
                ctx.set("name", "World").unwrap();
            },
        )
        .unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_include_in_loop() {
        // The same partial is included once per loop iteration. Each
        // iteration uses the loop variable from the outer template.
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "row.atl", "[{{ x }}]");

        let template = r#"{% for _, x in ipairs(items) do %}{% include "row.atl" %}{% end %}"#;
        let result = render_with_includes(template, dir, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "a").unwrap();
            arr.set(2, "b").unwrap();
            arr.set(3, "c").unwrap();
            ctx.set("items", arr).unwrap();
        })
        .unwrap();
        assert_eq!(result, "[a][b][c]");
    }

    #[test]
    fn test_nested_include() {
        // A includes B includes C — output is the concatenation of all three.
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "a.atl", r#"A({% include "b.atl" %})"#);
        write_include(&dir, "b.atl", r#"B({% include "c.atl" %})"#);
        write_include(&dir, "c.atl", "C");

        let result = render_with_includes(
            r#"{% include "a.atl" %}"#,
            dir,
            |_, _| {},
        )
        .unwrap();
        assert_eq!(result, "A(B(C))");
    }

    #[test]
    fn test_include_not_found() {
        let (_tmp, dir) = temp_includes_dir();
        let mut resolver = IncludeResolver::single(dir);
        let result = TemplateCompiler::compile_with(
            r#"{% include "missing.atl" %}"#,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        );
        assert!(
            matches!(result, Err(TemplateCompileError::IncludeNotFound { .. })),
            "got {:?}",
            result
        );
    }

    #[test]
    fn test_include_cycle_detected() {
        // a.atl includes b.atl which includes a.atl back. The compile-time
        // resolver should catch this rather than letting it loop forever.
        // The cycle is detected inside a nested compile, so the error is
        // wrapped in IncludeChain entries — the root cause should still be
        // an IncludeCycle.
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "a.atl", r#"a:{% include "b.atl" %}"#);
        write_include(&dir, "b.atl", r#"b:{% include "a.atl" %}"#);

        let mut resolver = IncludeResolver::single(dir);
        let result = TemplateCompiler::compile_with(
            r#"{% include "a.atl" %}"#,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err.root_cause(), TemplateCompileError::IncludeCycle { .. }),
            "got {:?}",
            err
        );
        // The chain should mention the partials so the user knows where
        // the cycle was detected.
        let msg = err.to_string();
        assert!(msg.contains("a.atl"), "error should mention a.atl: {}", msg);
        assert!(msg.contains("b.atl"), "error should mention b.atl: {}", msg);
    }

    #[test]
    fn test_include_path_traversal_rejected() {
        // `..` in an include path must be rejected even if a file exists
        // at the resolved location, so authors can't escape the includes
        // sandbox.
        let (_tmp, dir) = temp_includes_dir();
        let mut resolver = IncludeResolver::single(dir);
        let result = TemplateCompiler::compile_with(
            r#"{% include "../escape.atl" %}"#,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        );
        assert!(
            matches!(result, Err(TemplateCompileError::IncludeNotFound { .. })),
            "got {:?}",
            result
        );
    }

    // ---------- Phase 7: sugar — end-to-end render ----------

    fn render_simple(template: &str, ctx_setup: impl FnOnce(&mlua::Lua, &mlua::Table)) -> String {
        let compiled = TemplateCompiler::compile(template, "test").unwrap();
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        ctx_setup(&lua, &ctx);
        let filters = lua.create_table().unwrap();
        func.call::<String>((ctx, filters)).unwrap()
    }

    #[test]
    fn test_for_sugar_single_var_renders() {
        let result = render_simple(
            r#"{% for item in items %}[{{ item }}]{% end %}"#,
            |lua, ctx| {
                let arr = lua.create_table().unwrap();
                arr.set(1, "a").unwrap();
                arr.set(2, "b").unwrap();
                arr.set(3, "c").unwrap();
                ctx.set("items", arr).unwrap();
            },
        );
        assert_eq!(result, "[a][b][c]");
    }

    #[test]
    fn test_for_sugar_two_var_renders() {
        // pairs() iterates a map; the result depends on Lua's hash order so
        // we sort the output for a deterministic comparison.
        let result = render_simple(
            r#"{% for k, v in items %}{{ k }}={{ v }};{% end %}"#,
            |lua, ctx| {
                let map = lua.create_table().unwrap();
                map.set("a", "1").unwrap();
                map.set("b", "2").unwrap();
                ctx.set("items", map).unwrap();
            },
        );
        let mut parts: Vec<&str> = result.trim_end_matches(';').split(';').collect();
        parts.sort();
        assert_eq!(parts, vec!["a=1", "b=2"]);
    }

    #[test]
    fn test_local_declaration_renders() {
        // ATL deliberately does not sugar `local` — authors write Lua-native
        // `local NAME = EXPR` inside `{% ... %}` blocks. The local is in
        // scope for the rest of the compiled function.
        let result = render_simple(
            r#"{% local greeting = "Hello" %}{{ greeting }}, {{ name }}!"#,
            |_, ctx| {
                ctx.set("name", "World").unwrap();
            },
        );
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_range_sugar_one_arg_renders() {
        let result = render_simple(
            r#"{% for i in range(5) %}{{ i }};{% end %}"#,
            |_, _| {},
        );
        assert_eq!(result, "0;1;2;3;4;");
    }

    #[test]
    fn test_range_sugar_two_args_renders() {
        let result = render_simple(
            r#"{% for i in range(2, 6) %}{{ i }};{% end %}"#,
            |_, _| {},
        );
        assert_eq!(result, "2;3;4;5;");
    }

    #[test]
    fn test_range_sugar_three_args_renders() {
        let result = render_simple(
            r#"{% for i in range(0, 10, 2) %}{{ i }};{% end %}"#,
            |_, _| {},
        );
        assert_eq!(result, "0;2;4;6;8;");
    }

    #[test]
    fn test_explicit_lua_for_still_works() {
        // Author falls back to raw Lua — no rewrite, runs as-is.
        let result = render_simple(
            r#"{% for i = 1, 5 do %}{{ i }};{% end %}"#,
            |_, _| {},
        );
        assert_eq!(result, "1;2;3;4;5;");
    }

    // ---------- Phase 7.3: trim_blocks / lstrip_blocks ----------

    fn render_with_opts(
        template: &str,
        opts: CompileOptions,
        ctx_setup: impl FnOnce(&mlua::Lua, &mlua::Table),
    ) -> String {
        let mut resolver = IncludeResolver::disabled();
        let compiled =
            TemplateCompiler::compile_with(template, "test", &mut resolver, opts).unwrap();
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        ctx_setup(&lua, &ctx);
        let filters = lua.create_table().unwrap();
        func.call::<String>((ctx, filters)).unwrap()
    }

    #[test]
    fn test_trim_blocks_strips_first_newline_after_block() {
        // Without trim_blocks: the newline after `{% local x = 1 %}` would
        // appear in the output. With trim_blocks: it's stripped.
        let opts = CompileOptions {
            trim_blocks: true,
            ..CompileOptions::default()
        };
        let result = render_with_opts("{% local x = 1 %}\nLine after", opts, |_, _| {});
        assert_eq!(result, "Line after");
    }

    #[test]
    fn test_trim_blocks_off_keeps_newline() {
        let result = render_with_opts(
            "{% local x = 1 %}\nLine after",
            CompileOptions::default(),
            |_, _| {},
        );
        assert_eq!(result, "\nLine after");
    }

    #[test]
    fn test_lstrip_blocks_strips_indent_before_block() {
        // The leading spaces on the line containing `{% local x = 1 %}`
        // should be stripped, but the leading spaces on `Hello` should remain.
        let opts = CompileOptions {
            lstrip_blocks: true,
            trim_blocks: true,
            ..CompileOptions::default()
        };
        let template = "Hello\n    {% local x = 1 %}\nWorld";
        let result = render_with_opts(template, opts, |_, _| {});
        assert_eq!(result, "Hello\nWorld");
    }

    #[test]
    fn test_lstrip_blocks_does_not_strip_content_indent() {
        // The leading spaces before "Indented" must be preserved when the
        // next token is not a block tag.
        let opts = CompileOptions {
            lstrip_blocks: true,
            ..CompileOptions::default()
        };
        let result = render_with_opts(
            "    Indented {{ name }}",
            opts,
            |_, ctx| {
                ctx.set("name", "value").unwrap();
            },
        );
        assert_eq!(result, "    Indented value");
    }

    // ---------- Phase 6: strict mode ----------
    //
    // Strict mode installs a metatable on __ctx so any rawget-nil lookup
    // raises a Lua RuntimeError that surfaces as a render-time failure
    // instead of silently rendering empty.

    fn render_strict(
        template: &str,
        ctx_setup: impl FnOnce(&mlua::Lua, &mlua::Table),
    ) -> Result<String, mlua::Error> {
        let mut resolver = IncludeResolver::disabled();
        let opts = CompileOptions {
            strict: true,
            ..CompileOptions::default()
        };
        let compiled =
            TemplateCompiler::compile_with(template, "test", &mut resolver, opts).unwrap();
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        ctx_setup(&lua, &ctx);
        let filters = lua.create_table().unwrap();
        func.call::<String>((ctx, filters))
    }

    #[test]
    fn test_strict_mode_errors_on_undefined() {
        // No `name` set in context — strict mode should error.
        let err = render_strict("Hello {{ name }}!", |_, _| {}).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("undefined template variable"),
            "expected undefined-variable error, got: {}",
            msg
        );
        assert!(msg.contains("name"), "error should name the missing key, got: {}", msg);
    }

    #[test]
    fn test_strict_mode_allows_defined_var() {
        let result = render_strict("Hello {{ name }}!", |_, ctx| {
            ctx.set("name", "World").unwrap();
        })
        .unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_strict_mode_errors_on_undefined_in_logic_block() {
        // The metatable also fires inside `{% %}` Lua code, since logic
        // blocks resolve names through the same _ENV chain.
        let err = render_strict(
            r#"{% for _, x in ipairs(items) do %}{{ x }}{% end %}"#,
            |_, _| {},
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("items"),
            "expected error mentioning `items`, got: {}",
            err
        );
    }

    #[test]
    fn test_strict_mode_lenient_mode_still_works() {
        // Default options should NOT be strict — undefined renders empty.
        let mut resolver = IncludeResolver::disabled();
        let compiled = TemplateCompiler::compile_with(
            "Hello {{ name }}!",
            "test",
            &mut resolver,
            CompileOptions::default(),
        )
        .unwrap();
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        let filters = lua.create_table().unwrap();
        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "Hello !");
    }

    #[test]
    fn test_strict_mode_does_not_break_filter_lookup() {
        // Filters live in __filters, which is checked BEFORE __ctx in the
        // lookup chain. The strict metatable on __ctx must not interfere
        // with filter resolution.
        let mut resolver = IncludeResolver::disabled();
        let opts = CompileOptions {
            strict: true,
            ..CompileOptions::default()
        };
        let compiled = TemplateCompiler::compile_with(
            "{{ name | upper_case }}",
            "test",
            &mut resolver,
            opts,
        )
        .unwrap();
        let lua = mlua::Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        ctx.set("name", "hello").unwrap();
        let filters = lua.create_table().unwrap();
        let upper = lua.create_function(|_, s: String| Ok(s.to_uppercase())).unwrap();
        filters.set("upper_case", upper).unwrap();
        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_include_chain_error_names_partial() {
        // Phase 8.4: when an inner include has a tokenizer error, the
        // outer error message should name the partial so the user knows
        // where to look. The leaf error stays accessible via root_cause().
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "broken.atl", "Hello {{ unterminated");

        let mut resolver = IncludeResolver::single(dir);
        let result = TemplateCompiler::compile_with(
            r#"{% include "broken.atl" %}"#,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("broken.atl"),
            "error should name the partial: {}",
            msg
        );
        assert!(
            matches!(
                err.root_cause(),
                TemplateCompileError::UnterminatedExpression { .. }
            ),
            "root cause should be UnterminatedExpression, got {:?}",
            err.root_cause()
        );
    }

    #[test]
    fn test_invalid_lua_syntax_includes_template_name() {
        // Phase 8.4: InvalidLuaSyntax errors carry the template name so
        // the user can tell which template the bad logic block came from.
        let result = TemplateCompiler::compile("{% if then %}oops{% end %}", "my_template.atl");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("my_template.atl"),
            "error should mention template name: {}",
            msg
        );
    }

    #[test]
    fn test_include_disabled_resolver_errors() {
        // The default `compile()` uses a disabled resolver, so any
        // `{% include %}` directive in a template compiled this way is an
        // error. This protects callers like `lua_render_path` (filename
        // templating) from accidentally enabling includes.
        let result = TemplateCompiler::compile(r#"{% include "header.atl" %}"#, "test");
        assert!(
            matches!(result, Err(TemplateCompileError::IncludeNotFound { .. })),
            "got {:?}",
            result
        );
    }

    #[test]
    fn test_include_invalid_syntax_unquoted() {
        // `{% include header.atl %}` (no quotes) is malformed.
        let (_tmp, dir) = temp_includes_dir();
        let mut resolver = IncludeResolver::single(dir);
        let result = TemplateCompiler::compile_with(
            r#"{% include header.atl %}"#,
            "outer",
            &mut resolver,
            CompileOptions::default(),
        );
        assert!(
            matches!(result, Err(TemplateCompileError::InvalidInclude { .. })),
            "got {:?}",
            result
        );
    }

    #[test]
    fn test_include_with_filter_chain_in_outer() {
        // The included template can use any built-in filter exposed by the
        // outer template's filter table. (No special wiring — the included
        // tokens get spliced into the same _ENV.)
        let (_tmp, dir) = temp_includes_dir();
        write_include(&dir, "row.atl", "{{ name }}");

        // We need a filters table here too, but the outer doesn't apply
        // filters. Just confirm the include can resolve __ctx.name.
        let result = render_with_includes(
            r#"{% include "row.atl" %}"#,
            dir,
            |_, ctx| {
                ctx.set("name", "Jimmie").unwrap();
            },
        )
        .unwrap();
        assert_eq!(result, "Jimmie");
    }

    // ---------- Lua-string-aware tokenizer ----------

    #[test]
    fn test_string_literal_emits_literal_delimiters() {
        // The motivating case: `{{ "{{ var }}" }}` produces `{{ var }}`
        let result = render_simple(
            r#"${{ "{{ github.event.inputs.x }}" }}"#,
            |_, _| {},
        );
        assert_eq!(result, "${{ github.event.inputs.x }}");
    }

    #[test]
    fn test_string_concat_emits_closing_braces() {
        let result = render_simple(
            r#"{{ "}" .. "}" }}"#,
            |_, _| {},
        );
        assert_eq!(result, "}}");
    }

    #[test]
    fn test_long_string_emits_raw_content() {
        let result = render_simple(
            "{{ [[ raw }} text ]] }}",
            |_, _| {},
        );
        assert_eq!(result, " raw }} text ");
    }

    // ---------- Built-in escape constants ----------

    #[test]
    fn test_builtin_left_expr_right_expr() {
        let result = render_simple("${{ LE }} var {{ RE }}", |_, _| {});
        assert_eq!(result, "${{ var }}");
    }

    #[test]
    fn test_builtin_left_stmt_right_stmt() {
        let result = render_simple("{{ LS }} if x then {{ RS }}", |_, _| {});
        assert_eq!(result, "{% if x then %}");
    }

    #[test]
    fn test_builtin_long_form_aliases() {
        let result = render_simple("{{ LEFT_EXPR }} v {{ RIGHT_EXPR }} and {{ LEFT_STMT }} s {{ RIGHT_STMT }}", |_, _| {});
        assert_eq!(result, "{{ v }} and {% s %}");
    }

    // ---------- {% raw %} / {% endraw %} ----------

    #[test]
    fn test_raw_block_renders_verbatim() {
        let result = render_simple(
            "before{% raw %}${{ github.event.inputs.x }}{% endraw %}after",
            |_, _| {},
        );
        assert_eq!(result, "before${{ github.event.inputs.x }}after");
    }

    #[test]
    fn test_raw_block_with_context_around_it() {
        let result = render_simple(
            "{{ name }} {% raw %}{{ not_evaluated }}{% endraw %} {{ name }}",
            |_, ctx| { ctx.set("name", "ok").unwrap(); },
        );
        assert_eq!(result, "ok {{ not_evaluated }} ok");
    }

    // ---------- Built-in escape constants ----------

    #[test]
    fn test_builtin_constants_mixed_with_context() {
        // Escape constants and context variables coexist: LE/RE produce
        // literal delimiters while context vars resolve normally.
        let result = render_simple(
            "{{ LE }} {{ name }} {{ RE }}",
            |_, ctx| { ctx.set("name", "project").unwrap(); },
        );
        assert_eq!(result, "{{ project }}");
    }
}

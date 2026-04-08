mod compiler;
mod error;
pub mod render;
mod tokenizer;

pub use error::TemplateCompileError;
pub use compiler::Compiler;
use tokenizer::Tokenizer;

/// A compiled template: Lua source code ready to be loaded into an mlua VM.
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    /// The Lua source code (a function definition).
    pub source: String,
    /// The template name (for error reporting).
    pub name: String,
}

/// Compiles Archetect Template Language (ATL) templates into Lua functions.
///
/// Templates use `{{ expr | filter }}` for interpolation and `{% lua_code %}`
/// for logic blocks. The compiled Lua function receives a context table and
/// a filters table, and returns the rendered string.
pub struct TemplateCompiler;

impl TemplateCompiler {
    /// Compile a template string into Lua source code.
    pub fn compile(template: &str, name: &str) -> Result<CompiledTemplate, TemplateCompileError> {
        let tokens = Tokenizer::tokenize(template)?;
        let source = Compiler::compile(&tokens);
        Ok(CompiledTemplate {
            source,
            name: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let result = TemplateCompiler::compile("Hello {{ name }}!", "test");
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.name, "test");
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
}

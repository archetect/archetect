use mlua::{Lua, Result as LuaResult, UserData, Value};

/// A single case expansion rule.
#[derive(Clone, Debug)]
pub enum CaseSpec {
    /// Auto-generate key and value in this case style.
    /// Key is derived from the prompt key, value from the input.
    Auto(CaseStyle),
    /// Fixed key name, value transformed with given style.
    Fixed { key: String, style: CaseStyle },
}

#[derive(Clone, Debug)]
pub enum CaseStyle {
    Snake,
    Pascal,
    Camel,
    Kebab,
    Train,
    Constant,
    Title,
    Lower,
    Upper,
    Sentence,
    Package,
    Directory,
    Cobol,
    Plural,
    Singular,
}

impl CaseStyle {
    pub fn transform_key(&self, input: &str) -> String {
        match self {
            CaseStyle::Snake => archetect_inflections::to_snake_case(input),
            CaseStyle::Pascal => archetect_inflections::to_pascal_case(input),
            CaseStyle::Camel => archetect_inflections::to_camel_case(input),
            CaseStyle::Kebab => archetect_inflections::to_kebab_case(input),
            CaseStyle::Train => archetect_inflections::to_train_case(input),
            CaseStyle::Constant => archetect_inflections::to_screaming_snake_case(input),
            CaseStyle::Title => archetect_inflections::to_title_case(input),
            CaseStyle::Lower => input.to_lowercase(),
            CaseStyle::Upper => input.to_uppercase(),
            CaseStyle::Sentence => archetect_inflections::to_sentence_case(input),
            CaseStyle::Package => archetect_inflections::to_package_case(input),
            CaseStyle::Directory => archetect_inflections::to_directory_case(input),
            CaseStyle::Cobol => archetect_inflections::to_cobol_case(input),
            CaseStyle::Plural => archetect_inflections::to_plural(input),
            CaseStyle::Singular => archetect_inflections::to_singular(input),
        }
    }

    pub fn transform_value(&self, input: &str) -> String {
        self.transform_key(input) // Same transform for values
    }
}

/// Expand a prompt key and value into multiple context entries based on case specs.
pub fn expand_cases(key: &str, value: &str, specs: &[CaseSpec]) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for spec in specs {
        match spec {
            CaseSpec::Auto(style) => {
                let new_key = style.transform_key(key);
                let new_value = style.transform_value(value);
                entries.push((new_key, new_value));
            }
            CaseSpec::Fixed { key: fixed_key, style } => {
                let new_value = style.transform_value(value);
                entries.push((fixed_key.clone(), new_value));
            }
        }
    }

    entries
}

/// The programming case preset: snake, pascal, camel, kebab, train, constant
pub fn programming_cases() -> Vec<CaseSpec> {
    vec![
        CaseSpec::Auto(CaseStyle::Snake),
        CaseSpec::Auto(CaseStyle::Pascal),
        CaseSpec::Auto(CaseStyle::Camel),
        CaseSpec::Auto(CaseStyle::Kebab),
        CaseSpec::Auto(CaseStyle::Train),
        CaseSpec::Auto(CaseStyle::Constant),
    ]
}

/// All available case styles
pub fn all_cases() -> Vec<CaseSpec> {
    vec![
        CaseSpec::Auto(CaseStyle::Snake),
        CaseSpec::Auto(CaseStyle::Pascal),
        CaseSpec::Auto(CaseStyle::Camel),
        CaseSpec::Auto(CaseStyle::Kebab),
        CaseSpec::Auto(CaseStyle::Train),
        CaseSpec::Auto(CaseStyle::Constant),
        CaseSpec::Auto(CaseStyle::Title),
        CaseSpec::Auto(CaseStyle::Lower),
        CaseSpec::Auto(CaseStyle::Upper),
        CaseSpec::Auto(CaseStyle::Sentence),
        CaseSpec::Auto(CaseStyle::Package),
        CaseSpec::Auto(CaseStyle::Directory),
        CaseSpec::Auto(CaseStyle::Cobol),
    ]
}

impl UserData for CaseStyle {}

/// Wrapper for a list of CaseSpecs, usable as Lua UserData
#[derive(Clone, Debug)]
pub struct CaseSpecList(pub Vec<CaseSpec>);

impl UserData for CaseSpecList {}

/// Wrapper for a single fixed CaseSpec, usable as Lua UserData
#[derive(Clone, Debug)]
pub struct CaseSpecEntry(pub CaseSpec);

impl UserData for CaseSpecEntry {}

/// Resolve a CaseStyle from a Case.* userdata value.
fn resolve_case_style(value: &Value) -> Result<CaseStyle, mlua::Error> {
    match value {
        Value::UserData(ud) => {
            ud.borrow::<CaseStyle>().map(|s| s.clone())
        }
        other => Err(mlua::Error::RuntimeError(format!(
            "Expected a Case style (e.g., Case.Snake, Case.Title), got {:?}",
            other
        ))),
    }
}

/// Register the Cases global table and Case constants in the Lua environment.
pub fn register_cases(lua: &Lua) -> LuaResult<()> {
    // -- Case constants (enum-style) --
    let case_table = lua.create_table()?;
    case_table.set("Snake", CaseStyle::Snake)?;
    case_table.set("Pascal", CaseStyle::Pascal)?;
    case_table.set("Camel", CaseStyle::Camel)?;
    case_table.set("Kebab", CaseStyle::Kebab)?;
    case_table.set("Train", CaseStyle::Train)?;
    case_table.set("Constant", CaseStyle::Constant)?;
    case_table.set("Title", CaseStyle::Title)?;
    case_table.set("Lower", CaseStyle::Lower)?;
    case_table.set("Upper", CaseStyle::Upper)?;
    case_table.set("Sentence", CaseStyle::Sentence)?;
    case_table.set("Package", CaseStyle::Package)?;
    case_table.set("Directory", CaseStyle::Directory)?;
    case_table.set("Cobol", CaseStyle::Cobol)?;
    case_table.set("Plural", CaseStyle::Plural)?;
    case_table.set("Singular", CaseStyle::Singular)?;
    lua.globals().set("Case", case_table)?;

    // -- Cases presets and constructors --
    let cases_table = lua.create_table()?;

    cases_table.set(
        "programming",
        lua.create_function(|_, ()| Ok(CaseSpecList(programming_cases())))?,
    )?;

    cases_table.set(
        "all",
        lua.create_function(|_, ()| Ok(CaseSpecList(all_cases())))?,
    )?;

    // Cases.set(Case.Snake, Case.Pascal, ...) or Cases.set("snake", "pascal", ...)
    cases_table.set(
        "set",
        lua.create_function(|_, args: mlua::Variadic<Value>| {
            let mut specs = Vec::new();
            for arg in args.iter() {
                let style = resolve_case_style(arg)?;
                specs.push(CaseSpec::Auto(style));
            }
            Ok(CaseSpecList(specs))
        })?,
    )?;

    // Cases.fixed("key", Case.Title) or Cases.fixed("key", "title")
    cases_table.set(
        "fixed",
        lua.create_function(|_, (key, style_value): (String, Value)| {
            let style = resolve_case_style(&style_value)?;
            Ok(CaseSpecEntry(CaseSpec::Fixed { key, style }))
        })?,
    )?;

    lua.globals().set("Cases", cases_table)?;
    Ok(())
}

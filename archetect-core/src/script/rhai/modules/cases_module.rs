use std::fmt::{Display, Formatter};

use cruet::case::to_case_snake_like;
use either::Either;
use rhai::plugin::*;
use rhai::{Dynamic, Map};

use CaseStrategy::{CasedIdentityCasedValue, CasedKeyCasedValue, FixedKeyCasedValue};

use crate::script::rhai::modules::cases_module::module::to_case;
use crate::script::rhai::modules::cases_module::CaseStrategy::FixedIdentityCasedValue;
use crate::script::rhai::modules::prompt_module::Caseable;

const LIST_DEFAULT_IDENTITY_KEY: &'static str = "item_name";

pub fn register(engine: &mut Engine) {
    let mut m = Module::new();
    m.set_var(
        "PROGRAMMING_CASES",
        vec![
            Dynamic::from(CaseStyle::CamelCase),
            Dynamic::from(CaseStyle::ConstantCase),
            Dynamic::from(CaseStyle::KebabCase),
            Dynamic::from(CaseStyle::PascalCase),
            Dynamic::from(CaseStyle::SnakeCase),
        ],
    );
    m.set_var(
        "PROGRAMMING_CASES_ALL",
        vec![
            Dynamic::from(CaseStyle::CamelCase),
            Dynamic::from(CaseStyle::CobolCase),
            Dynamic::from(CaseStyle::ConstantCase),
            Dynamic::from(CaseStyle::KebabCase),
            Dynamic::from(CaseStyle::PascalCase),
            Dynamic::from(CaseStyle::SnakeCase),
            Dynamic::from(CaseStyle::TrainCase),
        ],
    );
    engine.register_global_module(m.into());
    engine.register_global_module(exported_module!(module).into());
    engine.register_fn("camel_case", cruet::to_camel_case);
    engine.register_fn("class_case", cruet::to_class_case);
    engine.register_fn("cobol_case", to_cobol_case);
    engine.register_fn("constant_case", cruet::to_screaming_snake_case);
    engine.register_fn("directory_case", to_directory_case);
    engine.register_fn("kebab_case", cruet::to_kebab_case);
    engine.register_fn("lower_case", str::to_lowercase);
    engine.register_fn("package_case", to_package_case);
    engine.register_fn("pascal_case", cruet::to_pascal_case);
    engine.register_fn("snake_case", cruet::to_snake_case);
    engine.register_fn("sentence_case", cruet::to_sentence_case);
    engine.register_fn("title_case", cruet::to_title_case);
    engine.register_fn("train_case", cruet::to_train_case);
    engine.register_fn("upper_case", str::to_uppercase);

    engine.register_fn("pluralize", cruet::to_plural);
    engine.register_fn("plural", cruet::to_plural);
    engine.register_fn("singularize", cruet::to_singular);
    engine.register_fn("singular", cruet::to_singular);

    engine.register_fn("ordinalize", cruet::ordinalize);
    engine.register_fn("ordinalize", |value: i64| cruet::ordinalize(value.to_string().as_str()));
    engine.register_fn("deordinalize", cruet::deordinalize);
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum CaseStyle {
    CamelCase,
    ClassCase,
    CobolCase,
    ConstantCase,
    DirectoryCase,
    KebabCase,
    LowerCase,
    PackageCase,
    PascalCase,
    SentenceCase,
    SnakeCase,
    TitleCase,
    TrainCase,
    UpperCase,
}

impl Display for CaseStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CaseStyle::CamelCase => {
                write!(f, "CamelCase")
            }
            CaseStyle::ClassCase => {
                write!(f, "ClassCase")
            }
            CaseStyle::CobolCase => {
                write!(f, "CobolCase")
            }
            CaseStyle::ConstantCase => {
                write!(f, "ConstantCase")
            }
            CaseStyle::DirectoryCase => {
                write!(f, "DirectoryCase")
            }
            CaseStyle::KebabCase => {
                write!(f, "KebabCase")
            }
            CaseStyle::LowerCase => {
                write!(f, "LowerCase")
            }
            CaseStyle::PackageCase => {
                write!(f, "PackageCase")
            }
            CaseStyle::PascalCase => {
                write!(f, "PascalCase")
            }
            CaseStyle::SentenceCase => {
                write!(f, "SentenceCase")
            }
            CaseStyle::SnakeCase => {
                write!(f, "SnakeCase")
            }
            CaseStyle::TitleCase => {
                write!(f, "TitleCase")
            }
            CaseStyle::TrainCase => {
                write!(f, "TrainCase")
            }
            CaseStyle::UpperCase => {
                write!(f, "UpperCase")
            }
        }
    }
}

impl CaseStyle {
    pub fn to_case(&self, input: &str) -> String {
        match self {
            CaseStyle::CamelCase => cruet::to_camel_case(input),
            CaseStyle::ClassCase => cruet::to_class_case(input),
            CaseStyle::CobolCase => to_cobol_case(input),
            CaseStyle::ConstantCase => cruet::to_screaming_snake_case(input),
            CaseStyle::DirectoryCase => to_directory_case(input),
            CaseStyle::KebabCase => cruet::to_kebab_case(input),
            CaseStyle::LowerCase => str::to_lowercase(input),
            CaseStyle::PascalCase => cruet::to_pascal_case(input),
            CaseStyle::PackageCase => to_package_case(input),
            CaseStyle::SnakeCase => cruet::to_snake_case(input),
            CaseStyle::SentenceCase => cruet::to_sentence_case(input),
            CaseStyle::TitleCase => cruet::to_title_case(input),
            CaseStyle::TrainCase => cruet::to_train_case(input),
            CaseStyle::UpperCase => str::to_uppercase(input),
        }
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum CaseStrategy {
    CasedIdentityCasedValue { styles: Vec<CaseStyle> },
    CasedKeyCasedValue { key: String, styles: Vec<CaseStyle> },
    FixedIdentityCasedValue { style: CaseStyle },
    FixedKeyCasedValue { key: String, style: CaseStyle },
}

pub fn to_cobol_case(non_snake_case_string: &str) -> String {
    to_case_snake_like(non_snake_case_string, "-", "upper")
}

pub fn to_package_case(non_snake_case_string: &str) -> String {
    to_case_snake_like(non_snake_case_string, ".", "lower")
}

pub fn to_directory_case(non_snake_case_string: &str) -> String {
    to_case_snake_like(non_snake_case_string, "/", "lower")
}

pub fn extract_case_strategies(settings: &Map) -> Result<Either<Vec<CaseStrategy>, CaseStyle>, String> {
    let mut results = vec![];
    if let Some(specification) = extract_casing_specification(settings) {
        if let Some(style) = extract_case_style(specification) {
            return Ok(Either::Right(style));
        } else if let Some(strategy) = extract_case_strategy(specification) {
            results.push(strategy);
        } else if let Some(strategies) = specification.clone().try_cast::<Vec<Dynamic>>() {
            for strategy in strategies.into_iter() {
                if let Some(strategy) = strategy.clone().try_cast::<CaseStrategy>() {
                    results.push(strategy);
                } else {
                    let requirement = format!(
                        "an array of CaseStrategy elements, but contains {:?} ({})",
                        &strategy,
                        &strategy.type_name()
                    );
                    return Err(requirement);
                }
            }
        } else {
            return Err("an array of CaseStrategy elements, a single CaseStrategy, or a single CaseStyle".to_string());
        }
    }
    Ok(Either::Left(results))
}

pub fn extract_case_style(specification: &Dynamic) -> Option<CaseStyle> {
    if specification.is::<CaseStyle>() {
       return Some(specification.clone_cast());
    }
    None
}

pub fn extract_case_strategy(specification: &Dynamic) -> Option<CaseStrategy> {
    if specification.is::<CaseStrategy>() {
        return Some(specification.clone_cast());
    }
    None
}

fn extract_casing_specification(settings: &Map) -> Option<&Dynamic> {
    settings.get("cased_as")
        .or(settings.get("cased_with"))
        .or(settings.get("casing"))
        .or(settings.get("cases"))
}

pub fn expand_key_value_cases(
    strategy: &Either<Vec<CaseStrategy>, CaseStyle>,
    results: &mut Map,
    key: &str,
    caseable: Caseable,
) {
    match strategy {
        Either::Left(case_strategies) => {
            if case_strategies.len() == 0 {
                insert_keys_and_values_without_casing(results, key, caseable);
            } else {
                expand_keys_and_values_with_case_strategies(case_strategies, results, key, caseable);
            }
        }
        Either::Right(case_style) => {
            expand_keys_and_values_with_case_style(case_style, results, key, caseable);
        }
    }
}

fn insert_keys_and_values_without_casing(results: &mut Map, key: &str, caseable: Caseable) {
    match caseable {
        Caseable::String(value) => {
            results.insert(key.into(), value.into());
        }
        Caseable::List(list) => {
            let dynamic_list = list
                .clone()
                .into_iter()
                .map(|v| Dynamic::from(v))
                .collect::<Vec<Dynamic>>();
            results.insert(key.into(), dynamic_list.into());
        }
        Caseable::Opaque(value) => {
            // Don't case anything.  Whatever was passed in is not caseable.
            results.insert(key.into(), value.clone_cast());
        }
    }
}

fn expand_keys_and_values_with_case_style(case_style: &CaseStyle, results: &mut Map, key: &str, caseable: Caseable) {
    // A single CaseStyle was provided.  Only apply casing to scalar values and each item in a list
    match caseable {
        Caseable::String(value) => {
            results.insert(key.into(), case_style.to_case(&value).into());
        }
        Caseable::List(list) => {
            let dynamic_list = list
                .clone()
                .into_iter()
                .map(|item| case_style.to_case(&item))
                .map(|v| Dynamic::from(v))
                .collect::<Vec<Dynamic>>();
            results.insert(key.into(), dynamic_list.into());
        }
        Caseable::Opaque(value) => {
            // Don't case anything.  Whatever was passed in is not caseable.
            results.insert(key.into(), value.clone_cast());
        }
    }
}


fn expand_keys_and_values_with_case_strategies(
    case_strategies: &Vec<CaseStrategy>,
    results: &mut Map,
    key: &str,
    caseable: Caseable,
) {
    for strategy in case_strategies {
        match strategy {
            CasedIdentityCasedValue { styles } => {
                for style in styles {
                    match &caseable {
                        Caseable::String(value) => {
                            results.insert(
                                style.to_case(key.to_string().as_str()).into(),
                                style.to_case(&value).into(),
                            );
                        }
                        Caseable::List(list) => {
                            let mut item_list = vec![];
                            for item in list {
                                let mut item_map = Map::new();
                                expand_keys_and_values_with_case_strategies(
                                    &case_strategies,
                                    &mut item_map,
                                    LIST_DEFAULT_IDENTITY_KEY,
                                    Caseable::String(item.clone()),
                                );
                                item_list.push(item_map);
                            }
                            results.insert(key.into(), item_list.into());
                        }
                        Caseable::Opaque(value) => {
                            results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                        }
                    }
                }
            }
            CasedKeyCasedValue { key: item_key, styles } => {
                for style in styles {
                    match &caseable {
                        Caseable::String(value) => {
                            results.insert(
                                style.to_case(item_key.to_string().as_str()).into(),
                                style.to_case(value).into(),
                            );
                        }
                        Caseable::List(list) => {
                            let mut item_list = vec![];
                            for item in list {
                                let mut item_map = Map::new();
                                expand_keys_and_values_with_case_strategies(
                                    &case_strategies,
                                    &mut item_map,
                                    item_key,
                                    Caseable::String(item.clone()),
                                );
                                item_list.push(item_map);
                            }
                            results.insert(key.into(), item_list.into());
                        }
                        Caseable::Opaque(value) => {
                            results.insert(style.to_case(item_key.to_string().as_str()).into(), value.clone_cast());
                        }
                    }
                }
            }
            FixedIdentityCasedValue { style } => match &caseable {
                Caseable::String(value) => {
                    results.insert(key.into(), style.to_case(value).into());
                }
                Caseable::List(list) => {
                    let mut item_list = vec![];
                    for item in list {
                        let mut item_map = Map::new();
                        expand_keys_and_values_with_case_strategies(
                            &case_strategies,
                            &mut item_map,
                            LIST_DEFAULT_IDENTITY_KEY,
                            Caseable::String(item.clone()),
                        );
                        item_list.push(item_map);
                    }
                    results.insert(key.into(), item_list.into());
                }
                Caseable::Opaque(value) => {
                    results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                }
            },
            FixedKeyCasedValue { key: item_key, style } => match &caseable {
                Caseable::String(value) => {
                    results.insert(item_key.into(), to_case(value, *style).into());
                }
                Caseable::List(list) => {
                    let mut item_list = vec![];
                    for item in list {
                        let mut item_map = Map::new();
                        expand_keys_and_values_with_case_strategies(
                            &case_strategies,
                            &mut item_map,
                            item_key,
                            Caseable::String(item.clone()),
                        );
                        item_list.push(item_map);
                    }
                    results.insert(key.into(), item_list.into());
                }
                Caseable::Opaque(value) => {
                    results.insert(style.to_case(item_key.to_string().as_str()).into(), value.clone_cast());
                }
            },
        }
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use log::warn;
    use rhai::Dynamic;

    pub type CaseStyle = super::CaseStyle;
    pub type CaseStrategy = super::CaseStrategy;

    pub const CamelCase: CaseStyle = CaseStyle::CamelCase;
    pub const ClassCase: CaseStyle = CaseStyle::ClassCase;
    pub const CobolCase: CaseStyle = CaseStyle::CobolCase;
    pub const ConstantCase: CaseStyle = CaseStyle::ConstantCase;
    pub const DirectoryCase: CaseStyle = CaseStyle::DirectoryCase;
    pub const KebabCase: CaseStyle = CaseStyle::KebabCase;
    pub const LowerCase: CaseStyle = CaseStyle::LowerCase;
    pub const PackageCase: CaseStyle = CaseStyle::PackageCase;
    pub const PascalCase: CaseStyle = CaseStyle::PascalCase;
    pub const SentenceCase: CaseStyle = CaseStyle::SentenceCase;
    pub const SnakeCase: CaseStyle = CaseStyle::SnakeCase;
    pub const TitleCase: CaseStyle = CaseStyle::TitleCase;
    pub const TrainCase: CaseStyle = CaseStyle::TrainCase;
    pub const UpperCase: CaseStyle = CaseStyle::UpperCase;

    pub fn CasedIdentityCasedValue(styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CasedIdentityCasedValue { styles }
    }

    /// An alias for 'CasedIdentityCasedValue'
    pub fn CasedIdentityAndValue(styles: Vec<Dynamic>) -> CaseStrategy {
        CasedIdentityCasedValue(styles)
    }

    pub fn CasedIdentity(styles: Vec<Dynamic>) -> CaseStrategy {
        warn!("'CasedIdentity' has been deprecated.  Please use 'CasedIdentityCasedValue' instead.");
        CasedIdentityCasedValue(styles)
    }

    pub fn CasedKeyCasedValue(key: String, styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CasedKeyCasedValue { key, styles }
    }

    /// An alias for 'CasedKeyCasedValue'
    pub fn CasedKeyAndValue(key: String, styles: Vec<Dynamic>) -> CaseStrategy {
        CasedKeyCasedValue(key, styles)
    }

    pub fn CasedKeys(key: String, styles: Vec<Dynamic>) -> CaseStrategy {
        warn!("'CasedKeys' has been deprecated.  Please use 'CasedKeyAndValue' instead.");
        CasedKeyCasedValue(key, styles)
    }

    pub fn FixedIdentityCasedValue(style: CaseStyle) -> CaseStrategy {
        CaseStrategy::FixedIdentityCasedValue { style }
    }

    /// Alias to FixedIdentityCasedValue
    pub fn CasedValue(style: CaseStyle) -> CaseStrategy {
        FixedIdentityCasedValue(style)
    }

    pub fn FixedIdentity(style: CaseStyle) -> CaseStrategy {
        warn!("'FixedIdentity' has been deprecated.  Please use 'FixedIdentityCasedValue' instead.");
        FixedIdentityCasedValue(style)
    }

    pub fn FixedKeyCasedValue(key: String, style: CaseStyle) -> CaseStrategy {
        FixedKeyCasedValue { key, style }
    }

    pub fn FixedKey(key: String, style: CaseStyle) -> CaseStrategy {
        warn!("'FixedKey' has been deprecated.  Please use 'FixedKeyCasedValue' instead.");
        FixedKeyCasedValue(key, style)
    }

    pub fn to_case(input: &str, style: CaseStyle) -> String {
        style.to_case(input)
    }
}

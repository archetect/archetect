use std::fmt::{Display, Formatter};

use cruet::case::to_case_snake_like;
use rhai::{Dynamic, Map};
use rhai::plugin::*;

use crate::script::rhai::modules::cases_module::module::to_case;
use crate::script::rhai::modules::prompt_module::Caseable;

pub fn register(engine: &mut Engine) {
    let mut m = Module::new();
    m.set_var(
        "PROGRAMMING_CASES",
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

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
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

pub fn expand_key_value_cases(settings: &Map, results: &mut Map, key: &str, value: Caseable) {
    if let Some(strategies) = settings.get("cased_as").or(settings.get("cases")) {
        let maybe_strategies: Option<Vec<Dynamic>> = strategies.clone().try_cast::<Vec<Dynamic>>();
        if let Some(strategies) = maybe_strategies {
            let strategies = strategies
                .into_iter()
                .filter_map(|style| style.try_cast::<CaseStrategy>())
                .collect::<Vec<CaseStrategy>>();
            for strategy in strategies {
                match strategy {
                    CaseStrategy::CasedIdentityCasedValue { styles } => {
                        for style in styles {
                            match &value {
                                Caseable::String(value) => {
                                    results.insert(
                                        style.to_case(key.to_string().as_str()).into(),
                                        style.to_case(&value).into(),
                                    );
                                }
                                Caseable::List(list) => {
                                    let value = list
                                        .into_iter()
                                        .map(|v| style.to_case(&v))
                                        .map(|v| Dynamic::from(v))
                                        .collect::<Vec<Dynamic>>();
                                    results.insert(style.to_case(key.to_string().as_str()).into(), value.into());
                                }
                                Caseable::Opaque(value) => {
                                    results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                                }
                            }
                        }
                    }
                    CaseStrategy::CasedKeyCasedValue { key, styles } => {
                        for style in styles {
                            match &value {
                                Caseable::String(value) => {
                                    results.insert(
                                        style.to_case(key.to_string().as_str()).into(),
                                        style.to_case(value).into(),
                                    );
                                }
                                Caseable::List(list) => {
                                    let value = list
                                        .into_iter()
                                        .map(|v| style.to_case(&v))
                                        .map(|v| Dynamic::from(v))
                                        .collect::<Vec<Dynamic>>();
                                    results.insert(style.to_case(key.to_string().as_str()).into(), value.into());
                                }
                                Caseable::Opaque(value) => {
                                    results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                                }
                            }
                        }
                    }
                    CaseStrategy::FixedIdentityCasedValue { style } => match &value {
                        Caseable::String(value) => {
                            results.insert(key.into(), to_case(value, style).into());
                        }
                        Caseable::List(list) => {
                            let value = list
                                .into_iter()
                                .map(|v| style.to_case(&v))
                                .map(|v| Dynamic::from(v))
                                .collect::<Vec<Dynamic>>();
                            results.insert(style.to_case(key.to_string().as_str()).into(), value.into());
                        }
                        Caseable::Opaque(value) => {
                            results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                        }
                    },
                    CaseStrategy::FixedKeyCasedValue { key, style } => match &value {
                        Caseable::String(value) => {
                            results.insert(key.into(), to_case(value, style).into());
                        }
                        Caseable::List(list) => {
                            let value = list
                                .into_iter()
                                .map(|v| style.to_case(&v))
                                .map(|v| Dynamic::from(v))
                                .collect::<Vec<Dynamic>>();
                            results.insert(style.to_case(key.to_string().as_str()).into(), value.into());
                        }
                        Caseable::Opaque(value) => {
                            results.insert(style.to_case(key.to_string().as_str()).into(), value.clone_cast());
                        }
                    },
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use log::warn;
    use rhai::{Dynamic};

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
        CaseStrategy::CasedIdentityCasedValue { styles }
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
        CaseStrategy::CasedKeyCasedValue { key, styles }
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
        CaseStrategy::FixedKeyCasedValue { key, style }
    }

    pub fn FixedKey(key: String, style: CaseStyle) -> CaseStrategy {
        warn!("'FixedKey' has been deprecated.  Please use 'FixedKeyCasedValue' instead.");
        FixedKeyCasedValue(key, style)
    }

    pub fn to_case(input: &str, style: CaseStyle) -> String {
        style.to_case(input)
    }
}
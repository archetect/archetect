use crate::v2::script::rhai::modules::cases::module::to_case;
use cruet::case::to_case_snake_like;
use rhai::plugin::*;
use rhai::Map;

pub fn register(engine: &mut Engine) {
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
    CasedIdentity { styles: Vec<CaseStyle> },
    CasedKeys { key: String, styles: Vec<CaseStyle> },
    CasedKeysWithSuffix { suffix: String, styles: Vec<CaseStyle> },
    CasedKeysWithPrefix { prefix: String, styles: Vec<CaseStyle> },
    CasedValue { style: CaseStyle },
    FixedIdentity { style: CaseStyle },
    FixedKey { key: String, style: CaseStyle },
    FixedKeyWithSuffix { suffix: String, style: CaseStyle },
    FixedKeyWithPrefix { prefix: String, style: CaseStyle },
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

pub fn expand_cases(settings: &Map, results: &mut Map, key: &str, value: &str) {
    if let Some(strategies) = settings.get("cases") {
        let maybe_strategies: Option<Vec<Dynamic>> = strategies.clone().try_cast::<Vec<Dynamic>>();
        if let Some(strategies) = maybe_strategies {
            let strategies = strategies
                .into_iter()
                .filter_map(|style| style.try_cast::<CaseStrategy>())
                .collect::<Vec<CaseStrategy>>();
            for strategy in strategies {
                match strategy {
                    CaseStrategy::CasedIdentity { styles } => {
                        for style in styles {
                            results.insert(
                                to_case(key.to_string().as_str(), style.clone()).into(),
                                to_case(value, style).into(),
                            );
                        }
                    }
                    CaseStrategy::CasedKeysWithSuffix { suffix, styles } => {
                        for style in styles {
                            results.insert(
                                to_case(format!("{}-{}", key, suffix).as_str(), style.clone()).into(),
                                to_case(value, style).into(),
                            );
                        }
                    }
                    CaseStrategy::CasedKeysWithPrefix { prefix, styles } => {
                        for style in styles {
                            results.insert(
                                to_case(format!("{}-{}", prefix, key).as_str(), style.clone()).into(),
                                to_case(value, style).into(),
                            );
                        }
                    }
                    CaseStrategy::CasedKeys { key, styles } => {
                        for style in styles {
                            results.insert(
                                to_case(key.as_str(), style.clone()).into(),
                                to_case(value, style).into(),
                            );
                        }
                    }
                    CaseStrategy::FixedIdentity { style } => {
                        results.insert(key.into(), to_case(value, style).into());
                    }
                    CaseStrategy::CasedValue { style } => {
                        results.insert(key.into(), to_case(value, style).into());
                    }
                    CaseStrategy::FixedKey { key, style } => {
                        results.insert(key.into(), to_case(value, style).into());
                    }
                    CaseStrategy::FixedKeyWithSuffix { suffix, style } => {
                        results.insert(
                            format!("{}{}", key, suffix).as_str().into(),
                            to_case(value, style).into(),
                        );
                    }
                    CaseStrategy::FixedKeyWithPrefix { prefix, style } => {
                        results.insert(
                            format!("{}{}", prefix, key).as_str().into(),
                            to_case(value, style).into(),
                        );
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use rhai::{Dynamic, Map};

    pub type CaseStyle = crate::v2::script::rhai::modules::cases::CaseStyle;
    pub type CaseStrategy = crate::v2::script::rhai::modules::cases::CaseStrategy;

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

    pub fn CasedIdentity(styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CaseStrategy::CasedIdentity { styles }
    }

    pub fn CasedKeys(key: String, styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CaseStrategy::CasedKeys { key, styles }
    }

    pub fn CasedKeysWithSuffix(suffix: String, styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CaseStrategy::CasedKeysWithSuffix {
            suffix: suffix.to_string(),
            styles,
        }
    }

    pub fn CasedKeysWithPrefix(prefix: String, styles: Vec<Dynamic>) -> CaseStrategy {
        let styles = styles
            .into_iter()
            .filter_map(|style| style.try_cast::<CaseStyle>())
            .collect::<Vec<CaseStyle>>();
        CaseStrategy::CasedKeysWithPrefix {
            prefix: prefix.to_string(),
            styles,
        }
    }

    pub fn FixedIdentity(style: CaseStyle) -> CaseStrategy {
        CaseStrategy::FixedIdentity { style }
    }

    pub fn CasedValue(style: CaseStyle) -> CaseStrategy {
        CaseStrategy::CasedValue { style }
    }

    pub fn FixedKey(key: String, style: CaseStyle) -> CaseStrategy {
        CaseStrategy::FixedKey { key, style }
    }

    pub fn FixedKeyWithSuffix(suffix: String, style: CaseStyle) -> CaseStrategy {
        CaseStrategy::FixedKeyWithSuffix { suffix, style }
    }

    pub fn FixedKeyWithPrefix(prefix: String, style: CaseStyle) -> CaseStrategy {
        CaseStrategy::FixedKeyWithPrefix { prefix, style }
    }

    pub fn to_case(input: &str, style: CaseStyle) -> String {
        style.to_case(input)
    }

    pub fn all_cases() -> Map {
        let mut results = Map::new();
        results.insert("CamelCase".into(), Dynamic::from(CaseStyle::CamelCase));
        results.insert("ClassCase".into(), Dynamic::from(CaseStyle::ClassCase));
        results.insert("CobolCase".into(), Dynamic::from(CaseStyle::CobolCase));
        results.insert("ConstantCase".into(), Dynamic::from(CaseStyle::ConstantCase));
        results.insert("DirectoryCase".into(), Dynamic::from(CaseStyle::DirectoryCase));
        results.insert("KebabCase".into(), Dynamic::from(CaseStyle::KebabCase));
        results.insert("LowerCase".into(), Dynamic::from(CaseStyle::LowerCase));
        results.insert("PascalCase".into(), Dynamic::from(CaseStyle::PascalCase));
        results.insert("PackageCase".into(), Dynamic::from(CaseStyle::PackageCase));
        results.insert("SnakeCase".into(), Dynamic::from(CaseStyle::SnakeCase));
        results.insert("SentenceCase".into(), Dynamic::from(CaseStyle::SentenceCase));
        results.insert("TitleCase".into(), Dynamic::from(CaseStyle::TitleCase));
        results.insert("TrainCase".into(), Dynamic::from(CaseStyle::TrainCase));
        results.insert("UpperCase".into(), Dynamic::from(CaseStyle::UpperCase));
        results
    }
}

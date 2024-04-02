#![deny(
    warnings,
    unused_variables,
    missing_docs,
    unsafe_code,
    unused_extern_crates
)]
#![cfg_attr(feature = "unstable", feature(test))]

//! Adds String based inflections for Rust. Snake, kebab, train, camel,
//! sentence, class, and title cases as well as ordinalize,
//! deordinalize, demodulize, deconstantize, and foreign key are supported as
//! both traits and pure functions acting on String types.
//! ```rust
//! use archetect_inflections::Inflector;
//! let camel_case_string: String = "some_string".to_camel_case();
//! let is_camel_cased: bool= camel_case_string.is_camel_case();
//! assert!(is_camel_cased == true);
//! ```

/// Provides case inflections
/// - Camel case
/// - Class case
/// - Kebab case
/// - Train case
/// - Screaming snake case
/// - Table case
/// - Sentence case
/// - Snake case
/// - Pascal case
pub mod case;
/// Provides number inflections
/// - Ordinalize
/// - Deordinalize
pub mod number;
/// Provides string inflections
/// - Deconstantize
/// - Demodulize
/// - Pluralize
/// - Singularize
pub mod string;

pub use case::class::is_class_case;
pub use case::class::to_class_case;

pub use case::camel::is_camel_case;
pub use case::camel::to_camel_case;

pub use case::pascal::is_pascal_case;
pub use case::pascal::to_pascal_case;

pub use case::snake::is_snake_case;
pub use case::snake::to_snake_case;

pub use case::screaming_snake::is_screaming_snake_case;
pub use case::screaming_snake::to_screaming_snake_case;

pub use case::kebab::is_kebab_case;
pub use case::kebab::to_kebab_case;

pub use case::cobol::is_cobol_case;
pub use case::cobol::to_cobol_case;

pub use case::train::is_train_case;
pub use case::train::to_train_case;

pub use case::sentence::is_sentence_case;
pub use case::sentence::to_sentence_case;

pub use case::directory::is_directory_case;
pub use case::directory::to_directory_case;

pub use case::package::is_package_case;
pub use case::package::to_package_case;

pub use case::title::is_title_case;
pub use case::title::to_title_case;

pub use case::table::is_table_case;
pub use case::table::to_table_case;

pub use number::deordinalize::deordinalize;
pub use number::ordinalize::ordinalize;

pub use string::deconstantize::deconstantize;
pub use string::demodulize::demodulize;
pub use string::pluralize::to_plural;
pub use string::singularize::to_singular;

#[allow(missing_docs)]
pub trait Inflector {
    fn to_camel_case(&self) -> String;
    fn is_camel_case(&self) -> bool;

    fn to_pascal_case(&self) -> String;
    fn is_pascal_case(&self) -> bool;

    fn to_snake_case(&self) -> String;
    fn is_snake_case(&self) -> bool;

    fn to_screaming_snake_case(&self) -> String;
    fn is_screaming_snake_case(&self) -> bool;

    fn to_kebab_case(&self) -> String;
    fn is_kebab_case(&self) -> bool;

    fn to_train_case(&self) -> String;
    fn is_train_case(&self) -> bool;

    fn to_sentence_case(&self) -> String;
    fn is_sentence_case(&self) -> bool;

    fn to_title_case(&self) -> String;
    fn is_title_case(&self) -> bool;

    fn ordinalize(&self) -> String;
    fn deordinalize(&self) -> String;

    fn demodulize(&self) -> String;

    fn deconstantize(&self) -> String;

    fn to_class_case(&self) -> String;

    fn is_class_case(&self) -> bool;

    fn to_table_case(&self) -> String;

    fn is_table_case(&self) -> bool;

    fn to_plural(&self) -> String;

    fn to_singular(&self) -> String;
}

#[allow(missing_docs)]
pub trait InflectorNumbers {
    fn ordinalize(&self) -> String;
}

macro_rules! define_implementations {
    ( $slf:ident; $($imp_trait:ident => $typ:ident), *) => {
        $(
            #[inline]
            fn $imp_trait(&$slf) -> $typ {
                $imp_trait($slf)
            }
        )*
    }
}

macro_rules! define_number_implementations {
    ( $slf:ident; $($imp_trait:ident => $typ:ident), *) => {
        $(
            #[inline]
            fn $imp_trait(&$slf) -> $typ {
                $imp_trait(&$slf.to_string())
            }
        )*
    }
}

macro_rules! define_gated_implementations {
    ( $slf:ident; $($imp_trait:ident => $typ:ident), *) => {
        $(
            #[inline]

            fn $imp_trait(&$slf) -> $typ {
                $imp_trait($slf)
            }
        )*
    }
}

macro_rules! implement_string_for {
    ( $trt:ident; $($typ:ident), *) => {
        $(
            impl $trt for $typ {
                define_implementations![self;
                    to_camel_case => String,
                    is_camel_case => bool,
                    to_pascal_case => String,
                    is_pascal_case => bool,
                    to_screaming_snake_case => String,
                    is_screaming_snake_case => bool,
                    to_snake_case => String,
                    is_snake_case => bool,
                    to_kebab_case => String,
                    is_kebab_case => bool,
                    to_train_case => String,
                    is_train_case => bool,
                    to_sentence_case => String,
                    is_sentence_case => bool,
                    to_title_case => String,
                    is_title_case => bool,
                    ordinalize => String,
                    deordinalize => String
                ];
                define_gated_implementations![self;
                    to_class_case => String,
                    is_class_case => bool,
                    to_table_case => String,
                    is_table_case => bool,
                    to_plural => String,
                    to_singular => String,
                    demodulize => String,
                    deconstantize => String
                ];
            }
        )*
    }
}

macro_rules! implement_number_for {
    ( $trt:ident; $($typ:ident), *) => {
        $(
            impl $trt for $typ {
                define_number_implementations![self;
                    ordinalize => String
                ];
            }
        )*
    }
}

implement_string_for![
    Inflector;
    String, str
];

implement_number_for![
    InflectorNumbers;
    i8, i16, i32, i64, u8, u16, u32, u64, isize, usize, f32, f64
];

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;
    use Inflector;

    macro_rules! benchmarks {
        ( $($test_name:ident => $imp_trait:ident => $to_cast:expr), *) => {
            $(
                #[bench]
                fn $test_name(b: &mut Bencher) {
                    b.iter(|| {
                        $to_cast.$imp_trait()
                    });
                }
            )*
        }
    }

    benchmarks![
        benchmark_str_to_camel => to_camel_case => "foo_bar",
        benchmark_str_is_camel => is_camel_case => "fooBar",
        benchmark_str_to_screaming_snake => to_screaming_snake_case => "fooBar",
        benchmark_str_is_screaming_snake => is_screaming_snake_case => "FOO_BAR",
        benchmark_str_to_snake => to_snake_case => "fooBar",
        benchmark_str_is_snake => is_snake_case => "foo_bar",
        benchmark_str_to_kebab => to_kebab_case => "fooBar",
        benchmark_str_is_kebab => is_kebab_case => "foo-bar",
        benchmark_str_to_train => to_train_case => "fooBar",
        benchmark_str_is_train => is_train_case => "Foo-Bar",
        benchmark_str_to_sentence => to_sentence_case => "fooBar",
        benchmark_str_is_sentence => is_sentence_case => "Foo bar",
        benchmark_str_to_title => to_title_case => "fooBar",
        benchmark_str_is_title => is_title_case => "Foo Bar",
        benchmark_str_ordinalize  => ordinalize => "1",
        benchmark_str_deordinalize  => deordinalize => "1st",
        benchmark_string_to_camel => to_camel_case => "foo_bar".to_string(),
        benchmark_string_is_camel => is_camel_case => "fooBar".to_string(),
        benchmark_string_to_screaming_snake => to_screaming_snake_case => "fooBar".to_string(),
        benchmark_string_is_screaming_snake => is_screaming_snake_case => "FOO_BAR".to_string(),
        benchmark_string_to_snake => to_snake_case => "fooBar".to_string(),
        benchmark_string_is_snake => is_snake_case => "foo_bar".to_string(),
        benchmark_string_to_kebab => to_kebab_case => "fooBar".to_string(),
        benchmark_string_is_kebab => is_kebab_case => "foo-bar".to_string(),
        benchmark_string_to_train => to_train_case => "fooBar".to_string(),
        benchmark_string_is_train => is_train_case => "Foo-Bar".to_string(),
        benchmark_string_to_sentence => to_sentence_case => "fooBar".to_string(),
        benchmark_string_is_sentence => is_sentence_case => "Foo bar".to_string(),
        benchmark_string_to_title => to_title_case => "fooBar".to_string(),
        benchmark_string_is_title => is_title_case => "Foo Bar".to_string(),
        benchmark_string_ordinalize  => ordinalize => "1".to_string(),
        benchmark_string_deordinalize  => deordinalize => "1st".to_string(),
        benchmark_string_to_foreign_key => to_foreign_key => "Foo::Bar".to_string(),
        benchmark_string_is_foreign_key => is_foreign_key => "bar_id".to_string()
    ];

    benchmarks![
        benchmark_str_to_class => to_class_case => "foo",
        benchmark_str_is_class => is_class_case => "Foo",
        benchmark_str_to_table => to_table_case => "fooBar",
        benchmark_str_is_table => is_table_case => "foo_bars",
        benchmark_str_pluralize => to_plural => "crate",
        benchmark_str_singular => to_singular => "crates",
        benchmark_string_to_class => to_class_case => "foo".to_string(),
        benchmark_string_is_class => is_class_case => "Foo".to_string(),
        benchmark_string_to_table => to_table_case => "fooBar".to_string(),
        benchmark_string_is_table => is_table_case => "foo_bars".to_string(),
        benchmark_string_pluralize => to_plural => "crate".to_string(),
        benchmark_string_singular => to_singular => "crates".to_string(),
        benchmark_string_demodulize => demodulize => "Foo::Bar".to_string(),
        benchmark_string_deconstantize => deconstantize => "Foo::Bar".to_string(),
        benchmark_str_demodulize => demodulize => "Foo::Bar",
        benchmark_str_deconstantize => deconstantize => "Foo::Bar"
    ];
}

//! Collection of functions that make things a little less verbose.

use std::{error::Error, fmt::Display, str::FromStr};
use crate::vendor::read_input::test_generators::InsideFunc;
use crate::vendor::read_input::{InputBuilder, InputBuild};

/// Shortcut function. Fetches input that is validated with a test function.
pub fn valid_input<T, F>(test: F) -> T
where
    T: FromStr,
    F: Fn(&T) -> bool + 'static,
{
    input().add_test(test).get()
}

/// Shortcut function. Fetches input that is within a range, array or vector.
pub fn input_inside<T, U>(constraint: U) -> T
where
    T: FromStr,
    U: InsideFunc<T>,
{
    input().inside(constraint).get()
}

/// Shortcut function. Fetches input that is valid for whatever type needed.
pub fn simple_input<T: FromStr>() -> T {
    input().get()
}

/// Creates a new instance of `InputBuilder` with generic, minimal settings.
pub fn input<T: FromStr>() -> InputBuilder<T> {
    InputBuilder::new()
}

/// Creates a new instance of `InputBuilder` with settings specifically
/// tailored to the type you want.
pub fn input_d<T: DefaultBuilderSettings>() -> InputBuilder<T> {
    T::settings()
}

/// Trait for describing specifically tailored input settings for types.
pub trait DefaultBuilderSettings: FromStr {
    /// Returns tailored `InputBuilder`.
    fn settings() -> InputBuilder<Self>;
}

impl DefaultBuilderSettings for bool {
    fn settings() -> InputBuilder<Self> {
        input()
            .repeat_msg("Please input true or false: ")
            .err("Only type true or false.")
    }
}

impl DefaultBuilderSettings for char {
    fn settings() -> InputBuilder<Self> {
        input()
            .repeat_msg("Please input a character: ")
            .err("Only type a single character.")
    }
}

macro_rules! impl_default_builder_for_int {
    ($($t:ty),*) => {$(
    impl DefaultBuilderSettings for $t {
        fn settings() -> InputBuilder<Self> {
            input()
                .repeat_msg("Please input an integer: ")
                .err("Only type integers.")
        }
    }
    )*}
}

impl_default_builder_for_int! { i8, i16, i32, i64, i128, isize }

macro_rules! impl_default_builder_for_whole {
    ($($t:ty),*) => {$(
    impl DefaultBuilderSettings for $t {
        fn settings() -> InputBuilder<Self> {
            input()
                .repeat_msg("Please input a positive integer: ")
                .err("Only type positive integers.")
        }
    }
    )*}
}

impl_default_builder_for_whole! { u8, u16, u32, u64, u128, usize }

macro_rules! impl_default_builder_for_float {
    ($($t:ty),*) => {$(
    impl DefaultBuilderSettings for $t {
        fn settings() -> InputBuilder<Self> {
            input()
                .repeat_msg("Please input a number: ")
                .err("Only type numbers or decimal point.")
        }
    }
    )*}
}

impl_default_builder_for_float! { f32, f64 }

/// Produces an error message from an error type. Made for use in `.err_match()`
pub fn with_display<T: Display>(x: &T) -> Option<String> {
    Some(format!("Error: \"{}\"", x))
}

#[deprecated(
    since = "0.8.4",
    note = "Deprecated due to the depreciation of `std::error::Error::description`. Please use the `with_display` function instead."
)]
#[allow(deprecated)]
/// Produces an error message from an error type. Made for use in `.err_match()`
pub fn with_description<T: Error>(x: &T) -> Option<String> {
    Some(format!("Error: \"{}\"", (*x).description()))
}

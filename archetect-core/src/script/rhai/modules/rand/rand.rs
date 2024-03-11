#[allow(unused_imports)]
use rhai::plugin::*;

#[export_module]
pub mod rand_functions {
    use rand::prelude::*;
    use rhai::{EvalAltResult, Position, INT};
    use std::ops::{Range, RangeInclusive};

    #[cfg(feature = "float")]
    use rhai::FLOAT;

    #[cfg(feature = "decimal")]
    use rust_decimal::Decimal;

    /// Generate a random boolean value.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let decision = rand_bool();
    ///
    /// if decision {
    ///     print("You hit the Jackpot!")
    /// }
    /// ```
    pub fn rand_bool() -> bool {
        rand::random()
    }

    /// Generate a random boolean value with a probability of being `true`.
    /// Requires the `float` feature.
    ///
    /// `probability` must be between `0.0` and `1.0` (inclusive).
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let decision = rand_bool(0.01);  // 1% probability
    ///
    /// if decision {
    ///     print("You hit the Jackpot!")
    /// }
    /// ```
    #[cfg(feature = "float")]
    #[rhai_fn(name = "rand_bool", return_raw)]
    pub fn rand_bool_with_probability(probability: FLOAT) -> Result<bool, Box<EvalAltResult>> {
        if probability < 0.0 || probability > 1.0 {
            Err(EvalAltResult::ErrorArithmetic(
                format!(
                    "Invalid probability (must be between 0.0 and 1.0): {}",
                    probability
                ),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_bool(probability as f64))
        }
    }

    /// Generate a random integer number.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand();
    ///
    /// print(`I'll give you a random number: ${number}`);
    /// ```
    pub fn rand() -> INT {
        rand::random()
    }

    /// Generate a random integer number within an exclusive range.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand(18..39);
    ///
    /// print(`I'll give you a random number between 18 and 38: ${number}`);
    /// ```
    #[rhai_fn(name = "rand", return_raw)]
    pub fn rand_exclusive_range(range: Range<INT>) -> Result<INT, Box<EvalAltResult>> {
        if range.is_empty() {
            Err(EvalAltResult::ErrorArithmetic(
                format!("Range is empty: {:?}", range),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_range(range))
        }
    }

    /// Generate a random integer number within an inclusive range.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand(18..=38);
    ///
    /// print(`I'll give you a random number between 18 and 38: ${number}`);
    /// ```
    #[rhai_fn(name = "rand", return_raw)]
    pub fn rand_inclusive_range(range: RangeInclusive<INT>) -> Result<INT, Box<EvalAltResult>> {
        if range.is_empty() {
            Err(EvalAltResult::ErrorArithmetic(
                format!("Range is empty: {:?}", range),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_range(range))
        }
    }

    /// Generate a random integer number within an inclusive range.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand(18, 38);
    ///
    /// print(`I'll give you a random number between 18 and 38: ${number}`);
    /// ```
    #[rhai_fn(name = "rand", return_raw)]
    pub fn rand_from_to_inclusive(start: INT, end: INT) -> Result<INT, Box<EvalAltResult>> {
        if start >= end {
            Err(EvalAltResult::ErrorArithmetic(
                format!("Range is empty: {}..{}", start, end),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_range(start..=end))
        }
    }

    /// Generate a random floating-point number between `0.0` and `1.0` (exclusive).
    /// Requires the `float` feature.
    ///
    /// `1.0` is _excluded_ from the possibilities.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand_float();
    ///
    /// print(`I'll give you a random number between 0 and 1: ${number}`);
    /// ```
    #[cfg(feature = "float")]
    pub fn rand_float() -> FLOAT {
        rand::random()
    }
    /// Generate a random floating-point number within an exclusive range.
    /// Requires the `float` feature.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand_float(123.456, 789.678);
    ///
    /// print(`I'll give you a random number between 123.456 and 789.678: ${number}`);
    /// ```
    #[cfg(feature = "float")]
    #[rhai_fn(name = "rand_float", return_raw)]
    pub fn rand_float_range(start: FLOAT, end: FLOAT) -> Result<FLOAT, Box<EvalAltResult>> {
        if start >= end {
            Err(EvalAltResult::ErrorArithmetic(
                format!("Range is empty: {}..{}", start, end),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_range(start..=end))
        }
    }

    /// Generate a random [decimal](https://crates.io/crates/rust_decimal) number.
    /// Requires the `decimal` feature.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand_decimal();
    ///
    /// print(`I'll give you a random decimal number: ${number}`);
    /// ```
    #[cfg(feature = "decimal")]
    pub fn rand_decimal() -> Decimal {
        rand::random()
    }
    /// Generate a random [decimal](https://crates.io/crates/rust_decimal) number within a range.
    /// Requires the `decimal` feature.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let number = rand(18.to_decimal(), 38.to_decimal());
    ///
    /// print(`I'll give you a random number between 18 and 38: ${number}`);
    /// ```
    #[cfg(feature = "decimal")]
    #[rhai_fn(name = "rand_decimal", return_raw)]
    pub fn rand_decimal_range(start: Decimal, end: Decimal) -> Result<Decimal, Box<EvalAltResult>> {
        if start >= end {
            Err(EvalAltResult::ErrorArithmetic(
                format!("Range is empty: {}..{}", start, end),
                Position::NONE,
            )
                .into())
        } else {
            Ok(rand::thread_rng().gen_range(start..=end))
        }
    }
}
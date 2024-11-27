#[allow(unused_imports)]
use rhai::plugin::*;

#[export_module]
pub mod rand_functions {
    use rhai::{EvalAltResult, Position, INT};
    use std::ops::{Range, RangeInclusive};
    use rand::Rng;

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
}
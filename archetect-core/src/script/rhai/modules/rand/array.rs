#[allow(unused_imports)]
use rhai::plugin::*;

#[export_module]
pub mod array_functions {
    use rand::prelude::*;
    use rhai::{Array, Dynamic, INT};

    /// Copy a random element from the array and return it.
    /// Requires the `array` feature.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let x = [1, 2, 3, 4, 5];
    ///
    /// let number = x.sample();
    ///
    /// print(`I'll give you a random number between 1 and 5: ${number}`);
    /// ```
    #[rhai_fn(global)]
    pub fn sample(array: &mut Array) -> Dynamic {
        if !array.is_empty() {
            let mut rng = rand::rng();
            if let Some(res) = array.choose(&mut rng) {
                return res.clone();
            }
        }
        Dynamic::UNIT
    }

    /// Copy a non-repeating random sample of elements from the array and return it.
    /// Requires the `array` feature.
    ///
    /// Elements in the return array are likely not in the same order as in the original array.
    ///
    /// * If `amount` ≤ 0, the empty array is returned.
    /// * If `amount` ≥ length of array, the entire array is returned, but shuffled.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let x = [1, 2, 3, 4, 5];
    ///
    /// let samples = x.sample(3);
    ///
    /// print(`I'll give you 3 random numbers between 1 and 5: ${samples}`);
    /// ```
    #[rhai_fn(global, name = "sample")]
    pub fn sample_with_amount(array: &mut Array, amount: INT) -> Array {
        if array.is_empty() || amount <= 0 {
            return Array::new();
        }

        let mut rng = rand::rng();
        let amount = amount as usize;

        if amount >= array.len() {
            let mut res = array.clone();
            res.shuffle(&mut rng);
            res
        } else {
            let mut res: Array = array.choose_multiple(&mut rng, amount).cloned().collect();
            // Although the elements are selected randomly, the order of elements in
            // the buffer is neither stable nor fully random. So we must shuffle the
            // result to achieve random ordering.
            res.shuffle(&mut rng);
            res
        }
    }

    /// Shuffle the elements in the array.
    /// Requires the `array` feature.
    ///
    /// ### Example
    ///
    /// ```rhai
    /// let x = [1, 2, 3, 4, 5];
    ///
    /// x.shuffle();    // shuffle the elements inside the array
    /// ```
    #[rhai_fn(global)]
    pub fn shuffle(array: &mut Array) {
        let mut rng = rand::rng();
        array.shuffle(&mut rng);
    }
}
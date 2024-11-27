use rhai::def_package;
use rhai::plugin::*;

mod rand;
mod array;

def_package! {
    /// Package for random number generation, sampling and shuffling.
    pub RandomPackage(lib) {
        combine_with_exported_module!(lib, "rand", rand::rand_functions);

        combine_with_exported_module!(lib, "array", array::array_functions);
    }
}
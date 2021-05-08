/// This trait defines a mixed case conversion.
///
/// In mixedCase, word boundaries are indicated by capital letters, excepting
/// the first word.
///
/// ## Example:
///
/// ```rust
/// fn main() {
///     
///     use archetect_core::vendor::heck::CamelCase;
///
///     let sentence = "It is we who built these palaces and cities.";
///     assert_eq!(sentence.to_camel_case(), "itIsWeWhoBuiltThesePalacesAndCities");
/// }
/// ```
pub trait CamelCase: ToOwned {
    /// Convert this type to mixed case.
    fn to_camel_case(&self) -> Self::Owned;
}

impl CamelCase for str {
    fn to_camel_case(&self) -> String {
        crate::vendor::heck::transform(
            self,
            |s, out| {
                if out.is_empty() {
                    crate::vendor::heck::lowercase(s, out);
                } else {
                    crate::vendor::heck::capitalize(s, out)
                }
            },
            |_| {},
        )
    }
}

#[cfg(test)]
mod tests {
    use super::CamelCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_camel_case(), $s2)
            }
        };
    }

    t!(test1: "PascalCase" => "pascalCase");
    t!(test2: "This is Human case." => "thisIsHumanCase");
    t!(test3: "MixedUP PascalCase, with some Spaces" => "mixedUpPascalCaseWithSomeSpaces");
    t!(test4: "mixed_up_ snake_case, with some _spaces" => "mixedUpSnakeCaseWithSomeSpaces");
    t!(test5: "train-case" => "trainCase");
    t!(test6: "CONSTANT_CASE" => "constantCase");
    t!(test7: "snake_case" => "snakeCase");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "thisContainsAllKindsOfWordBoundaries");
    t!(test9: "XΣXΣ baﬄe" => "xσxςBaﬄe");
    t!(test10: "XMLHttpRequest" => "xmlHttpRequest");
    t!(test11: "package.case" => "packageCase");
    t!(test12: "directory/case" => "directoryCase");
    // TODO unicode tests
}

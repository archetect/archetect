/// This trait defines a camel case conversion.
///
/// In PascalCase, word boundaries are indicated by capital letters, including
/// the first word.
///
/// ## Example:
///
/// ```rust
/// use archetect_core::vendor::heck;
///
/// fn main() {
///     
///     use archetect_core::vendor::heck::PascalCase;
///
///     let sentence = "We are not in the least afraid of ruins.";
///     assert_eq!(sentence.to_pascal_case(), "WeAreNotInTheLeastAfraidOfRuins");
/// }
/// ```
pub trait PascalCase: ToOwned {
    /// Convert this type to pascal case.
    fn to_pascal_case(&self) -> Self::Owned;
}

impl PascalCase for str {
    fn to_pascal_case(&self) -> String {
        crate::vendor::heck::transform(self, crate::vendor::heck::capitalize, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use super::PascalCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_pascal_case(), $s2)
            }
        };
    }

    t!(test1: "PascalCase" => "PascalCase");
    t!(test2: "This is Human case." => "ThisIsHumanCase");
    t!(test3: "MixedUP_PascalCase, with some Spaces" => "MixedUpPascalCaseWithSomeSpaces");
    t!(test4: "mixed_up_ snake_case, with some _spaces" => "MixedUpSnakeCaseWithSomeSpaces");
    t!(test5: "train-case" => "TrainCase");
    t!(test6: "CONSTANT_CASE" => "ConstantCase");
    t!(test7: "snake_case" => "SnakeCase");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "ThisContainsAllKindsOfWordBoundaries");
    t!(test9: "XΣXΣ baﬄe" => "XσxςBaﬄe");
    t!(test10: "XMLHttpRequest" => "XmlHttpRequest");
    t!(test11: "directory/case" => "DirectoryCase");
}

/// This trait defines a constant case conversion.
///
/// In CONSTANT_CASE, word boundaries are indicated by underscores and all
/// words are in uppercase.
///
/// ## Example:
///
/// ```rust
/// fn main() {
///     
///     use archetect_core::vendor::heck::ConstantCase;
///
///     let sentence = "That world is growing in this minute.";
///     assert_eq!(sentence.to_constant_case(), "THAT_WORLD_IS_GROWING_IN_THIS_MINUTE");
/// }
/// ```
pub trait ConstantCase: ToOwned {
    /// Convert this type to constant case.
    fn to_constant_case(&self) -> Self::Owned;
}

impl ConstantCase for str {
    fn to_constant_case(&self) -> Self::Owned {
        crate::vendor::heck::transform(self, crate::vendor::heck::uppercase, |s| s.push('_'))
    }
}

#[cfg(test)]
mod tests {
    use super::ConstantCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_constant_case(), $s2)
            }
        };
    }

    t!(test1: "PascalCase" => "PASCAL_CASE");
    t!(test2: "This is Human case." => "THIS_IS_HUMAN_CASE");
    t!(test3: "MixedUP PascalCase, with some Spaces" => "MIXED_UP_PASCAL_CASE_WITH_SOME_SPACES");
    t!(test4: "mixed_up_snake_case with some _spaces" => "MIXED_UP_SNAKE_CASE_WITH_SOME_SPACES");
    t!(test5: "train-case" => "TRAIN_CASE");
    t!(test6: "CONSTANT_CASE" => "CONSTANT_CASE");
    t!(test7: "snake_case" => "SNAKE_CASE");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "THIS_CONTAINS_ALL_KINDS_OF_WORD_BOUNDARIES");
    t!(test9: "XΣXΣ baﬄe" => "XΣXΣ_BAFFLE");
    t!(test10: "XMLHttpRequest" => "XML_HTTP_REQUEST");
    t!(test11: "package.case" => "PACKAGE_CASE");
    t!(test12: "directory/case" => "DIRECTORY_CASE");
}

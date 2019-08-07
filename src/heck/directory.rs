/// This trait defines a directory case conversion.
///
/// In directory_case, word boundaries are indicated by forward slashes.
///
/// ## Example:
///
/// ```rust
/// fn main() {
///
///     use archetect::heck::DirectoryCase;
///
///     let sentence = "We carry a new world here, in our hearts.";
///     assert_eq!(sentence.to_directory_case(), "we/carry/a/new/world/here/in/our/hearts");
/// }
/// ```
pub trait DirectoryCase: ToOwned {
    /// Convert this type to snake case.
    fn to_directory_case(&self) -> Self::Owned;
}

impl DirectoryCase for str {
    fn to_directory_case(&self) -> String {
        crate::heck::transform(self,crate::heck::lowercase, |s| s.push('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::DirectoryCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_directory_case(), $s2)
            }
        }
    }

    t!(test1: "PascalCase" => "pascal/case");
    t!(test2: "This is Human case." => "this/is/human/case");
    t!(test3: "MixedUP PascalCase, with some Spaces" => "mixed/up/pascal/case/with/some/spaces");
    t!(test4: "mixed_up_ snake_case with some _spaces" => "mixed/up/snake/case/with/some/spaces");
    t!(test5: "train-case" => "train/case");
    t!(test6: "CONSTANT_CASE" => "constant/case");
    t!(test7: "snake_case" => "snake/case");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "this/contains/all/kinds/of/word/boundaries");
    t!(test9: "XΣXΣ baﬄe" => "xσxς/baﬄe");
    t!(test10: "XMLHttpRequest" => "xml/http/request");
    t!(test11: "FIELD_NAME11" => "field/name11");
    t!(test12: "99BOTTLES" => "99bottles");
    t!(test13: "FieldNamE11" => "field/nam/e11");

    t!(test14: "abc123def456" => "abc123def456");
    t!(test16: "abc123DEF456" => "abc123/def456");
    t!(test17: "abc123Def456" => "abc123/def456");
    t!(test18: "abc123DEf456" => "abc123/d/ef456");
    t!(test19: "ABC123def456" => "abc123def456");
    t!(test20: "ABC123DEF456" => "abc123def456");
    t!(test21: "ABC123Def456" => "abc123/def456");
    t!(test22: "ABC123DEf456" => "abc123d/ef456");
    t!(test23: "ABC123dEEf456FOO" => "abc123d/e/ef456/foo");
    t!(test24: "abcDEF" => "abc/def");
    t!(test25: "ABcDE" => "a/bc/de");
    t!(test26: "package.case" => "package/case");
    t!(test27: "directory/case" => "directory/case");

}

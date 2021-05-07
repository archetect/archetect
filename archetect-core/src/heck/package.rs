/// This trait defines a java package case conversion.
///
/// In package.case, word boundaries are indicated by periods.
///
/// ## Example:
///
/// ```rust
///
/// fn main() {
///
///     use archetect_core::heck::PackageCase;
///
///     let sentence = "We are going to inherit the earth.";
///     assert_eq!(sentence.to_package_case(), "we.are.going.to.inherit.the.earth");
/// }
/// ```
pub trait PackageCase: ToOwned {
    /// Convert this type to package case.
    fn to_package_case(&self) -> Self::Owned;
}

impl PackageCase for str {
    fn to_package_case(&self) -> Self::Owned {
        crate::heck::transform(self, crate::heck::lowercase, |s| s.push('.'))
    }
}

#[cfg(test)]
mod tests {
    use super::PackageCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_package_case(), $s2)
            }
        };
    }

    t!(test1: "PascalCase" => "pascal.case");
    t!(test2: "This is Human case." => "this.is.human.case");
    t!(test3: "MixedUP PascalCase, with some Spaces" => "mixed.up.pascal.case.with.some.spaces");
    t!(test4: "mixed_up_ snake_case with some _spaces" => "mixed.up.snake.case.with.some.spaces");
    t!(test5: "train-case" => "train.case");
    t!(test6: "CONSTANT_CASE" => "constant.case");
    t!(test7: "snake_case" => "snake.case");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "this.contains.all.kinds.of.word.boundaries");
    t!(test9: "XΣXΣ baﬄe" => "xσxς.baﬄe");
    t!(test10: "XMLHttpRequest" => "xml.http.request");
    t!(test11: "directory/case" => "directory.case");
}

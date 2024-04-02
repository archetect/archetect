/// Deorginalizes a `&str`
///
/// ```
/// use archetect_inflections::number::deordinalize::deordinalize;
///
/// assert!(deordinalize("0.1") == "0.1");
/// assert!(deordinalize("-1st") == "-1");
/// assert!(deordinalize("0th") == "0");
/// assert!(deordinalize("1st") == "1");
/// assert!(deordinalize("2nd") == "2");
/// assert!(deordinalize("3rd") == "3");
/// assert!(deordinalize("9th") == "9");
/// assert!(deordinalize("12th") == "12");
/// assert!(deordinalize("12000th") == "12000");
/// assert!(deordinalize("12001th") == "12001");
/// assert!(deordinalize("12002nd") == "12002");
/// assert!(deordinalize("12003rd") == "12003");
/// assert!(deordinalize("12004th") == "12004");
/// assert!(deordinalize("3rd") == "3");
/// assert!(deordinalize("3rd") == "3");
/// ```
pub fn deordinalize(non_ordinalized_string: &str) -> String {
    if non_ordinalized_string.contains('.') {
        non_ordinalized_string.to_owned()
    } else {
        non_ordinalized_string
            .trim_end_matches("st")
            .trim_end_matches("nd")
            .trim_end_matches("rd")
            .trim_end_matches("th")
            .to_owned()
    }
}

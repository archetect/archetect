/// Provides deordinalization of a string.
///
/// Example string "1st" becomes "1"
pub mod deordinalize;
pub use deordinalize::deordinalize;

/// Provides ordinalization of a string.
///
/// Example string "1" becomes "1st"
pub mod ordinalize;
pub use ordinalize::ordinalize;

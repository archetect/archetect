/// Provides deconstantizea string.
///
/// Example string `Foo::Bar` becomes `Foo`
pub mod deconstantize;
pub use deconstantize::deconstantize;

/// Provides demodulize a string.
///
/// Example string `Foo::Bar` becomes `Bar`
pub mod demodulize;
pub use demodulize::demodulize;

/// Provides conversion to plural strings.
///
/// Example string `FooBar` -> `FooBars`
pub mod pluralize;
pub use pluralize::to_plural;

/// Provides conversion to singular strings.
///
/// Example string `FooBars` -> `FooBar`
pub mod singularize;
pub use singularize::to_singular;

mod constants;

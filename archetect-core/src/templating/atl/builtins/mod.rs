//! Built-in functions and filters for the ATL template engine.
//!
//! Each submodule registers its functions into the shared `__filters` Lua
//! table. Because of the filter/function symmetry implemented in the compiler
//! (see `compiler.rs` — `_ENV` falls back through `__filters`), every entry
//! in `__filters` is reachable from templates in two equivalent forms:
//!
//!   `{{ x | foo }}`              — pipe form, compiled to `__filters.foo(x)`
//!   `{{ foo(x) }}`               — function form, resolved at render time
//!                                  via `_ENV → __filters → foo`
//!
//! Authors pick whichever reads better at the call site. The pipe form wins
//! when the input dominates the call (`items | join(", ")`); the function
//! form wins when the args dominate (`default(maybe, "fallback")`) or for
//! deeply-nested compositions where reading right-to-left is clearer.
//!
//! Filters take precedence over context keys with the same name, so authors
//! cannot accidentally shadow a builtin by calling `ctx:set("now", ...)`.

use mlua::{Lua, Result as LuaResult, Table};

pub mod collections;
pub mod datetime;
pub mod paths;
pub mod strings;
pub mod uuid;

/// Register every built-in module into the shared filter table.
pub fn register_all(lua: &Lua, filters: &Table) -> LuaResult<()> {
    strings::register(lua, filters)?;
    collections::register(lua, filters)?;
    datetime::register(lua, filters)?;
    uuid::register(lua, filters)?;
    paths::register(lua, filters)?;
    Ok(())
}

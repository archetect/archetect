//! UUID built-in functions.
//!
//!   `{{ uuid() }}`        — alias for uuid_v4(), the default
//!   `{{ uuid_v4() }}`     — random v4 UUID
//!   `{{ uuid_v7() }}`     — time-ordered v7 UUID (sortable)
//!   `{{ uuid_nil() }}`    — `00000000-0000-0000-0000-000000000000`

use mlua::{Lua, Result as LuaResult, Table};
use uuid::Uuid;

pub fn register(lua: &Lua, filters: &Table) -> LuaResult<()> {
    filters.set(
        "uuid",
        lua.create_function(|_, ()| Ok(Uuid::new_v4().to_string()))?,
    )?;

    filters.set(
        "uuid_v4",
        lua.create_function(|_, ()| Ok(Uuid::new_v4().to_string()))?,
    )?;

    filters.set(
        "uuid_v7",
        lua.create_function(|_, ()| Ok(Uuid::now_v7().to_string()))?,
    )?;

    filters.set(
        "uuid_nil",
        lua.create_function(|_, ()| Ok(Uuid::nil().to_string()))?,
    )?;

    Ok(())
}

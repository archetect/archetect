//! Date / time built-in filters and functions.
//!
//! All callable from `{{ }}` expressions:
//!
//!   `{{ now() }}`        — current local datetime as RFC3339
//!   `{{ now_utc() }}`    — current UTC datetime as RFC3339
//!   `{{ today() }}`      — current local date as YYYY-MM-DD
//!   `{{ year() }}`       — current local year as integer
//!   `{{ timestamp() }}`  — current Unix timestamp as integer
//!
//! Filter form:
//!
//!   `{{ value | date(format) }}` — strftime-style formatting of an RFC3339 input

use chrono::{Datelike, Local, NaiveDate, TimeZone, Utc};
use mlua::{Error as LuaError, Lua, Result as LuaResult, Table};

pub fn register(lua: &Lua, filters: &Table) -> LuaResult<()> {
    filters.set(
        "now",
        lua.create_function(|_, ()| Ok(Local::now().to_rfc3339()))?,
    )?;

    filters.set(
        "now_utc",
        lua.create_function(|_, ()| Ok(Utc::now().to_rfc3339()))?,
    )?;

    filters.set(
        "today",
        lua.create_function(|_, ()| Ok(Local::now().date_naive().format("%Y-%m-%d").to_string()))?,
    )?;

    filters.set(
        "year",
        lua.create_function(|_, ()| Ok(Local::now().year() as i64))?,
    )?;

    filters.set(
        "timestamp",
        lua.create_function(|_, ()| Ok(Utc::now().timestamp()))?,
    )?;

    // date(value, format) — format an RFC3339 datetime string OR a YYYY-MM-DD
    // date string with the given strftime-style format. The most common use
    // is `{{ today() | date("%Y") }}` to extract just the year.
    filters.set(
        "date",
        lua.create_function(|_, (value, format): (String, String)| {
            // Try parsing as RFC3339 first; fall back to YYYY-MM-DD.
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&value) {
                return Ok(dt.format(&format).to_string());
            }
            if let Ok(d) = NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                // Convert to a DateTime at midnight UTC for formatting
                let dt = Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap());
                return Ok(dt.format(&format).to_string());
            }
            Err(LuaError::RuntimeError(format!(
                "filter `date`: could not parse `{}` as RFC3339 or YYYY-MM-DD",
                value
            )))
        })?,
    )?;

    Ok(())
}

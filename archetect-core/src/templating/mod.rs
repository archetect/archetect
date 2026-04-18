/// Templating engines used by archetect. Currently only ATL — moved here
/// from `script::lua::template_engine` so the engine isn't implied to be a
/// Lua-script-only concern (Phase 8.2 of the ATL evolution plan).
pub mod atl;

#[cfg(test)]
mod tests {
    /// Phase 8.2: verify the templating engine is reachable at its new
    /// canonical path. If this stops compiling, the engine has moved
    /// somewhere else and downstream callers will have broken too.
    #[test]
    fn test_template_engine_module_path_under_templating() {
        let _ = crate::templating::atl::TemplateCompiler::compile("hello {{ name }}", "test")
            .expect("template should compile under crate::templating::atl");
    }
}

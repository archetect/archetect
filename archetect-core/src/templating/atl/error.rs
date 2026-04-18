use std::fmt;

#[derive(Debug, Clone)]
pub enum TemplateCompileError {
    UnterminatedExpression { line: usize },
    UnterminatedLogic { line: usize },
    UnterminatedComment { line: usize },
    EmptyExpression { line: usize },
    InvalidFilter { line: usize, detail: String },
    /// The generated Lua source failed to parse. This typically means a `{% ... %}`
    /// logic block contained malformed Lua. The `template` field is the
    /// human-readable name of the template (file path or `<inline>`); the
    /// `detail` is the underlying mlua parser message.
    InvalidLuaSyntax { template: String, detail: String },
    /// `{% include "..." %}` was malformed (missing path, unbalanced quotes, etc.).
    InvalidInclude { line: usize, detail: String },
    /// An `{% include %}` referenced a path that does not resolve to a file
    /// inside the configured includes directory.
    IncludeNotFound { path: String, line: usize },
    /// An `{% include %}` resolved successfully but the file could not be read.
    IncludeReadError {
        path: String,
        line: usize,
        detail: String,
    },
    /// Including this path would form a cycle. The stack lists the active
    /// chain of includes, outermost first.
    IncludeCycle { path: String, stack: Vec<String> },
    /// An error that originated inside a nested include. Wraps the
    /// underlying error with the include path so the user can tell which
    /// partial template the failure came from. Phase 8.4.
    IncludeChain {
        include_path: String,
        source: Box<TemplateCompileError>,
    },
    /// Wraps a top-level compile error with the name of the template
    /// being compiled. Added by `TemplateCompiler::compile_with` so any
    /// caller of the engine — not just the render layer — sees which
    /// template a tokenize/compile error came from. Phase 8.4.
    ///
    /// Render-layer wrappers (`RenderError::LuaTemplateCompileError`)
    /// strip this variant before stringifying to avoid duplicating the
    /// template path that they already report.
    InTemplate {
        template_name: String,
        source: Box<TemplateCompileError>,
    },
    /// `{% raw %}` was opened but no matching `{% endraw %}` was found.
    UnterminatedRaw { line: usize },
}

impl fmt::Display for TemplateCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedExpression { line } => {
                write!(f, "Unterminated expression '{{{{' at line {}", line)
            }
            Self::UnterminatedLogic { line } => {
                write!(f, "Unterminated logic block '{{% ' at line {}", line)
            }
            Self::UnterminatedComment { line } => {
                write!(f, "Unterminated comment '{{#' at line {}", line)
            }
            Self::EmptyExpression { line } => {
                write!(f, "Empty expression '{{{{ }}}}' at line {}", line)
            }
            Self::InvalidFilter { line, detail } => {
                write!(f, "Invalid filter at line {}: {}", line, detail)
            }
            Self::InvalidLuaSyntax { template, detail } => {
                if template.is_empty() {
                    write!(f, "Invalid Lua syntax in compiled template: {}", detail)
                } else {
                    write!(
                        f,
                        "Invalid Lua syntax in compiled template `{}`: {}",
                        template, detail
                    )
                }
            }
            Self::InvalidInclude { line, detail } => {
                write!(f, "Invalid include at line {}: {}", line, detail)
            }
            Self::IncludeNotFound { path, line } => {
                write!(f, "Include `{}` not found (line {})", path, line)
            }
            Self::IncludeReadError {
                path,
                line,
                detail,
            } => {
                write!(
                    f,
                    "Failed to read include `{}` at line {}: {}",
                    path, line, detail
                )
            }
            Self::IncludeCycle { path, stack } => {
                write!(
                    f,
                    "Include cycle detected at `{}` (chain: {})",
                    path,
                    stack.join(" -> ")
                )
            }
            Self::IncludeChain { include_path, source } => {
                write!(f, "while compiling include `{}`: {}", include_path, source)
            }
            Self::InTemplate { template_name, source } => {
                write!(f, "in template `{}`: {}", template_name, source)
            }
            Self::UnterminatedRaw { line } => {
                write!(f, "Unterminated '{{% raw %}}' block at line {}", line)
            }
        }
    }
}

impl std::error::Error for TemplateCompileError {}

impl TemplateCompileError {
    /// Walk past any `IncludeChain` or `InTemplate` wrappers to the
    /// underlying error. Useful for callers (and tests) that want to
    /// inspect the leaf variant without caring about the wrapping chain.
    #[allow(dead_code)] // exposed for test introspection and future API consumers
    pub fn root_cause(&self) -> &TemplateCompileError {
        let mut cur = self;
        loop {
            match cur {
                TemplateCompileError::IncludeChain { source, .. }
                | TemplateCompileError::InTemplate { source, .. } => cur = source,
                _ => return cur,
            }
        }
    }
}

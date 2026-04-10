use std::fmt;

#[derive(Debug, Clone)]
pub enum TemplateCompileError {
    UnterminatedExpression { line: usize },
    UnterminatedLogic { line: usize },
    UnterminatedComment { line: usize },
    EmptyExpression { line: usize },
    InvalidFilter { line: usize, detail: String },
    /// The generated Lua source failed to parse. This typically means a `{% ... %}`
    /// logic block contained malformed Lua. The `detail` is the underlying mlua
    /// parser message (which carries its own line offset within the generated source).
    InvalidLuaSyntax { detail: String },
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
            Self::InvalidLuaSyntax { detail } => {
                write!(f, "Invalid Lua syntax in compiled template: {}", detail)
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
        }
    }
}

impl std::error::Error for TemplateCompileError {}

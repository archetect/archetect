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
        }
    }
}

impl std::error::Error for TemplateCompileError {}

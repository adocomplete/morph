//! Errors in Morph are meant to be read by people *and* by AI models.
//!
//! Every error carries:
//! - a stable code (like `E002`) that never changes meaning,
//! - the exact line and column where it happened,
//! - a plain sentence saying what went wrong,
//! - a hint saying how to fix it.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct MorphError {
    pub code: &'static str,
    pub message: String,
    pub hint: String,
    pub span: Span,
}

impl MorphError {
    pub fn new(
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        MorphError {
            code,
            message: message.into(),
            hint: hint.into(),
            span,
        }
    }

    /// Render the error with the offending source line and a caret under the column.
    pub fn render(&self, file: &str, source: &str) -> String {
        let line_text = source.lines().nth(self.span.line - 1).unwrap_or("");
        let gutter = self.span.line.to_string();
        let pad = " ".repeat(gutter.len());
        let caret = " ".repeat(self.span.col.saturating_sub(1));
        format!(
            "error[{code}]: {msg}\n{pad}--> {file}:{line}:{col}\n{pad} |\n{gutter} | {line_text}\n{pad} | {caret}^\n{pad} |\n{pad} = hint: {hint}\n",
            code = self.code,
            msg = self.message,
            line = self.span.line,
            col = self.span.col,
            hint = self.hint,
        )
    }
}

impl fmt::Display for MorphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{}] at {}:{}: {}",
            self.code, self.span.line, self.span.col, self.message
        )
    }
}

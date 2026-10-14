//! Morph: a small programming language, built to be easy for people and AI models to read and write.

pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;

use std::io::Write;

use error::MorphError;

/// Run Morph source code, writing anything it prints to `out`.
pub fn run(source: &str, out: impl Write) -> Result<(), MorphError> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    interpreter::Interpreter::new(out).run(&program)
}

//! The lexer turns source text into a list of tokens ("words" of the language).

use crate::error::{MorphError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Number(f64),
    Str(String),
    Ident(String),
    Let,
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, MorphError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut i, mut line, mut col) = (0, 1, 1);

    while i < chars.len() {
        let c = chars[i];
        let span = Span { line, col };
        let mut push = |tok: Tok| {
            tokens.push(Token {
                tok,
                span: span.clone(),
            })
        };

        match c {
            ' ' | '\t' | '\r' => {}
            '\n' => {
                push(Tok::Newline);
                line += 1;
                col = 1;
                i += 1;
                continue;
            }
            // Comments run from `#` to the end of the line.
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '=' => push(Tok::Equals),
            '+' => push(Tok::Plus),
            '-' => push(Tok::Minus),
            '*' => push(Tok::Star),
            '/' => push(Tok::Slash),
            '(' => push(Tok::LParen),
            ')' => push(Tok::RParen),
            ',' => push(Tok::Comma),
            '"' => {
                let mut s = String::new();
                let mut j = i + 1;
                loop {
                    match chars.get(j) {
                        None | Some('\n') => {
                            return Err(MorphError::new(
                                "E001",
                                span,
                                "this string is never closed",
                                "add a closing `\"` before the end of the line",
                            ));
                        }
                        Some('"') => break,
                        Some('\\') => {
                            let escaped = match chars.get(j + 1) {
                                Some('n') => '\n',
                                Some('t') => '\t',
                                Some('"') => '"',
                                Some('\\') => '\\',
                                other => {
                                    let shown = other.map(|c| c.to_string()).unwrap_or_default();
                                    return Err(MorphError::new(
                                        "E002",
                                        Span {
                                            line,
                                            col: col + (j - i),
                                        },
                                        format!("unknown escape `\\{shown}` in string"),
                                        "the escapes Morph knows are \\n, \\t, \\\" and \\\\",
                                    ));
                                }
                            };
                            s.push(escaped);
                            j += 2;
                        }
                        Some(&ch) => {
                            s.push(ch);
                            j += 1;
                        }
                    }
                }
                push(Tok::Str(s));
                col += j + 1 - i;
                i = j + 1;
                continue;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let value = text.parse::<f64>().map_err(|_| {
                    MorphError::new(
                        "E003",
                        span.clone(),
                        format!("`{text}` is not a valid number"),
                        "numbers look like `42` or `3.14`, with at most one `.`",
                    )
                })?;
                tokens.push(Token {
                    tok: Tok::Number(value),
                    span,
                });
                col += i - start;
                continue;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let tok = if word == "let" {
                    Tok::Let
                } else {
                    Tok::Ident(word)
                };
                tokens.push(Token { tok, span });
                col += i - start;
                continue;
            }
            other => {
                return Err(MorphError::new(
                    "E004",
                    span,
                    format!("Morph does not understand the character `{other}`"),
                    "remove it; Morph uses letters, digits, \"strings\", + - * / = ( ) , and # comments",
                ));
            }
        }
        i += 1;
        col += 1;
    }

    tokens.push(Token {
        tok: Tok::Newline,
        span: Span { line, col },
    });
    tokens.push(Token {
        tok: Tok::Eof,
        span: Span { line, col },
    });
    Ok(tokens)
}

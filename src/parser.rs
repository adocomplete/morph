//! The parser turns tokens into a tree (the "AST") that says what the program means.
//!
//! Grammar (one statement per line):
//!
//! ```text
//! program    = { statement? NEWLINE }
//! statement  = "let" IDENT "=" expression
//!            | expression
//! expression = term { ("+" | "-") term }
//! term       = unary { ("*" | "/") unary }
//! unary      = "-" unary | call
//! call       = primary [ "(" [ expression { "," expression } ] ")" ]
//! primary    = NUMBER | STRING | IDENT | "(" expression ")"
//! ```

use crate::error::{MorphError, Span};
use crate::lexer::{Tok, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Var(String, Span),
    Neg(Box<Expr>, Span),
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr, Span),
    Expr(Expr),
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, MorphError> {
    let mut p = Parser { tokens, pos: 0 };
    let mut program = Vec::new();
    loop {
        while p.peek() == &Tok::Newline {
            p.pos += 1;
        }
        if p.peek() == &Tok::Eof {
            return Ok(program);
        }
        program.push(p.statement()?);
        p.end_of_line()?;
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span.clone()
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn end_of_line(&mut self) -> Result<(), MorphError> {
        match self.peek() {
            Tok::Newline => {
                self.pos += 1;
                Ok(())
            }
            other => Err(MorphError::new(
                "E101",
                self.span(),
                format!(
                    "unexpected {} after the end of a statement",
                    describe(other)
                ),
                "put each statement on its own line",
            )),
        }
    }

    fn statement(&mut self) -> Result<Stmt, MorphError> {
        if self.peek() == &Tok::Let {
            let let_span = self.advance().span;
            let name = match self.peek().clone() {
                Tok::Ident(name) => {
                    self.pos += 1;
                    name
                }
                other => {
                    return Err(MorphError::new(
                        "E102",
                        self.span(),
                        format!(
                            "expected a variable name after `let`, found {}",
                            describe(&other)
                        ),
                        "write it like `let name = value`",
                    ));
                }
            };
            if self.peek() != &Tok::Equals {
                return Err(MorphError::new(
                    "E103",
                    self.span(),
                    format!(
                        "expected `=` after `let {name}`, found {}",
                        describe(self.peek())
                    ),
                    format!("write it like `let {name} = value`"),
                ));
            }
            self.pos += 1;
            let value = self.expression()?;
            return Ok(Stmt::Let(name, value, let_span));
        }
        Ok(Stmt::Expr(self.expression()?))
    }

    fn expression(&mut self) -> Result<Expr, MorphError> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(left),
            };
            let span = self.advance().span;
            let right = self.term()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
    }

    fn term(&mut self) -> Result<Expr, MorphError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                _ => return Ok(left),
            };
            let span = self.advance().span;
            let right = self.unary()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
    }

    fn unary(&mut self) -> Result<Expr, MorphError> {
        if self.peek() == &Tok::Minus {
            let span = self.advance().span;
            return Ok(Expr::Neg(Box::new(self.unary()?), span));
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, MorphError> {
        let expr = self.primary()?;
        if self.peek() != &Tok::LParen {
            return Ok(expr);
        }
        let (name, span) = match expr {
            Expr::Var(name, span) => (name, span),
            _ => {
                return Err(MorphError::new(
                    "E104",
                    self.span(),
                    "only a function name can be called with `(...)`",
                    "call functions by name, like `print(1)`",
                ));
            }
        };
        self.pos += 1; // `(`
        let mut args = Vec::new();
        if self.peek() != &Tok::RParen {
            loop {
                args.push(self.expression()?);
                if self.peek() == &Tok::Comma {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if self.peek() != &Tok::RParen {
            return Err(MorphError::new(
                "E105",
                self.span(),
                format!(
                    "expected `)` to close the call to `{name}`, found {}",
                    describe(self.peek())
                ),
                "separate arguments with `,` and close the call with `)`",
            ));
        }
        self.pos += 1;
        Ok(Expr::Call(name, args, span))
    }

    fn primary(&mut self) -> Result<Expr, MorphError> {
        let token = self.advance();
        match token.tok {
            Tok::Number(n) => Ok(Expr::Number(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Ident(name) => Ok(Expr::Var(name, token.span)),
            Tok::LParen => {
                let inner = self.expression()?;
                if self.peek() != &Tok::RParen {
                    return Err(MorphError::new(
                        "E106",
                        self.span(),
                        format!(
                            "expected `)` to match the `(` at line {}, column {}",
                            token.span.line, token.span.col
                        ),
                        "add the missing `)`",
                    ));
                }
                self.pos += 1;
                Ok(inner)
            }
            other => {
                self.pos -= 1;
                Err(MorphError::new(
                    "E107",
                    token.span,
                    format!("expected a value, found {}", describe(&other)),
                    "a value is a number like `42`, a string like `\"hi\"`, a variable name, or `( ... )`",
                ))
            }
        }
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Number(n) => format!("the number `{n}`"),
        Tok::Str(_) => "a string".to_string(),
        Tok::Ident(name) => format!("the name `{name}`"),
        Tok::Let => "`let`".to_string(),
        Tok::Equals => "`=`".to_string(),
        Tok::Plus => "`+`".to_string(),
        Tok::Minus => "`-`".to_string(),
        Tok::Star => "`*`".to_string(),
        Tok::Slash => "`/`".to_string(),
        Tok::LParen => "`(`".to_string(),
        Tok::RParen => "`)`".to_string(),
        Tok::Comma => "`,`".to_string(),
        Tok::Newline => "the end of the line".to_string(),
        Tok::Eof => "the end of the file".to_string(),
    }
}

//! The parser turns tokens into a tree (the "AST") that says what the program means.
//!
//! Grammar (one statement per line):
//!
//! ```text
//! program    = { statement? NEWLINE }
//! statement  = "let" IDENT "=" expression
//!            | if
//!            | expression
//! if         = "if" expression block [ "else" ( if | block ) ]
//! block      = "{" NEWLINE { statement? NEWLINE } "}"
//! expression = and { "or" and }
//! and        = not { "and" not }
//! not        = "not" not | comparison
//! comparison = sum [ ("==" | "!=" | "<" | "<=" | ">" | ">=") sum ]
//! sum        = term { ("+" | "-") term }
//! term       = unary { ("*" | "/") unary }
//! unary      = "-" unary | call
//! call       = primary [ "(" [ expression { "," expression } ] ")" ]
//! primary    = NUMBER | STRING | "true" | "false" | IDENT | "(" expression ")"
//! ```

use crate::error::{MorphError, Span};
use crate::lexer::{Tok, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "==",
            BinOp::NotEq => "!=",
            BinOp::Less => "<",
            BinOp::LessEq => "<=",
            BinOp::Greater => ">",
            BinOp::GreaterEq => ">=",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Bool(bool),
    Var(String, Span),
    Neg(Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),
    Logic(Box<Expr>, LogicOp, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr, Span),
    If {
        cond: Expr,
        cond_span: Span,
        then: Vec<Stmt>,
        otherwise: Option<Vec<Stmt>>,
    },
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
        if self.peek() == &Tok::If {
            return self.if_statement();
        }
        if self.peek() == &Tok::Else {
            return Err(MorphError::new(
                "E112",
                self.span(),
                "`else` must come right after the `}` of an `if` block",
                "put `else` on the same line as the `}`, like `} else {`",
            ));
        }
        Ok(Stmt::Expr(self.expression()?))
    }

    fn if_statement(&mut self) -> Result<Stmt, MorphError> {
        self.pos += 1; // `if`
        let cond_span = self.span();
        let cond = self.expression()?;
        let then = self.block("if")?;
        let otherwise = if self.peek() == &Tok::Else {
            self.pos += 1;
            if self.peek() == &Tok::If {
                Some(vec![self.if_statement()?])
            } else {
                Some(self.block("else")?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            cond_span,
            then,
            otherwise,
        })
    }

    /// Parse `{`, then statements on their own lines, then `}`.
    fn block(&mut self, owner: &str) -> Result<Vec<Stmt>, MorphError> {
        if self.peek() != &Tok::LBrace {
            return Err(MorphError::new(
                "E109",
                self.span(),
                format!(
                    "expected `{{` to start the `{owner}` block, found {}",
                    describe(self.peek())
                ),
                format!("write it like `{owner} ... {{`, with the `{{` on the same line"),
            ));
        }
        let open = self.advance().span;
        if self.peek() != &Tok::Newline {
            return Err(MorphError::new(
                "E110",
                self.span(),
                format!("unexpected {} right after `{{`", describe(self.peek())),
                "start a new line after `{`, and put each statement in the block on its own line",
            ));
        }
        let mut body = Vec::new();
        loop {
            while self.peek() == &Tok::Newline {
                self.pos += 1;
            }
            match self.peek() {
                Tok::RBrace => {
                    self.pos += 1;
                    return Ok(body);
                }
                Tok::Eof => {
                    return Err(MorphError::new(
                        "E111",
                        open,
                        format!("this `{owner}` block is never closed"),
                        "add a `}` on its own line where the block ends",
                    ));
                }
                _ => {
                    body.push(self.statement()?);
                    self.end_of_line()?;
                }
            }
        }
    }

    fn expression(&mut self) -> Result<Expr, MorphError> {
        let mut left = self.and()?;
        while self.peek() == &Tok::Or {
            let span = self.advance().span;
            let right = self.and()?;
            left = Expr::Logic(Box::new(left), LogicOp::Or, Box::new(right), span);
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, MorphError> {
        let mut left = self.not()?;
        while self.peek() == &Tok::And {
            let span = self.advance().span;
            let right = self.not()?;
            left = Expr::Logic(Box::new(left), LogicOp::And, Box::new(right), span);
        }
        Ok(left)
    }

    fn not(&mut self) -> Result<Expr, MorphError> {
        if self.peek() == &Tok::Not {
            let span = self.advance().span;
            return Ok(Expr::Not(Box::new(self.not()?), span));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, MorphError> {
        let left = self.sum()?;
        let Some(op) = comparison_op(self.peek()) else {
            return Ok(left);
        };
        let span = self.advance().span;
        let right = self.sum()?;
        if let Some(next) = comparison_op(self.peek()) {
            return Err(MorphError::new(
                "E108",
                self.span(),
                format!("cannot chain `{}` after `{}`", next.symbol(), op.symbol()),
                "compare two things at a time and join them with `and`, like `a < b and b < c`",
            ));
        }
        Ok(Expr::Binary(Box::new(left), op, Box::new(right), span))
    }

    fn sum(&mut self) -> Result<Expr, MorphError> {
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
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
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
                    "a value is a number like `42`, a string like `\"hi\"`, `true`, `false`, a variable name, or `( ... )`",
                ))
            }
        }
    }
}

fn comparison_op(tok: &Tok) -> Option<BinOp> {
    match tok {
        Tok::EqEq => Some(BinOp::Eq),
        Tok::NotEq => Some(BinOp::NotEq),
        Tok::Less => Some(BinOp::Less),
        Tok::LessEq => Some(BinOp::LessEq),
        Tok::Greater => Some(BinOp::Greater),
        Tok::GreaterEq => Some(BinOp::GreaterEq),
        _ => None,
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Number(n) => format!("the number `{n}`"),
        Tok::Str(_) => "a string".to_string(),
        Tok::Ident(name) => format!("the name `{name}`"),
        Tok::Let => "`let`".to_string(),
        Tok::True => "`true`".to_string(),
        Tok::False => "`false`".to_string(),
        Tok::If => "`if`".to_string(),
        Tok::Else => "`else`".to_string(),
        Tok::And => "`and`".to_string(),
        Tok::Or => "`or`".to_string(),
        Tok::Not => "`not`".to_string(),
        Tok::Equals => "`=`".to_string(),
        Tok::EqEq => "`==`".to_string(),
        Tok::NotEq => "`!=`".to_string(),
        Tok::Less => "`<`".to_string(),
        Tok::LessEq => "`<=`".to_string(),
        Tok::Greater => "`>`".to_string(),
        Tok::GreaterEq => "`>=`".to_string(),
        Tok::LBrace => "`{`".to_string(),
        Tok::RBrace => "`}`".to_string(),
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

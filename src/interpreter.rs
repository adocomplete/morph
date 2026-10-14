//! The interpreter walks the tree and runs it.

use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use crate::error::{MorphError, Span};
use crate::parser::{BinOp, Expr, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
        }
    }
}

pub struct Interpreter<W: Write> {
    vars: HashMap<String, Value>,
    out: W,
}

impl<W: Write> Interpreter<W> {
    pub fn new(out: W) -> Self {
        Interpreter {
            vars: HashMap::new(),
            out,
        }
    }

    pub fn run(&mut self, program: &[Stmt]) -> Result<(), MorphError> {
        for stmt in program {
            match stmt {
                Stmt::Let(name, expr, span) => {
                    if self.vars.contains_key(name) {
                        return Err(MorphError::new(
                            "E201",
                            span.clone(),
                            format!("`{name}` already exists"),
                            format!(
                                "pick a new name; each `let` creates a new variable, and `{name}` was created earlier"
                            ),
                        ));
                    }
                    let value = self.eval(expr)?;
                    self.vars.insert(name.clone(), value);
                }
                Stmt::Expr(expr) => {
                    self.eval(expr)?;
                }
            }
        }
        Ok(())
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value, MorphError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Var(name, span) => self.vars.get(name).cloned().ok_or_else(|| {
                MorphError::new(
                    "E202",
                    span.clone(),
                    format!("there is no variable named `{name}`"),
                    match self.closest_name(name) {
                        Some(close) => format!("did you mean `{close}`?"),
                        None => format!("create it first with `let {name} = ...`"),
                    },
                )
            }),
            Expr::Neg(inner, span) => match self.eval(inner)? {
                Value::Number(n) => Ok(Value::Number(-n)),
                other => Err(MorphError::new(
                    "E203",
                    span.clone(),
                    format!("cannot put `-` in front of a {}", other.type_name()),
                    "`-` only works on numbers",
                )),
            },
            Expr::Binary(left, op, right, span) => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                binary(&l, *op, &r, span)
            }
            Expr::Call(name, args, span) => {
                let values = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(name, values, span)
            }
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>, span: &Span) -> Result<Value, MorphError> {
        match name {
            "print" => {
                let line = args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                writeln!(self.out, "{line}").expect("failed to write output");
                Ok(Value::Str(String::new()))
            }
            _ => Err(MorphError::new(
                "E204",
                span.clone(),
                format!("there is no function named `{name}`"),
                "the only built-in function so far is `print`",
            )),
        }
    }

    /// Find an existing variable whose name is one small typo away.
    fn closest_name(&self, name: &str) -> Option<String> {
        self.vars
            .keys()
            .map(|k| (edit_distance(k, name), k))
            .filter(|(d, _)| *d <= 2)
            .min()
            .map(|(_, k)| k.clone())
    }
}

fn binary(l: &Value, op: BinOp, r: &Value, span: &Span) -> Result<Value, MorphError> {
    use Value::*;
    match (l, op, r) {
        (Number(a), BinOp::Add, Number(b)) => Ok(Number(a + b)),
        (Number(a), BinOp::Sub, Number(b)) => Ok(Number(a - b)),
        (Number(a), BinOp::Mul, Number(b)) => Ok(Number(a * b)),
        (Number(_), BinOp::Div, Number(b)) if *b == 0.0 => Err(MorphError::new(
            "E205",
            span.clone(),
            "division by zero",
            "check that the number on the right of `/` is not 0",
        )),
        (Number(a), BinOp::Div, Number(b)) => Ok(Number(a / b)),
        (Str(a), BinOp::Add, Str(b)) => Ok(Str(format!("{a}{b}"))),
        (Str(_), BinOp::Add, Number(_)) | (Number(_), BinOp::Add, Str(_)) => Err(MorphError::new(
            "E206",
            span.clone(),
            format!("cannot add a {} and a {}", l.type_name(), r.type_name()),
            "Morph never converts types on its own; to show both, use `print(a, b)`",
        )),
        _ => {
            let symbol = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
            };
            Err(MorphError::new(
                "E207",
                span.clone(),
                format!(
                    "cannot use `{symbol}` with a {} and a {}",
                    l.type_name(),
                    r.type_name()
                ),
                "`-`, `*` and `/` only work on numbers; `+` works on two numbers or two strings",
            ))
        }
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

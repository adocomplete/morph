//! The interpreter walks the tree and runs it.

use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use crate::error::{MorphError, Span};
use crate::parser::{BinOp, Expr, LogicOp, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "boolean",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
        }
    }
}

pub struct Interpreter<W: Write> {
    /// One map of variables per open block; the first is the whole program.
    scopes: Vec<HashMap<String, Value>>,
    out: W,
}

impl<W: Write> Interpreter<W> {
    pub fn new(out: W) -> Self {
        Interpreter {
            scopes: vec![HashMap::new()],
            out,
        }
    }

    pub fn run(&mut self, program: &[Stmt]) -> Result<(), MorphError> {
        for stmt in program {
            self.exec(stmt)?;
        }
        Ok(())
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<(), MorphError> {
        match stmt {
            Stmt::Let(name, expr, span) => {
                if self.lookup(name).is_some() {
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
                self.scopes
                    .last_mut()
                    .expect("there is always a scope")
                    .insert(name.clone(), value);
            }
            Stmt::If {
                cond,
                cond_span,
                then,
                otherwise,
            } => {
                let chosen = match self.eval(cond)? {
                    Value::Bool(true) => Some(then),
                    Value::Bool(false) => otherwise.as_ref(),
                    other => {
                        return Err(MorphError::new(
                            "E209",
                            cond_span.clone(),
                            format!(
                                "the condition of `if` must be true or false, but this is a {}",
                                other.type_name()
                            ),
                            "use a comparison, like `if count > 0 {`",
                        ));
                    }
                };
                if let Some(block) = chosen {
                    self.scopes.push(HashMap::new());
                    let result = self.run(block);
                    self.scopes.pop();
                    result?;
                }
            }
            Stmt::Expr(expr) => {
                self.eval(expr)?;
            }
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value, MorphError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Var(name, span) => self.lookup(name).cloned().ok_or_else(|| {
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
            Expr::Not(inner, span) => match self.eval(inner)? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                other => Err(MorphError::new(
                    "E210",
                    span.clone(),
                    format!("`not` needs true or false, but got a {}", other.type_name()),
                    "use a comparison, like `not (count == 0)`",
                )),
            },
            Expr::Logic(left, op, right, span) => {
                let word = match op {
                    LogicOp::And => "and",
                    LogicOp::Or => "or",
                };
                let check = |value: Value| match value {
                    Value::Bool(b) => Ok(b),
                    other => Err(MorphError::new(
                        "E210",
                        span.clone(),
                        format!(
                            "`{word}` needs true or false on both sides, but got a {}",
                            other.type_name()
                        ),
                        format!("use comparisons on both sides, like `a > 0 {word} b > 0`"),
                    )),
                };
                let l = check(self.eval(left)?)?;
                // Stop early when the left side already decides the answer.
                match (op, l) {
                    (LogicOp::And, false) => Ok(Value::Bool(false)),
                    (LogicOp::Or, true) => Ok(Value::Bool(true)),
                    _ => Ok(Value::Bool(check(self.eval(right)?)?)),
                }
            }
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
        self.scopes
            .iter()
            .flat_map(|scope| scope.keys())
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
        (Number(a), BinOp::Less, Number(b)) => Ok(Bool(a < b)),
        (Number(a), BinOp::LessEq, Number(b)) => Ok(Bool(a <= b)),
        (Number(a), BinOp::Greater, Number(b)) => Ok(Bool(a > b)),
        (Number(a), BinOp::GreaterEq, Number(b)) => Ok(Bool(a >= b)),
        (_, BinOp::Eq | BinOp::NotEq, _) if l.type_name() != r.type_name() => Err(MorphError::new(
            "E208",
            span.clone(),
            format!(
                "cannot compare a {} with a {}",
                l.type_name(),
                r.type_name()
            ),
            "`==` and `!=` only compare values of the same type, so this would always be a mistake",
        )),
        (_, BinOp::Eq, _) => Ok(Bool(l == r)),
        (_, BinOp::NotEq, _) => Ok(Bool(l != r)),
        (Str(a), BinOp::Add, Str(b)) => Ok(Str(format!("{a}{b}"))),
        (Str(_), BinOp::Add, Number(_)) | (Number(_), BinOp::Add, Str(_)) => Err(MorphError::new(
            "E206",
            span.clone(),
            format!("cannot add a {} and a {}", l.type_name(), r.type_name()),
            "Morph never converts types on its own; to show both, use `print(a, b)`",
        )),
        _ => {
            let hint = match op {
                BinOp::Add => "`+` works on two numbers or two strings",
                BinOp::Less | BinOp::LessEq | BinOp::Greater | BinOp::GreaterEq => {
                    "`<`, `<=`, `>` and `>=` only compare numbers"
                }
                _ => "`-`, `*` and `/` only work on numbers",
            };
            Err(MorphError::new(
                "E207",
                span.clone(),
                format!(
                    "cannot use `{}` with a {} and a {}",
                    op.symbol(),
                    l.type_name(),
                    r.type_name()
                ),
                hint,
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

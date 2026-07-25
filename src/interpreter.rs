use std::fmt;

use crate::{
    expr::Expr,
    scanner::{self, Token},
    stmt::Stmt,
};
use anyhow::{Result as AnyhowResult, anyhow};

#[derive(Debug)]
struct RuntimeError {
    token: Token,
    message: String,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Runtime Error line: {}, token: {}] {}",
            self.token.line,
            self.token.token_type.pretty_print(),
            self.message
        )
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum LoxValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
}

impl fmt::Display for LoxValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Number(x) => {
                let s: String = x.to_string();
                if s.ends_with(".0") {
                    write!(f, "{}", s.split('.').next().unwrap())
                } else {
                    write!(f, "{}", s)
                }
            }
            Self::String(s) => write!(f, "{}", s),
        }
    }
}

impl LoxValue {
    fn truthiness(&self) -> bool {
        match self {
            LoxValue::Nil => false,
            LoxValue::Boolean(b) => *b,
            LoxValue::Number(_) => true,
            LoxValue::String(_) => true,
        }
    }

    fn get_number(&self) -> Result<f64, String> {
        match self {
            LoxValue::Number(x) => Ok(*x),
            LoxValue::Nil => Err(String::from("Type error: expected Number, found Nil")),
            LoxValue::Boolean(_) => Err(String::from("Type error: expected Number, found Boolean")),
            LoxValue::String(_) => Err(String::from("Type error: expected Number, found String")),
        }
    }

    fn get_string(&self) -> Result<&str, String> {
        match self {
            LoxValue::String(s) => Ok(s),
            LoxValue::Number(_) => Err(String::from("Type error: expected String, found Number")),
            LoxValue::Nil => Err(String::from("Type error: expected String, found Nil")),
            LoxValue::Boolean(_) => Err(String::from("Type error: expected String, found Boolean")),
        }
    }

    fn is_equal(&self, other: &Self) -> bool {
        if let (Ok(a), Ok(b)) = (self.get_number(), other.get_number()) {
            (a.is_nan() && b.is_nan()) || (a == b)
        } else {
            self == other
        }
    }
}

fn evaluate(expr: &Expr) -> Result<LoxValue, RuntimeError> {
    match expr {
        Expr::Literal(literal) => {
            let out = match &literal.value {
                scanner::TokenType::Nil => LoxValue::Nil,
                scanner::TokenType::True => LoxValue::Boolean(true),
                scanner::TokenType::False => LoxValue::Boolean(false),
                scanner::TokenType::String(s) => LoxValue::String(s.clone()),
                scanner::TokenType::Number(x) => LoxValue::Number(*x),
                _ => panic!("Unexpected token type for literal!"), // TODO: Don't panic, make unrepresentable
            };
            Ok(out)
        }
        Expr::Grouping(grouping) => evaluate(&grouping.expression),
        Expr::Unary(unary) => {
            let right = evaluate(&unary.expression)?;

            let out = match &unary.operator.token_type {
                scanner::TokenType::Bang => LoxValue::Boolean(!right.truthiness()),
                scanner::TokenType::Minus => {
                    LoxValue::Number(-right.get_number().map_err(|message| RuntimeError {
                        token: unary.operator.clone(),
                        message,
                    })?)
                }
                _ => panic!("Unexpected token type for unary!"), // TODO: Don't panic, make unrepresentable
            };

            Ok(out)
        }
        Expr::Binary(binary) => {
            let left = evaluate(&binary.left)?;
            let right = evaluate(&binary.right)?;

            let err = |message: String| RuntimeError {
                token: binary.operator.clone(),
                message,
            };
            match binary.operator.token_type {
                scanner::TokenType::Minus => {
                    Ok(LoxValue::Number(left.get_number().map_err(err)? - right.get_number().map_err(err)?))
                }
                scanner::TokenType::Plus => {
                    if let (Ok(l), Ok(r)) = (left.get_number(), right.get_number()) {
                        Ok(LoxValue::Number(l + r))
                    } else if let (Ok(l), Ok(r)) = (left.get_string(), right.get_string()) {
                        let mut s = String::with_capacity(l.len() + r.len());
                        s.push_str(l);
                        s.push_str(r);
                        Ok(LoxValue::String(s))
                    } else {
                        Err(err(String::from("Type error: + with incompatible types")))
                    }
                }
                scanner::TokenType::Slash => {
                    let right = right.get_number().map_err(err)?;
                    if right == 0.0 {
                        Err(err(String::from("Division by zero")))
                    } else {
                        Ok(LoxValue::Number(left.get_number().map_err(err)? / right))
                    }
                }
                scanner::TokenType::Star => {
                    Ok(LoxValue::Number(left.get_number().map_err(err)? * right.get_number().map_err(err)?))
                }
                scanner::TokenType::Greater => {
                    Ok(LoxValue::Boolean(left.get_number().map_err(err)? > right.get_number().map_err(err)?))
                }
                scanner::TokenType::GreaterEqual => {
                    Ok(LoxValue::Boolean(left.get_number().map_err(err)? >= right.get_number().map_err(err)?))
                }
                scanner::TokenType::Less => {
                    Ok(LoxValue::Boolean(left.get_number().map_err(err)? < right.get_number().map_err(err)?))
                }
                scanner::TokenType::LessEqual => {
                    Ok(LoxValue::Boolean(left.get_number().map_err(err)? <= right.get_number().map_err(err)?))
                }
                scanner::TokenType::BangEqual => Ok(LoxValue::Boolean(!left.is_equal(&right))),
                scanner::TokenType::EqualEqual => Ok(LoxValue::Boolean(left.is_equal(&right))),
                _ => panic!("Unexpected token type for binary!"), // TODO: Don't panic, make unrepresentable
            }
        }
    }
}

fn execute_stmt(stmt: Stmt) -> Result<(), RuntimeError> {
    match stmt {
        Stmt::StmtExpression(stmt_expression) => {
            evaluate(&stmt_expression.expression)?;
            Ok(())
        }
        Stmt::Print(print) => {
            println!("{}", evaluate(&print.expression)?);
            Ok(())
        }
    }
}

// This will have to be converted into an Interpreter struct once we have global vars
// that should be preserved across runs in the REPL. Or alternatively, some kind of
// &mut Globals passed in or something like that.
pub(crate) fn interpret(stmts: Vec<Stmt>) -> AnyhowResult<()> {
    for stmt in stmts {
        execute_stmt(stmt).map_err(|e| {
            eprintln!("{e}");
            anyhow!("")
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{parser::parse, scanner::scan_tokens};

    use super::*;

    fn execute_str(s: &str) -> Result<LoxValue, RuntimeError> {
        let mut with_semicolon = String::with_capacity(s.len() + 1);
        with_semicolon.push_str(s);
        with_semicolon.push(';');
        let parsed = parse(scan_tokens(&with_semicolon).expect("scan error")).expect("parse error");
        assert!(parsed.len() == 1);
        match parsed.first().unwrap() {
            Stmt::StmtExpression(stmt_expression) => evaluate(&stmt_expression.expression),
            Stmt::Print(_) => {
                panic!("Expected statement expression")
            }
        }
        // interpret_expr(&)
    }

    fn number(x: f64) -> LoxValue {
        LoxValue::Number(x)
    }

    fn string(s: &str) -> LoxValue {
        LoxValue::String(String::from(s))
    }

    fn boolean(b: bool) -> LoxValue {
        LoxValue::Boolean(b)
    }

    const INTER_ERR: &str = "interpret error";

    #[test]
    fn test_literals() {
        assert_eq!(execute_str("1").expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str("(1)").expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str("false").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("true").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#""asdf""#).expect(INTER_ERR), string("asdf"));
    }

    #[test]
    fn test_unary() {
        assert_eq!(execute_str("!true").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("!false").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str("!5").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("!nil").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str("!!nil").expect(INTER_ERR), boolean(false));

        assert_eq!(execute_str("-5").expect(INTER_ERR), number(-5.0));
        assert!(execute_str(r#"-"asdf""#).is_err());
    }

    #[test]
    fn test_binary() {
        assert_eq!(execute_str("1 / 2").expect(INTER_ERR), number(0.5));
        assert_eq!(execute_str("1 - 1").expect(INTER_ERR), number(0.0));
        assert_eq!(execute_str("3 * 5").expect(INTER_ERR), number(15.0));
        assert_eq!(execute_str("\"Hello \" + \"World\"").expect(INTER_ERR), string("Hello World"));

        assert_eq!(execute_str("1 == 1").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str("1 != 1").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("1 == 2").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("1 != 2").expect(INTER_ERR), boolean(true));

        assert_eq!(execute_str("0 < 1").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str("1 < 1").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("1 <= 1").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str("1 > 2").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("1 >= 2").expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str("2 >= 2").expect(INTER_ERR), boolean(true));

        assert_eq!(execute_str(r#""asdf" == "asdf""#).expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#""asdf" != "asdf""#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#""asdf" == "wxyz""#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#""asdf" != "wxyz""#).expect(INTER_ERR), boolean(true));
    }

    #[test]
    fn test_complex() {
        assert_eq!(execute_str("(1 / 2) * 2").expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str("(1 + 2) * 5").expect(INTER_ERR), number(15.0));
        assert_eq!(execute_str("(1 + 2) * -5").expect(INTER_ERR), number(-15.0));
        assert_eq!(execute_str("1 + (2 * 5)").expect(INTER_ERR), number(11.0));
        assert_eq!(execute_str("1 + (2 * 5) < (1 + 2) * 5").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#"!(("s1" == "s2") == nil)"#).expect(INTER_ERR), boolean(true));

        for i in 1..20 {
            let mut s = String::new();
            for _ in 0..i {
                s.push('!');
            }
            s.push_str("nil");

            assert_eq!(execute_str(&s).expect(INTER_ERR), boolean((i % 2) == 1));
        }
    }

    #[test]
    fn test_runtime_errors() {
        assert!(execute_str("1 * false").is_err());
        assert!(execute_str("true * false").is_err());
        assert!(execute_str("-true").is_err());
        assert!(execute_str("1 < false").is_err());
        assert!(execute_str("1 / 0").is_err());
        assert!(execute_str(r#"-"String""#).is_err());
    }
}

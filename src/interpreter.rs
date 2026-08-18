use std::fmt;

use crate::{
    ast::{Expr, Stmt},
    environment::Environment,
    scanner::TokenLike,
};
use anyhow::{Result as AnyhowResult, anyhow};
use std::io::Write;

enum LoopControlFlow {
    Normal,
    Break,
}

#[derive(Debug)]
pub(crate) struct RuntimeError {
    line: usize,
    token_display: String,
    message: String,
}

impl RuntimeError {
    pub(crate) fn new(token: &impl TokenLike, message: String) -> Self {
        Self {
            line: token.line(),
            token_display: token.token_display(),
            message,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Runtime Error line: {}, token: {}] {}", self.line, self.token_display, self.message)
    }
}

#[derive(Debug, PartialEq, Clone)]
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

pub(crate) struct Interpreter<W: Write> {
    environment: Environment,
    writer: W,
}

impl<W: Write> Interpreter<W> {
    pub(crate) fn new(writer: W) -> Self {
        Interpreter {
            environment: Environment::new(),
            writer,
        }
    }

    pub(crate) fn evaluate(&mut self, expr: &Expr) -> Result<LoxValue, RuntimeError> {
        match expr {
            Expr::LiteralExpr(literal) => {
                let out = match &literal.value {
                    crate::parser::ParsedLiteral::Nil => LoxValue::Nil,
                    crate::parser::ParsedLiteral::True => LoxValue::Boolean(true),
                    crate::parser::ParsedLiteral::False => LoxValue::Boolean(false),
                    crate::parser::ParsedLiteral::String(s) => LoxValue::String(s.clone()),
                    crate::parser::ParsedLiteral::Number(x) => LoxValue::Number(*x),
                };
                Ok(out)
            }
            Expr::Grouping(grouping) => self.evaluate(&grouping.expression),
            Expr::Unary(unary) => {
                let right = self.evaluate(&unary.expression)?;

                let out = match &unary.operator.op() {
                    crate::parser::UnaryOp::Bang => LoxValue::Boolean(!right.truthiness()),
                    crate::parser::UnaryOp::Minus => LoxValue::Number(
                        -right.get_number().map_err(|message| RuntimeError::new(&unary.operator, message))?,
                    ),
                };

                Ok(out)
            }
            Expr::Binary(binary) => {
                let left = self.evaluate(&binary.left)?;
                let right = self.evaluate(&binary.right)?;

                let err = |message: String| RuntimeError::new(&binary.operator, message);

                match binary.operator.op() {
                    crate::parser::BinaryOp::Minus => Ok(LoxValue::Number(
                        left.get_number().map_err(err)? - right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Plus => {
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
                    crate::parser::BinaryOp::Slash => {
                        let right = right.get_number().map_err(err)?;
                        if right == 0.0 {
                            Err(err(String::from("Division by zero")))
                        } else {
                            Ok(LoxValue::Number(left.get_number().map_err(err)? / right))
                        }
                    }
                    crate::parser::BinaryOp::Star => Ok(LoxValue::Number(
                        left.get_number().map_err(err)? * right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Greater => Ok(LoxValue::Boolean(
                        left.get_number().map_err(err)? > right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::GreaterEqual => Ok(LoxValue::Boolean(
                        left.get_number().map_err(err)? >= right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Less => Ok(LoxValue::Boolean(
                        left.get_number().map_err(err)? < right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::LessEqual => Ok(LoxValue::Boolean(
                        left.get_number().map_err(err)? <= right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::BangEqual => Ok(LoxValue::Boolean(!left.is_equal(&right))),
                    crate::parser::BinaryOp::EqualEqual => Ok(LoxValue::Boolean(left.is_equal(&right))),
                    _ => panic!("Unexpected token type for binary!"), // TODO: Don't panic, make unrepresentable
                }
            }
            Expr::Variable(variable) => self.environment.get(&variable.name),
            Expr::Assign(assign) => {
                let value = self.evaluate(&assign.value)?;
                // TODO: Any way to avoid the clone here?
                match self.environment.assign(&assign.name, value.clone()) {
                    Ok(()) => Ok(value),
                    Err(()) => Err(RuntimeError::new(&assign.name, "Undefined variable".to_string())),
                }
            }
            Expr::Logical(logical) => {
                let left = self.evaluate(&logical.left)?;

                match logical.operator.op() {
                    crate::parser::LogicalOp::And => {
                        if !left.truthiness() {
                            return Ok(left);
                        }
                    }
                    crate::parser::LogicalOp::Or => {
                        if left.truthiness() {
                            return Ok(left);
                        }
                    }
                }

                self.evaluate(&logical.right)
            }
        }
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<LoopControlFlow, RuntimeError> {
        match stmt {
            Stmt::StmtExpression(stmt_expression) => {
                self.evaluate(&stmt_expression.expression)?;
                Ok(LoopControlFlow::Normal)
            }
            Stmt::Print(print) => {
                let value = self.evaluate(&print.expression)?;
                writeln!(self.writer, "{}", value)
                    .map_err(|e| RuntimeError {
                        line: 0,
                        token_display: "".to_string(),
                        message: format!("Error writing to output stream: {e}"),
                    })
                    .map(|_| LoopControlFlow::Normal)
            }
            Stmt::Var(var) => {
                let value = if let Some(initializer) = &var.initializer {
                    self.evaluate(&initializer)
                } else {
                    Ok(LoxValue::Nil)
                }?;

                self.environment.define(&var.name, value);

                Ok(LoopControlFlow::Normal)
            }
            Stmt::Block(block) => {
                self.environment.enter_scope();
                let mut out = Ok(LoopControlFlow::Normal);
                for statement in block.statements.iter() {
                    match self.execute_stmt(statement) {
                        Ok(control_flow) => {
                            match control_flow {
                                LoopControlFlow::Normal => {}
                                LoopControlFlow::Break => {
                                    out = Ok(control_flow);
                                    break;
                                }
                            };
                        }
                        Err(runtime_error) => {
                            out = Err(runtime_error);
                            break;
                        }
                    }
                }
                self.environment.exit_scope();
                out
            }
            Stmt::If(if_stmt) => {
                if self.evaluate(&if_stmt.condition)?.truthiness() {
                    self.execute_stmt(&if_stmt.then_branch)
                } else if let Some(else_stmt) = &if_stmt.else_branch {
                    self.execute_stmt(else_stmt)
                } else {
                    Ok(LoopControlFlow::Normal)
                }
            }
            Stmt::While(while_stmt) => {
                while self.evaluate(&while_stmt.condition)?.truthiness() {
                    match self.execute_stmt(&while_stmt.body)? {
                        LoopControlFlow::Normal => {}
                        LoopControlFlow::Break => {
                            break;
                        }
                    }
                }
                Ok(LoopControlFlow::Normal)
            }
            Stmt::Break(_) => Ok(LoopControlFlow::Break),
        }
    }

    // This will have to be converted into an Interpreter struct once we have global vars
    // that should be preserved across runs in the REPL. Or alternatively, some kind of
    // &mut Globals passed in or something like that.
    pub(crate) fn interpret(&mut self, stmts: &[Stmt]) -> AnyhowResult<()> {
        for stmt in stmts {
            self.execute_stmt(stmt).map_err(|e| {
                eprintln!("{e}");
                anyhow!("")
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::parse, scanner::scan_tokens};

    use super::*;

    fn execute_str(s: &str) -> Result<LoxValue, RuntimeError> {
        let mut interpreter = Interpreter::new(std::io::stdout());
        let mut with_semicolon = String::with_capacity(s.len() + 1);
        with_semicolon.push_str(s);
        with_semicolon.push(';');
        let parsed =
            parse(scan_tokens(&with_semicolon).expect("scan error"), std::io::stderr()).expect("parse error");
        assert!(parsed.len() == 1);
        match parsed.first().unwrap() {
            Stmt::StmtExpression(stmt_expression) => interpreter.evaluate(&stmt_expression.expression),
            _ => {
                panic!("Expected statement expression")
            }
        }
        // interpret_expr(&)
    }

    fn execute_stmt_and_expr(stmt: &str, expr: &str) -> Result<LoxValue, RuntimeError> {
        let mut interpreter = Interpreter::new(std::io::stdout());

        interpreter
            .interpret(
                &parse(scan_tokens(stmt).expect("scan error"), std::io::stderr()).expect("parse error"),
            )
            .expect("interpreting error");

        let parsed = parse(scan_tokens(expr).expect("scan error"), std::io::stderr()).expect("parse error");
        assert!(parsed.len() == 1);
        match parsed.first().unwrap() {
            Stmt::StmtExpression(stmt_expression) => interpreter.evaluate(&stmt_expression.expression),
            _ => {
                panic!("Expected statement expression")
            }
        }
    }

    fn execute_stmts(stmt: &str) -> String {
        let buffer = Vec::new();
        let mut interpreter = Interpreter::new(buffer);

        interpreter
            .interpret(
                &parse(scan_tokens(stmt).expect("scan error"), std::io::stderr()).expect("parse error"),
            )
            .expect("interpreting error");

        String::from_utf8(interpreter.writer).expect("Error with UTF8 encoding")
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

        assert_eq!(execute_str("0 and 2").expect(INTER_ERR), number(2.0));
        assert_eq!(execute_str("1 and 2").expect(INTER_ERR), number(2.0));
        assert_eq!(execute_str("true and 2").expect(INTER_ERR), number(2.0));
        assert_eq!(execute_str("1 and true").expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#"1 and "Hi!""#).expect(INTER_ERR), string("Hi!"));
        assert_eq!(execute_str(r#""Hi!" and 1"#).expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str(r#"false and 1"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"1 and false"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"1 and nil"#).expect(INTER_ERR), LoxValue::Nil);
        assert_eq!(execute_str(r#"nil and 1"#).expect(INTER_ERR), LoxValue::Nil);

        assert_eq!(execute_str("1 or 2").expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str(r#""Hi!" or 1"#).expect(INTER_ERR), string("Hi!"));
        assert_eq!(execute_str(r#"false or 1"#).expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str(r#"false or true"#).expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#"false or false"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"nil or false"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"false or nil"#).expect(INTER_ERR), LoxValue::Nil);
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

    #[test]
    fn test_variables() {
        assert_eq!(LoxValue::Nil, execute_stmt_and_expr("var x;", "x;").expect("Error"));
        assert_eq!(number(1.0), execute_stmt_and_expr("var x = 1;", "x;").expect("Error"));
        assert_eq!(string("Hi!"), execute_stmt_and_expr(r#"var x = "Hi!";"#, "x;").expect("Error"));
    }

    #[test]
    fn test_assignment() {
        assert_eq!(number(20.0), execute_stmt_and_expr("var x = 10; x = x * 2;", "x;").expect("Error"));
    }

    #[test]
    fn test_prints() {
        assert_eq!(String::from("1\n"), execute_stmts("print 1;"));
    }

    #[test]
    fn test_scopes() {
        assert_eq!(String::from("5\n5\n"), execute_stmts("var x = 0; {x = 5; print x;} print x;"));
        assert_eq!(String::from("5\n0\n"), execute_stmts("var x = 0; {var x = 5; print x;} print x;"));
    }

    #[test]
    fn test_if() {
        assert_eq!(String::from("1\n"), execute_stmts("if (true) print 1;"));
        assert_eq!(String::from(""), execute_stmts("if (false) print 1;"));
    }

    #[test]
    fn test_if_else() {
        assert_eq!(String::from("1\n"), execute_stmts("if (true) print 1; else print 2;"));
        assert_eq!(String::from("2\n"), execute_stmts("if (false) print 1; else print 2;"));
    }

    #[test]
    fn test_while() {
        assert_eq!(
            String::from("5\n"),
            execute_stmts(
                r#"
                var x = 10;
                var counter = 0;
                while (x > 0) {
                    x = x - 2;
                    counter = counter + 1;
                }
                print counter;
                "#
            )
        );

        assert_eq!(
            String::from("0\n"),
            execute_stmts(
                r#"
                var x = 10;
                var counter = 0;
                while (x > 10) {
                    x = x - 2;
                    counter = counter + 1;
                }
                print counter;
                "#
            )
        );

        assert_eq!(
            String::from("8\n6\n4\n2\n0\n"),
            execute_stmts(
                r#"
                var x = 10;
                while (x > 0) {
                    x = x - 2;
                    print x;
                }
                "#
            )
        );
    }

    #[test]
    fn test_for() {
        assert_eq!(
            String::from(
                "0\n1\n1\n2\n3\n5\n8\n13\n21\n34\n55\n89\n144\n233\n377\n610\n987\n1597\n2584\n4181\n6765\n"
            ),
            execute_stmts(
                r#"
                var a = 0;
                var temp;

                for (var b = 1; a < 10000; b = temp + b) {
                    print a;
                    temp = a;
                    a = b;
                }
                "#
            )
        );
    }

    #[test]
    fn test_break() {
        assert_eq!(
            String::from("0\n1\n1\n2\n3\n5\n8\n13\n21\n34\n55\n89\n"),
            execute_stmts(
                r#"
                var a = 0;
                var temp;

                for (var b = 1; a < 10000; b = temp + b) {
                    if (a > 100) break;
                    print a;
                    temp = a;
                    a = b;
                }
                "#
            )
        );
    }
}

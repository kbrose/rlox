use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    ast::{Expr, Stmt},
    interpreter::{
        EnvironmentWrapper,
        callables::LoxCallable,
        environment::{Environment, get_from_env_at, set_from_env_at},
        lox_object::{LoxFunction, LoxObject},
    },
    parser::IdentifierToken,
    scanner::TokenLike,
};
use anyhow::{Result as AnyhowResult, anyhow};
use std::io::Write;

pub(super) enum LoopControlFlow {
    Normal,
    Break,
}

#[derive(Debug)]
pub(crate) enum EvaluationException {
    Return(LoxObject),
    RuntimeError(RuntimeError),
}

impl From<RuntimeError> for EvaluationException {
    fn from(error: RuntimeError) -> Self {
        EvaluationException::RuntimeError(error)
    }
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

pub(crate) struct Interpreter<W: Write> {
    writer: W,
    globals: EnvironmentWrapper,
    pub(super) environment: EnvironmentWrapper,
    locals: HashMap<Expr, usize>,
}

impl<W: Write> Interpreter<W> {
    pub(crate) fn new(writer: W) -> Self {
        let globals = Rc::new(RefCell::new(Environment::new_root()));
        let environment = Rc::clone(&globals);
        Interpreter {
            writer,
            locals: HashMap::new(),
            globals,
            environment,
        }
    }

    pub(crate) fn resolve(&mut self, expr: Expr, depth: usize) {
        self.locals.insert(expr, depth);
    }

    fn look_up_variable(
        &self,
        name: &IdentifierToken,
        expr: &Expr,
    ) -> Result<LoxObject, EvaluationException> {
        if let Some(distance) = self.locals.get(expr) {
            get_from_env_at(Rc::clone(&self.environment), name, *distance).map_err(|e| e.into())
        } else {
            self.globals.borrow().get(name).map_err(|e| e.into())
        }
    }

    pub(crate) fn evaluate(&mut self, expr: &Expr) -> Result<LoxObject, EvaluationException> {
        match expr {
            Expr::LiteralExpr(literal) => {
                let out = match &literal.value {
                    crate::parser::ParsedLiteral::Nil => LoxObject::Nil,
                    crate::parser::ParsedLiteral::True => LoxObject::Boolean(true),
                    crate::parser::ParsedLiteral::False => LoxObject::Boolean(false),
                    crate::parser::ParsedLiteral::String(s) => LoxObject::String(s.clone()),
                    crate::parser::ParsedLiteral::Number(x) => LoxObject::Number(*x),
                };
                Ok(out)
            }
            Expr::Grouping(grouping) => self.evaluate(&grouping.expression),
            Expr::Unary(unary) => {
                let right = self.evaluate(&unary.expression)?;

                let out = match &unary.operator.op() {
                    crate::parser::UnaryOp::Bang => LoxObject::Boolean(!right.truthiness()),
                    crate::parser::UnaryOp::Minus => LoxObject::Number(
                        -right.get_number().map_err(|message| RuntimeError::new(&unary.operator, message))?,
                    ),
                };

                Ok(out)
            }
            Expr::Binary(binary) => {
                let left = self.evaluate(&binary.left)?;
                let right = self.evaluate(&binary.right)?;

                let err = |message: String| {
                    EvaluationException::RuntimeError(RuntimeError::new(&binary.operator, message))
                };

                match binary.operator.op() {
                    crate::parser::BinaryOp::Minus => Ok(LoxObject::Number(
                        left.get_number().map_err(err)? - right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Plus => {
                        if let (Ok(l), Ok(r)) = (left.get_number(), right.get_number()) {
                            Ok(LoxObject::Number(l + r))
                        } else if let (Ok(l), Ok(r)) = (left.get_string(), right.get_string()) {
                            let mut s = String::with_capacity(l.len() + r.len());
                            s.push_str(l);
                            s.push_str(r);
                            Ok(LoxObject::String(s))
                        } else {
                            Err(err(String::from("Type error: + with incompatible types")))
                        }
                    }
                    crate::parser::BinaryOp::Slash => {
                        let right = right.get_number().map_err(err)?;
                        if right == 0.0 {
                            Err(err(String::from("Division by zero")))
                        } else {
                            Ok(LoxObject::Number(left.get_number().map_err(err)? / right))
                        }
                    }
                    crate::parser::BinaryOp::Star => Ok(LoxObject::Number(
                        left.get_number().map_err(err)? * right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Greater => Ok(LoxObject::Boolean(
                        left.get_number().map_err(err)? > right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::GreaterEqual => Ok(LoxObject::Boolean(
                        left.get_number().map_err(err)? >= right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::Less => Ok(LoxObject::Boolean(
                        left.get_number().map_err(err)? < right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::LessEqual => Ok(LoxObject::Boolean(
                        left.get_number().map_err(err)? <= right.get_number().map_err(err)?,
                    )),
                    crate::parser::BinaryOp::BangEqual => Ok(LoxObject::Boolean(!left.is_equal(&right))),
                    crate::parser::BinaryOp::EqualEqual => Ok(LoxObject::Boolean(left.is_equal(&right))),
                    _ => panic!("Unexpected token type for binary!"), // TODO: Don't panic, make unrepresentable
                }
            }
            Expr::Variable(variable) => {
                self.look_up_variable(&variable.name, expr)
                // environment.borrow().get(&variable.name).map_err(|e| e.into())
            }
            Expr::Assign(assign) => {
                let value = self.evaluate(&assign.value)?;

                if let Some(distance) = self.locals.get(expr) {
                    // TODO: Any way to avoid the clone here?
                    set_from_env_at(Rc::clone(&self.environment), &assign.name, value.clone(), *distance);
                    Ok(value)
                } else {
                    // TODO: Any way to avoid the clone here?
                    match self.globals.borrow_mut().assign(&assign.name, value.clone()) {
                        Ok(()) => Ok(value),
                        Err(()) => {
                            Err(RuntimeError::new(&assign.name, "Undefined variable".to_string()).into())
                        }
                    }
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
            Expr::Call(call) => {
                let callee: Box<dyn LoxCallable<_>> = self
                    .evaluate(&call.callee)?
                    .get_callable::<W>()
                    .map_err(|e| RuntimeError::new(&call.open_paren, e))?;

                let mut arguments: Vec<LoxObject> = Vec::with_capacity(call.arguments.len());
                for argument in call.arguments.iter() {
                    arguments.push(self.evaluate(argument)?)
                }

                match callee.align_arguments(arguments) {
                    Ok(parsed_arguments) => {
                        match callee.call(self, parsed_arguments) {
                            // Reinterpret Return exceptions as Ok values.
                            Err(EvaluationException::Return(return_value)) => Ok(return_value),
                            catchall => catchall, // Other Ok(x) and RuntimeErrors get passed on
                        }
                    }
                    Err(argument_length_mismatch) => Err(RuntimeError::new(
                        &call.open_paren,
                        format!(
                            "Expected {} arguments, got {}.",
                            argument_length_mismatch.expected(),
                            argument_length_mismatch.provided()
                        ),
                    )
                    .into()),
                }
            }
        }
    }

    pub(super) fn execute_stmt(&mut self, stmt: &Stmt) -> Result<LoopControlFlow, EvaluationException> {
        match stmt {
            Stmt::StmtExpression(stmt_expression) => {
                self.evaluate(&stmt_expression.expression)?;
                Ok(LoopControlFlow::Normal)
            }
            Stmt::Print(print) => {
                let value = self.evaluate(&print.expression)?;
                writeln!(self.writer, "{}", value.to_string::<W>()).map(|_| LoopControlFlow::Normal).map_err(
                    |e| {
                        RuntimeError {
                            line: 0,
                            token_display: "".to_string(),
                            message: format!("Error writing to output stream: {e}"),
                        }
                        .into()
                    },
                )
            }
            Stmt::Var(var) => {
                let value = if let Some(initializer) = &var.initializer {
                    self.evaluate(&initializer)
                } else {
                    Ok(LoxObject::Nil)
                }?;

                self.environment.borrow_mut().define(&var.name, value);

                Ok(LoopControlFlow::Normal)
            }
            Stmt::Block(block) => {
                let previous_environment = Rc::clone(&self.environment);
                self.environment = crate::interpreter::environment::new_scope(&Rc::clone(&self.environment));
                let out = self.execute_block(block);
                self.environment = previous_environment;
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
            Stmt::Function(function) => {
                let closure = crate::interpreter::environment::new_scope(&Rc::clone(&self.environment));
                let defined_function =
                    LoxObject::LoxFunction(LoxFunction::new((**function).clone(), closure));
                self.environment.borrow_mut().define(&function.name, defined_function);
                Ok(LoopControlFlow::Normal)
            }
            Stmt::Return(return_stmt) => {
                let out = match &return_stmt.value {
                    Some(expr) => self.evaluate(&expr)?,
                    None => LoxObject::Nil,
                };
                Err(EvaluationException::Return(out))
            }
        }
    }

    fn execute_block(
        &mut self,
        block: &Box<crate::ast::Block>,
    ) -> Result<LoopControlFlow, EvaluationException> {
        for statement in block.statements.iter() {
            match self.execute_stmt(statement) {
                Ok(control_flow) => {
                    match control_flow {
                        LoopControlFlow::Normal => {}
                        LoopControlFlow::Break => {
                            return Ok(control_flow);
                        }
                    };
                }
                Err(runtime_error) => {
                    return Err(runtime_error);
                }
            }
        }
        Ok(LoopControlFlow::Normal)
    }

    pub(crate) fn interpret(&mut self, stmts: &[Stmt]) -> AnyhowResult<()> {
        for stmt in stmts {
            self.execute_stmt(stmt).map_err(|e| {
                match e {
                    EvaluationException::Return(_) => {
                        panic!(
                            "Encountered return outside of function, should have been caught as syntax error"
                        );
                    }
                    EvaluationException::RuntimeError(runtime_error) => {
                        eprintln!("{runtime_error}");
                    }
                }
                anyhow!("")
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::resolver::resolve;
    use crate::{parser::parse, scanner::scan_tokens};

    use super::*;

    /// Run the given expression and return the result.
    fn execute_str(s: &str) -> Result<LoxObject, EvaluationException> {
        let mut interpreter = Interpreter::new(std::io::sink());
        let mut with_semicolon = String::with_capacity(s.len() + 1);
        with_semicolon.push_str(s);
        with_semicolon.push(';');
        let parsed =
            parse(scan_tokens(&with_semicolon).expect("scan error"), std::io::stderr()).expect("parse error");

        resolve(&parsed, &mut std::io::sink(), &mut interpreter).expect("Error resolving.");

        assert!(parsed.len() == 1);
        match parsed.first().unwrap() {
            Stmt::StmtExpression(stmt_expression) => interpreter.evaluate(&stmt_expression.expression),
            _ => {
                panic!("Expected statement expression")
            }
        }
    }

    /// Execute the statements, then evaluate the expression and return the result.
    fn execute_stmt_and_expr(stmt: &str, expr: &str) -> Result<LoxObject, EvaluationException> {
        let mut interpreter = Interpreter::new(std::io::sink());

        let parsed = parse(scan_tokens(stmt).expect("scan error"), std::io::stderr()).expect("parse error");

        resolve(&parsed, &mut std::io::sink(), &mut interpreter).expect("Error resolving.");

        interpreter.interpret(&parsed).expect("interpreting error");

        let parsed = parse(scan_tokens(expr).expect("scan error"), std::io::stderr()).expect("parse error");
        assert!(parsed.len() == 1);
        match parsed.first().unwrap() {
            Stmt::StmtExpression(stmt_expression) => interpreter.evaluate(&stmt_expression.expression),
            _ => {
                panic!("Expected statement expression")
            }
        }
    }

    /// Execute the statements and return everything written out.
    fn execute_stmts(stmt: &str) -> String {
        let buffer = Vec::new();
        let mut interpreter = Interpreter::new(buffer);

        let parsed = parse(scan_tokens(stmt).expect("scan error"), std::io::stderr()).expect("parse error");

        resolve(&parsed, &mut std::io::sink(), &mut interpreter).expect("Error resolving.");

        interpreter.interpret(&parsed).expect("interpreting error");

        String::from_utf8(interpreter.writer).expect("Error with UTF8 encoding")
    }

    fn number(x: f64) -> LoxObject {
        LoxObject::Number(x)
    }

    fn string(s: &str) -> LoxObject {
        LoxObject::String(String::from(s))
    }

    fn boolean(b: bool) -> LoxObject {
        LoxObject::Boolean(b)
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
        assert_eq!(execute_str(r#"1 and nil"#).expect(INTER_ERR), LoxObject::Nil);
        assert_eq!(execute_str(r#"nil and 1"#).expect(INTER_ERR), LoxObject::Nil);

        assert_eq!(execute_str("1 or 2").expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str(r#""Hi!" or 1"#).expect(INTER_ERR), string("Hi!"));
        assert_eq!(execute_str(r#"false or 1"#).expect(INTER_ERR), number(1.0));
        assert_eq!(execute_str(r#"false or true"#).expect(INTER_ERR), boolean(true));
        assert_eq!(execute_str(r#"false or false"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"nil or false"#).expect(INTER_ERR), boolean(false));
        assert_eq!(execute_str(r#"false or nil"#).expect(INTER_ERR), LoxObject::Nil);
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
        assert_eq!(LoxObject::Nil, execute_stmt_and_expr("var x;", "x;").expect("Error"));
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

    #[test]
    fn test_clock() {
        assert_eq!(
            String::from("true\n"),
            execute_stmts(
                r#"
                var x = clock();

                print x >= 0.0;
                "#
            )
        );
    }

    #[test]
    fn test_user_defined_function() {
        assert_eq!(
            String::from("Hi, Dear Reader!\n"),
            execute_stmts(
                r#"
                fun sayHi(first, last) {
                    print "Hi, " + first + " " + last + "!";
                }

                sayHi("Dear", "Reader");
                "#
            )
        );

        assert_eq!(
            String::from("55\n"),
            execute_stmts(
                r#"
                fun fib(n) {
                  if (n <= 1) return n;
                  return fib(n - 2) + fib(n - 1);
                }

                print fib(10);
                "#
            )
        )
    }
}

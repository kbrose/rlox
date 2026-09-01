use std::{collections::HashMap, io::Write};

use crate::{
    ast::{Expr, Function, Stmt},
    interpreter::Interpreter,
    parser::IdentifierToken,
    scanner::TokenLike,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionType {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Clone, Copy)]
enum ClassType {
    None,
    Class,
    Subclass,
}

pub(crate) struct Resolver<'a, 'b, W1: Write, W2: Write> {
    scopes: Vec<HashMap<String, bool>>,
    had_error: bool,
    error_writer: &'a mut W1,
    interpreter: &'b mut Interpreter<W2>,
    current_function: FunctionType,
    current_class: ClassType,
}

impl<'a, 'b, W1: Write, W2: Write> Resolver<'a, 'b, W1, W2> {
    fn new(error_writer: &'a mut W1, interpreter: &'b mut Interpreter<W2>) -> Resolver<'a, 'b, W1, W2> {
        Resolver {
            scopes: Vec::new(),
            had_error: false,
            error_writer,
            interpreter,
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
    }

    fn error(&mut self, token: &impl TokenLike, message: &str) {
        self.had_error = true;
        writeln!(
            self.error_writer,
            "[Syntax Error: line {}, token: {}] {}",
            token.line(),
            token.token_display(),
            message
        )
        .expect("Error writing error...");
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &IdentifierToken) {
        if let Some(innermost_scope) = self.scopes.last_mut() {
            if innermost_scope.insert(name.identifier().to_string(), false).is_some() {
                self.error(name, "Already a variable with this name in this scope.");
            }
        }
    }

    fn define(&mut self, name: &IdentifierToken) {
        if let Some(innermost_scope) = self.scopes.last_mut() {
            innermost_scope.insert(name.identifier().to_string(), true);
        }
    }

    fn resolve_function(&mut self, function: &'b Function, function_type: FunctionType) {
        let enclosing = self.current_function;
        self.current_function = function_type;
        self.begin_scope();
        for param in function.params.iter() {
            self.declare(param);
            self.define(param);
        }
        for body_stmt in function.body.iter() {
            self.resolve_statement(body_stmt);
        }
        self.end_scope();
        self.current_function = enclosing;
    }

    fn resolve_local(&mut self, expr: &'b Expr, key: &str) {
        for (i, scope) in self.scopes.iter().rev().enumerate() {
            if scope.contains_key(key) {
                self.interpreter.resolve(expr.clone(), i);
            }
        }
    }

    fn resolve_expression(&mut self, expr: &'b Expr) {
        match expr {
            Expr::Assign(assign) => {
                self.resolve_expression(&assign.value);
                self.resolve_local(expr, assign.name.identifier());
            }
            Expr::Binary(binary) => {
                self.resolve_expression(&binary.left);
                self.resolve_expression(&binary.right);
            }
            Expr::Call(call) => {
                self.resolve_expression(&call.callee);
                for arg in call.arguments.iter() {
                    self.resolve_expression(arg);
                }
            }
            Expr::Logical(logical) => {
                self.resolve_expression(&logical.left);
                self.resolve_expression(&logical.right);
            }
            Expr::Grouping(grouping) => {
                self.resolve_expression(&grouping.expression);
            }
            Expr::LiteralExpr(_) => {}
            Expr::Unary(unary) => {
                self.resolve_expression(&unary.expression);
            }
            Expr::Variable(variable) => {
                let key = variable.name.identifier();
                if let Some(innermost_scope) = self.scopes.last() {
                    if innermost_scope.get(key) == Some(&false) {
                        self.error(&variable.name, "Can't read local variable in its own initializer.");
                    }
                }

                self.resolve_local(expr, key);
            }
            Expr::Get(get) => {
                self.resolve_expression(&get.object);
            }
            Expr::Set(set) => {
                self.resolve_expression(&set.object);
                self.resolve_expression(&set.value);
            }
            Expr::This(this) => match self.current_class {
                ClassType::None => self.error(&this.keyword, "Can't use 'this' outside of a class."),
                ClassType::Class | ClassType::Subclass => self.resolve_local(expr, "this"),
            },
            Expr::Super(s) => {
                match self.current_class {
                    ClassType::None => {
                        self.error(&s.keyword, "Can't use 'super' outside of a class.");
                    }
                    ClassType::Class => {
                        self.error(&s.keyword, "Can't use 'super' in a class with no superclass.");
                    }
                    ClassType::Subclass => {
                        self.resolve_local(expr, "super");
                    }
                };
                self.resolve_local(expr, "super");
            }
        }
    }

    fn resolve_statement(&mut self, stmt: &'b Stmt) {
        match stmt {
            Stmt::StmtExpression(stmt_expression) => {
                self.resolve_expression(&stmt_expression.expression);
            }
            Stmt::Function(function) => {
                self.declare(&function.name);
                self.define(&function.name);

                self.resolve_function(function, FunctionType::Function);
            }
            Stmt::If(if_stmt) => {
                self.resolve_expression(&if_stmt.condition);
                self.resolve_statement(&if_stmt.then_branch);
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.resolve_statement(else_branch);
                }
            }
            Stmt::Print(print) => {
                self.resolve_expression(&print.expression);
            }
            Stmt::Return(return_stmt) => {
                match self.current_function {
                    FunctionType::None => self.error(&return_stmt.token, "Can't return from top-level code."),
                    FunctionType::Initializer => {} // this may or may not be a problem depending on whether they returned a value.
                    FunctionType::Function => {}
                    FunctionType::Method => {}
                }
                if let Some(return_stmt_stmt) = &return_stmt.value {
                    if self.current_function == FunctionType::Initializer {
                        self.error(&return_stmt.token, "Can't return from initializer.")
                    }
                    self.resolve_expression(return_stmt_stmt);
                }
            }
            Stmt::Var(var) => {
                self.declare(&var.name);
                if let Some(initializer) = &var.initializer {
                    self.resolve_expression(initializer);
                }
                self.define(&var.name);
            }
            Stmt::While(while_stmt) => {
                self.resolve_expression(&while_stmt.condition);
                self.resolve_statement(&while_stmt.body);
            }
            Stmt::Block(block) => {
                self.begin_scope();
                for block_stmt in block.statements.iter() {
                    self.resolve_statement(block_stmt);
                }
                self.end_scope();
            }
            Stmt::Break(_) => {}
            Stmt::Class(class) => {
                let old_class_type = self.current_class;
                self.current_class = ClassType::Class;

                self.declare(&class.name);
                self.define(&class.name);

                let mut num_scopes = 0;

                if let Some(superclass) = &class.superclass {
                    self.current_class = ClassType::Subclass;
                    match superclass {
                        Expr::Variable(variable) => {
                            if variable.name.identifier() == class.name.identifier() {
                                self.error(&variable.name, "A class can't inherit from itself.");
                            }
                        }
                        _ => panic!("Only Variable can be a superclass expression"),
                    }
                    self.resolve_expression(superclass);

                    self.begin_scope();
                    num_scopes += 1;
                    self.scopes
                        .last_mut()
                        .expect("scopes is empty immediately after begin_scope()?")
                        .insert("super".to_string(), true);
                }

                self.begin_scope();
                num_scopes += 1;
                self.scopes
                    .last_mut()
                    .expect("scopes is empty immediately after begin_scope()?")
                    .insert("this".to_string(), true);

                for method in class.methods.iter() {
                    if method.name.identifier() == "init" {
                        self.resolve_function(method, FunctionType::Initializer);
                    } else {
                        self.resolve_function(method, FunctionType::Method);
                    }
                }

                for _ in 0..num_scopes {
                    self.end_scope();
                }

                self.current_class = old_class_type;
            }
        }
    }
}

pub(crate) fn resolve<'a, 'b, W1: Write, W2: Write>(
    stmts: &'b [Stmt],
    error_writer: &'a mut W1,
    interpreter: &'b mut Interpreter<W2>,
) -> Result<(), ()> {
    let mut resolver = Resolver::new(error_writer, interpreter);

    for stmt in stmts {
        resolver.resolve_statement(stmt);
    }

    if resolver.had_error {
        Err(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::Interpreter;
    use crate::parser::parse;
    use crate::scanner::scan_tokens;

    /// Returns True if statements do not error during resolving. Otherwise false.
    fn resolves_from_str(s: &str) -> bool {
        let mut error_writer = std::io::sink();
        let mut interpreter = Interpreter::new(std::io::sink());
        resolve(
            &parse(scan_tokens(s).expect("Error scanning"), &mut error_writer).expect("Error parsing"),
            &mut error_writer,
            &mut interpreter,
        )
        .is_ok()
    }

    #[test]
    fn test_use_of_var_in_declaration() {
        assert!(!resolves_from_str("var a; {var a=a;}"))
    }

    #[test]
    fn test_double_declare_at_non_global_scope() {
        assert!(!resolves_from_str("fun bad() {var a = 1; var a = 2;}"))
    }

    #[test]
    fn test_return_from_init() {
        assert!(!resolves_from_str("class Foo {init() {return 5;}}"))
    }

    // This test is not valid, I actually added support for catching this during
    // the parsing stage earlier (I went a little off-book). So parsing will
    // fail before we can see that resolving will fail. I peeked ahead and the
    // same kind of thing will be used for catching returns from init statements,
    // so we'll just have to wait before we can test this machinery.
    //
    // #[test]
    // fn test_top_level_return() {
    //     assert!(!resolves_from_str("return 5;"))
    // }

    #[test]
    fn test_super_misuses() {
        assert!(!resolves_from_str("super.hi();"));
        assert!(!resolves_from_str("class A {f() { super.hi(); }}"));
        assert!(resolves_from_str("class A < B {f() { super.hi(); }}"));
    }
}

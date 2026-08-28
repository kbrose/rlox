use std::{collections::HashMap, io::Write};

use crate::{
    ast::{Expr, Function, Stmt},
    interpreter::Interpreter,
    parser::IdentifierToken,
    scanner::TokenLike,
};

// struct Parser<'a, W: Write> {
//     tokens: &'a [Token],
//     current: usize,
//     parse_error: bool,
//     error_writer: &'a mut W,
//     loop_level: u8,
//     function_level: u8,
// }

pub(crate) struct Resolver<'a, 'b, W1: Write, W2: Write> {
    scopes: Vec<HashMap<String, bool>>,
    had_error: bool,
    error_writer: &'a mut W1,
    interpreter: &'b mut Interpreter<W2>,
}

impl<'a, 'b, W1: Write, W2: Write> Resolver<'a, 'b, W1, W2> {
    fn new(error_writer: &'a mut W1, interpreter: &'b mut Interpreter<W2>) -> Resolver<'a, 'b, W1, W2> {
        Resolver {
            scopes: Vec::new(),
            had_error: false,
            error_writer,
            interpreter,
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
            innermost_scope.insert(name.identifier().to_string(), false);
        }
    }

    fn define(&mut self, name: &IdentifierToken) {
        if let Some(innermost_scope) = self.scopes.last_mut() {
            innermost_scope.insert(name.identifier().to_string(), true);
        }
    }

    fn resolve_function(&mut self, function: &'b Function) {
        self.begin_scope();
        for param in function.params.iter() {
            self.declare(param);
            self.define(param);
        }
        for body_stmt in function.body.iter() {
            self.resolve_statement(body_stmt);
        }
        self.end_scope();
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

                self.resolve_function(function);
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
                if let Some(return_stmt_stmt) = &return_stmt.value {
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::parser::parse;
//     use crate::scanner::scan_tokens;

//     /// Returns True if statements do not error during resolving. Otherwise false.
//     fn resolve_from_str(s: &str) -> bool {
//         let mut error_writer = std::io::sink();
//         resolve(
//             &parse(scan_tokens(s).expect("Error scanning"), &mut error_writer).expect("Error parsing"),
//             &mut error_writer,
//         )
//         .is_ok()
//     }

//     #[test]
//     fn test_use_of_var_in_declaration() {
//         assert!(!resolve_from_str("var a; {var a=a;}"))
//     }
// }

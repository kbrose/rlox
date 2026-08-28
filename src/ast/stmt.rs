use crate::ast::Expr;
use crate::parser::{ErrorTrackingToken, IdentifierToken};

define_ast! {
    Stmt {
        StmtExpression {expression: Expr},
        Function {name: IdentifierToken, params: Vec<IdentifierToken>, body: Vec<Stmt>},
        If {condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>},
        Print {expression: Expr},
        Return {token: ErrorTrackingToken, value: Option<Expr>},
        Var {name: IdentifierToken, initializer: Option<Expr>},
        While {condition: Expr, body: Stmt},
        Block {statements: Vec<Stmt>},
        Class {name: IdentifierToken, methods: Vec<Function>},
        Break {},
    }
}

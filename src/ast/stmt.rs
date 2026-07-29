use crate::ast::Expr;
use crate::parser::IdentifierToken;

define_ast! {
    Stmt {
        StmtExpression {expression: Expr},
        IfStmt {condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>},
        Print {expression: Expr},
        Var {name: IdentifierToken, initializer: Option<Expr>},
        While {condition: Expr, body: Stmt},
        Block {statements: Vec<Stmt>},
    }
}

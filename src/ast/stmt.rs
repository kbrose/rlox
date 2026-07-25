use crate::ast::Expr;
use crate::parser::IdentifierToken;

define_ast! {
    Stmt {
        StmtExpression {expression: Expr},
        Print {expression: Expr},
        Var {name: IdentifierToken, initializer: Option<Expr>}
    }
}

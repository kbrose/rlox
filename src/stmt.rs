use crate::expr::Expr;
use crate::scanner::IdentifierToken;

define_ast! {
    Stmt {
        StmtExpression {expression: Expr},
        Print {expression: Expr},
        Var {name: IdentifierToken, initializer: Option<Expr>}
    }
}

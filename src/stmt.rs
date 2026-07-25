use crate::expr::Expr;

define_ast! {
    Stmt {
        StmtExpression {expression: Expr},
        Print {expression: Expr},
    }
}

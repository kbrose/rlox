use crate::scanner::Token;

define_ast! {
    Expr {
        Binary {left: Expr, operator: Token, right: Expr},
        Grouping {expression: Expr},
        Literal {value: Token},
        Unary {operator: Token, expression: Expr},
    }
}

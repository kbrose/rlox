use crate::scanner::{Token, TokenType};

define_ast! {
    Expr {
        Binary {left: Expr, operator: Token, right: Expr},
        Grouping {expression: Expr},
        Literal {value: TokenType},
        Unary {operator: Token, expression: Expr},
    }
}

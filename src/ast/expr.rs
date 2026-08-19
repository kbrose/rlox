use crate::parser::{BinaryToken, CallerToken, IdentifierToken, LogicalToken, ParsedLiteral, UnaryToken};

// TODO: Rename all of these to end in Expr for clarity?
define_ast! {
    Expr {
        Assign {name: IdentifierToken, value: Expr},
        Binary {left: Expr, operator: BinaryToken, right: Expr},
        Call {callee: Expr, open_paren: CallerToken, arguments: Vec<Expr>, close_paren: CallerToken},
        Logical {left: Expr, operator: LogicalToken, right: Expr},
        Grouping {expression: Expr},
        LiteralExpr {value: ParsedLiteral},
        Unary {operator: UnaryToken, expression: Expr},
        Variable {name: IdentifierToken}
    }
}

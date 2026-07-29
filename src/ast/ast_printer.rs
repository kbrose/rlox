use crate::ast::*;

#[allow(unused)]
pub(crate) fn pretty_print_expr(expr: &Expr) -> String {
    match expr {
        Expr::Binary(binary) => {
            format!(
                "({} {} {})",
                pretty_print_expr(&binary.left),
                binary.operator.pretty_print(),
                pretty_print_expr(&binary.right)
            )
        }
        Expr::Grouping(grouping) => {
            format!("({})", pretty_print_expr(&grouping.expression))
        }
        Expr::LiteralExpr(literal) => literal.value.pretty_print(),
        Expr::Unary(unary) => {
            format!("({} {})", unary.operator.pretty_print(), pretty_print_expr(&unary.expression))
        }
        Expr::Variable(variable) => {
            format!("var {}", variable.name.pretty_print())
        }
        Expr::Assign(assign) => {
            format!("{} = {}", assign.name.pretty_print(), pretty_print_expr(&assign.value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{BinaryOp, BinaryToken, ParsedLiteral, UnaryOp, UnaryToken};

    fn unary_token(op: UnaryOp) -> UnaryToken {
        UnaryToken::new(op, 1)
    }

    fn binary_token(op: BinaryOp) -> BinaryToken {
        BinaryToken::new(op, 1)
    }

    fn b<T>(t: T) -> Box<T> {
        Box::new(t)
    }

    #[test]
    fn test_pretty_print() {
        let exp: Expr = Expr::Binary(b(Binary {
            left: Expr::Unary(b(Unary {
                operator: unary_token(UnaryOp::Minus),
                expression: Expr::LiteralExpr(b(LiteralExpr {
                    value: ParsedLiteral::Number(123.0),
                })),
            })),
            operator: binary_token(BinaryOp::Star),
            right: Expr::Grouping(b(Grouping {
                expression: Expr::LiteralExpr(b(LiteralExpr {
                    value: ParsedLiteral::Number(45.67),
                })),
            })),
        }));
        assert_eq!(pretty_print_expr(&exp), "((- 123) * (45.67))");
    }
}

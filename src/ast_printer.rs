use crate::expr::*;

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
        Expr::Literal(literal) => literal.value.pretty_print(),
        Expr::Unary(unary) => {
            format!("({} {})", unary.operator.pretty_print(), pretty_print_expr(&unary.expression))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_printer::pretty_print_expr;
    use crate::scanner::{Token, TokenType};

    fn make_token(token_type: TokenType) -> Token {
        Token {
            line: 0,
            token_type,
        }
    }

    fn b<T>(t: T) -> Box<T> {
        Box::new(t)
    }

    #[test]
    fn test_pretty_print() {
        let exp: Expr = Expr::Binary(b(Binary {
            left: Expr::Unary(b(Unary {
                operator: make_token(TokenType::Minus),
                expression: Expr::Literal(b(Literal {
                    value: make_token(TokenType::Number(123.0)),
                })),
            })),
            operator: make_token(TokenType::Star),
            right: Expr::Grouping(b(Grouping {
                expression: Expr::Literal(b(Literal {
                    value: make_token(TokenType::Number(45.67)),
                })),
            })),
        }));
        assert_eq!(pretty_print_expr(&exp), "((- 123) * (45.67))");
    }
}

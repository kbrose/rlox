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
        Expr::Logical(logical) => {
            format!(
                "({} {} {})",
                pretty_print_expr(&logical.left),
                logical.operator.pretty_print(),
                pretty_print_expr(&logical.right)
            )
        }
        Expr::Call(call) => {
            format!(
                "{}({})",
                pretty_print_expr(&call.callee),
                call.arguments.iter().map(pretty_print_expr).collect::<Vec<_>>().join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_expression;
    use crate::scanner::scan_tokens;

    #[test]
    fn test_pretty_print() {
        let expression =
            parse_expression(scan_tokens("-123 * (45.67)").expect("Error scanning"), std::io::sink())
                .expect("Error parsing");
        assert_eq!(pretty_print_expr(&expression), "((- 123) * (45.67))");

        let expression =
            parse_expression(scan_tokens("x    =      1").expect("Error scanning"), std::io::sink())
                .expect("Error parsing");
        assert_eq!(pretty_print_expr(&expression), "x = 1");
    }
}

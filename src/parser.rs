use crate::{
    expr::{Binary, Expr, Grouping, Literal, Unary},
    scanner::{Token, TokenType},
    stmt::{Print, Stmt, StmtExpression},
};
use anyhow::{Result, anyhow};
use std::fmt;

// Statement:
// program        → statement* EOF ;
// statement      → exprStmt
//                | printStmt ;
// exprStmt       → expression ";" ;
// printStmt      → "print" expression ";" ;
//
// Expression:
// expression     → comma ;
// comma          → equality ( "," equality )* ;
// equality       → comparison ( ( "!=" | "==" ) comparison )* ;
// comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term           → factor ( ( "-" | "+" ) factor )* ;
// factor         → unary ( ( "/" | "*" ) unary )* ;
// unary          → ( "!" | "-" ) unary
//                | primary ;
// primary        → NUMBER | STRING | "true" | "false" | "nil"
//                | "(" expression ")" ;

// TODO: Better names for these
type EResult = Result<Expr>;
type SResult = Result<Stmt>;

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    parse_error: bool,
}

#[derive(Debug)]
struct ParserError {
    token: Token,
    message: String,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Parse Error line: {}, token: {}] {}",
            self.token.line,
            self.token.token_type.pretty_print(),
            self.message
        )
    }
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            parse_error: false,
        }
    }

    fn parse(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = vec![];
        while !self.is_at_end() {
            statements.push(self.statement()?);
        }
        if self.parse_error {
            Err(anyhow!("Parsing error (recoverable)."))
        } else {
            Ok(statements)
        }
    }

    fn matches_token(&mut self, targets: &[TokenType]) -> bool {
        for target in targets {
            if self.check(target) {
                self.advance();
                return true;
            }
        }
        return false;
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            match (&self.peek().token_type, token_type) {
                // Special case our content-carrying enum options, equality is trickier here.
                (TokenType::String(_), TokenType::String(_)) => true,
                (TokenType::Number(_), TokenType::Number(_)) => true,
                (TokenType::Identifier(_), TokenType::Identifier(_)) => true,
                // Fall back to equality
                _ => &self.peek().token_type == token_type,
            }
        }
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> Option<&Token> {
        self.tokens.get(self.current.wrapping_sub(1))
    }

    fn consume(&mut self, expected: TokenType, message: &str) -> Result<Option<&Token>> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(anyhow!("{}", self.error(self.peek().clone(), message)))
        }
    }

    #[allow(unused)]
    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            let prev = self.previous().expect("Empty previous even after advancing?");

            if prev.token_type == TokenType::Semicolon {
                break;
            }

            match self.peek().token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    break;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn error(&mut self, cause: Token, message: &str) -> ParserError {
        self.parse_error = true;
        let error = ParserError {
            token: cause,
            message: message.to_string(),
        };
        eprintln!("{}", error);
        error
    }

    fn binary_op_missing_lhs_error_production<F>(&mut self, mut next_grammar: F) -> EResult
    where
        F: FnMut(&mut Self) -> EResult,
    {
        self.error(self.peek().clone(), "Expected expression, found binary operator");
        // Throwaway the extra RHS. Forward the hard error if there was one.
        let _ = next_grammar(self)?;
        self.expression() // Restart at the bottom of the hierarchy.
    }

    fn left_associative_binary_op<F>(&mut self, targets: &[TokenType], mut next_grammar: F) -> EResult
    where
        F: FnMut(&mut Self) -> EResult,
    {
        let mut expr = next_grammar(self)?;

        while self.matches_token(targets) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let right = next_grammar(self)?;
            expr = Binary::lift(expr, operator, right)
        }

        Ok(expr)
    }

    fn statement(&mut self) -> SResult {
        if self.matches_token(&[TokenType::Print]) {
            self.print_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> SResult {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Print::lift(expression))
    }

    fn expression_statement(&mut self) -> SResult {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(StmtExpression::lift(expression))
    }

    fn expression(&mut self) -> EResult {
        self.comma()
    }

    fn comma(&mut self) -> EResult {
        self.left_associative_binary_op(&[TokenType::Comma], Self::equality)
    }

    fn equality(&mut self) -> EResult {
        self.left_associative_binary_op(&[TokenType::BangEqual, TokenType::EqualEqual], Self::comparison)
    }

    fn comparison(&mut self) -> EResult {
        self.left_associative_binary_op(
            &[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual],
            Self::term,
        )
    }

    fn term(&mut self) -> EResult {
        self.left_associative_binary_op(&[TokenType::Minus, TokenType::Plus], Self::factor)
    }

    fn factor(&mut self) -> EResult {
        self.left_associative_binary_op(&[TokenType::Slash, TokenType::Star], Self::unary)
    }

    fn unary(&mut self) -> EResult {
        if self.matches_token(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let expression = self.unary()?;
            Ok(Unary::lift(operator, expression))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> EResult {
        // let mut binary_op_error_production = ||

        if self.matches_token(&[TokenType::False]) {
            Ok(Literal::lift(TokenType::False))
        } else if self.matches_token(&[TokenType::True]) {
            Ok(Literal::lift(TokenType::True))
        } else if self.matches_token(&[TokenType::Nil]) {
            Ok(Literal::lift(TokenType::Nil))
        } else if self.matches_token(&[TokenType::Number(0.0), TokenType::String(String::new())]) {
            // Note the 0.0 and String::new() values don't matter.
            // See implementation of matches_token for more.
            Ok(Literal::lift(
                self.previous().expect("Empty previous even after advancing?").token_type.clone(),
            ))
        } else if self.matches_token(&[TokenType::LeftParen]) {
            let expression = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            Ok(Grouping::lift(expression))
        } else if self.matches_token(&[TokenType::Comma]) {
            self.binary_op_missing_lhs_error_production(Self::equality)
        } else if self.matches_token(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            self.binary_op_missing_lhs_error_production(Self::comparison)
        } else if self.matches_token(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            self.binary_op_missing_lhs_error_production(Self::term)
        } else if self.matches_token(&[TokenType::Plus]) {
            self.binary_op_missing_lhs_error_production(Self::factor)
        } else if self.matches_token(&[TokenType::Slash, TokenType::Star]) {
            self.binary_op_missing_lhs_error_production(Self::unary)
        } else {
            Err(anyhow!("{}", self.error(self.peek().clone(), "Expected expresion.")))
        }
    }
}

pub(crate) fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{Token, TokenType, scan_tokens};

    fn make_token(token_type: TokenType) -> Token {
        Token {
            line: 1,
            token_type,
        }
    }

    fn literal_num(n: f64) -> Expr {
        Literal::lift(TokenType::Number(n))
    }

    fn literal_str(s: &str) -> Expr {
        Literal::lift(TokenType::String(s.to_string()))
    }

    fn single_expr(expr: Expr) -> Vec<Stmt> {
        vec![StmtExpression::lift(expr)]
    }

    #[test]
    fn test_parse_simple() {
        assert_eq!(
            parse(scan_tokens(&"123;").expect("Error scanning")).expect("Error parsing"),
            single_expr(literal_num(123.0))
        );

        assert_eq!(
            parse(scan_tokens(&r#""123";"#).expect("Error scanning")).expect("Error parsing"),
            single_expr(literal_str("123"))
        );
    }

    #[test]
    fn test_comma() {
        assert_eq!(
            parse(scan_tokens(&"1, 2;").expect("Error scanning.")).expect("Error parsing."),
            single_expr(Binary::lift(literal_num(1.0), make_token(TokenType::Comma), literal_num(2.0)))
        )
    }

    #[test]
    fn test_parse_complex() {
        let expected = single_expr(Binary::lift(
            Unary::lift(make_token(TokenType::Minus), literal_num(123.0)),
            make_token(TokenType::Star),
            Grouping::lift(literal_num(45.67)),
        ));

        assert_eq!(
            parse(scan_tokens(&"-123 * (45.67);").expect("Error scanning")).expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(scan_tokens(&"-123.0 /* comment */ * (45.67);").expect("Error scanning"))
                .expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(scan_tokens(&"-123 /* comment */ * (45.67);  //").expect("Error scanning"))
                .expect("Error parsing"),
            expected
        );

        let expected = single_expr(Binary::lift(
            Unary::lift(make_token(TokenType::Minus), literal_num(123.0)),
            make_token(TokenType::EqualEqual),
            Grouping::lift(literal_num(45.67)),
        ));

        assert_eq!(
            parse(scan_tokens(&"-123 == (45.67);").expect("Error scanning")).expect("Error parsing"),
            expected
        );
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse(scan_tokens(&"+123;").expect("Error scanning")).is_err());
        assert!(parse(scan_tokens(&"class;").expect("Error scanning")).is_err());
    }

    // #[test]
    // fn test_parse_recoverable_errors() {
    //     for op in [",", "!=", "==", ">", ">=", "<", "<=", "+", "/", "*"] {
    //         let tokens = scan_tokens(&format!("{}123 5;", op)).expect("Error scanning");
    //         let mut parser = Parser::new(tokens);
    //         let expr = parser.parse_unchecked().expect("Parsing had hard fail, expected recoverable.");
    //         assert_eq!(expr, single_expr(Literal::lift(TokenType::Number(5.0))));
    //     }
    // }
}

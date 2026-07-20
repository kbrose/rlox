use crate::{
    expr::{Binary, Expr, Grouping, Literal, Unary},
    scanner::{Token, TokenType},
};
use anyhow::{Result, anyhow};
use std::fmt;

// expression     → equality ;
// equality       → comparison ( ( "!=" | "==" ) comparison )* ;
// comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term           → factor ( ( "-" | "+" ) factor )* ;
// factor         → unary ( ( "/" | "*" ) unary )* ;
// unary          → ( "!" | "-" ) unary
//                | primary ;
// primary        → NUMBER | STRING | "true" | "false" | "nil"
//                | "(" expression ")" ;

struct Parser {
    tokens: Vec<Token>,
    current: usize,
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
        }
    }

    fn parse(&mut self) -> Result<Expr> {
        self.expression()
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
            Err(anyhow!("{}", self.error(self.peek(), message)))
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

    fn error(&self, cause: &Token, message: &str) -> ParserError {
        let error = ParserError {
            token: cause.clone(),
            message: message.to_string(),
        };
        eprintln!("{}", error);
        error
    }

    fn left_associative_binary_op<F>(
        &mut self,
        targets: &[TokenType],
        mut next_higher_precedence: F,
    ) -> Result<Expr>
    where
        F: FnMut(&mut Self) -> Result<Expr>,
    {
        let mut expr = next_higher_precedence(self)?;

        while self.matches_token(&targets) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let right = next_higher_precedence(self)?;
            expr = Expr::Binary(Box::new(Binary {
                left: expr,
                operator,
                right,
            }))
        }

        Ok(expr)
    }

    fn expression(&mut self) -> Result<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr> {
        self.left_associative_binary_op(&[TokenType::BangEqual, TokenType::EqualEqual], Self::comparison)
    }

    fn comparison(&mut self) -> Result<Expr> {
        self.left_associative_binary_op(
            &[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual],
            Self::term,
        )
    }

    fn term(&mut self) -> Result<Expr> {
        self.left_associative_binary_op(&[TokenType::Minus, TokenType::Plus], Self::factor)
    }

    fn factor(&mut self) -> Result<Expr> {
        self.left_associative_binary_op(&[TokenType::Slash, TokenType::Star], Self::unary)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.matches_token(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let expression = self.unary()?;
            Ok(Expr::Unary(Box::new(Unary {
                operator,
                expression,
            })))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr> {
        if self.matches_token(&[TokenType::False]) {
            Ok(Expr::Literal(Box::new(Literal {
                value: TokenType::False,
            })))
        } else if self.matches_token(&[TokenType::True]) {
            Ok(Expr::Literal(Box::new(Literal {
                value: TokenType::True,
            })))
        } else if self.matches_token(&[TokenType::Nil]) {
            Ok(Expr::Literal(Box::new(Literal {
                value: TokenType::Nil,
            })))
        } else if self.matches_token(&[TokenType::Number(0.0), TokenType::String(String::new())]) {
            // Note the 0.0 and String::new() values don't matter.
            // See implementation of matches_token for more.
            Ok(Expr::Literal(Box::new(Literal {
                value: self.previous().expect("Empty previous even after advancing?").token_type.clone(),
            })))
        } else if self.matches_token(&[TokenType::LeftParen]) {
            let expression = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            Ok(Expr::Grouping(Box::new(Grouping {
                expression,
            })))
        } else {
            Err(anyhow!("{}", self.error(self.peek(), "Expected expresion.")))
        }
    }
}

pub(crate) fn parse(tokens: Vec<Token>) -> Result<Expr> {
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

    fn b<T>(t: T) -> Box<T> {
        Box::new(t)
    }

    #[test]
    fn test_parse_simple() {
        assert_eq!(
            parse(scan_tokens(&"123").expect("Error scanning")).expect("Error parsing"),
            Expr::Literal(Box::new(Literal {
                value: TokenType::Number(123.0)
            }))
        );

        assert_eq!(
            parse(scan_tokens(&"\"123\"").expect("Error scanning")).expect("Error parsing"),
            Expr::Literal(Box::new(Literal {
                value: TokenType::String("123".to_string())
            }))
        );
    }

    #[test]
    fn test_parse_complex() {
        let expected: Expr = Expr::Binary(b(Binary {
            left: Expr::Unary(b(Unary {
                operator: make_token(TokenType::Minus),
                expression: Expr::Literal(b(Literal {
                    value: TokenType::Number(123.0),
                })),
            })),
            operator: make_token(TokenType::Star),
            right: Expr::Grouping(b(Grouping {
                expression: Expr::Literal(b(Literal {
                    value: TokenType::Number(45.67),
                })),
            })),
        }));

        assert_eq!(
            parse(scan_tokens(&"-123 * (45.67)").expect("Error scanning")).expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(scan_tokens(&"-123.0 /* comment */ * (45.67)").expect("Error scanning"))
                .expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(scan_tokens(&"-123 /* comment */ * (45.67)  //").expect("Error scanning"))
                .expect("Error parsing"),
            expected
        );

        let expected: Expr = Expr::Binary(b(Binary {
            left: Expr::Unary(b(Unary {
                operator: make_token(TokenType::Minus),
                expression: Expr::Literal(b(Literal {
                    value: TokenType::Number(123.0),
                })),
            })),
            operator: make_token(TokenType::EqualEqual),
            right: Expr::Grouping(b(Grouping {
                expression: Expr::Literal(b(Literal {
                    value: TokenType::Number(45.67),
                })),
            })),
        }));

        assert_eq!(
            parse(scan_tokens(&"-123 == (45.67)").expect("Error scanning")).expect("Error parsing"),
            expected
        );
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse(scan_tokens(&"+123").expect("Error scanning")).is_err());
        assert!(parse(scan_tokens(&"class").expect("Error scanning")).is_err());
    }
}

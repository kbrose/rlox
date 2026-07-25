use crate::{
    ast::{
        Assign, Binary, Block, Expr, Grouping, LiteralExpr, Print, Stmt, StmtExpression, Unary, Var, Variable,
    },
    parser::parsed_token::{BinaryOp, BinaryToken, IdentifierToken, ParsedLiteral, UnaryOp, UnaryToken},
    scanner::{Token, TokenType},
};

// TODO: Better names for these
type EResult = Result<Expr, ParserError>;
type SResult = Result<Stmt, ParserError>;

//                        Lox Grammar
// ===========================================================
//
//                        Statements
//
// program        → declaration* EOF ;
// declaration    → varDecl
//                | statement ;
// statement      → exprStmt
//                | printStmt
//                | block ;
// varDecl        → "var" IDENTIFIER ( "=" expression )? ";" ;
// exprStmt       → expression ";" ;
// printStmt      → "print" expression ";" ;
// block          → "{" declaration* "}" ;
//
//                        Expressions
//
// expression     → assignment ;
// assignment     → IDENTIFIER "=" assignment
//                | comma ;
// comma          → equality ( "," equality )* ;
// equality       → comparison ( ( "!=" | "==" ) comparison )* ;
// comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term           → factor ( ( "-" | "+" ) factor )* ;
// factor         → unary ( ( "/" | "*" ) unary )* ;
// unary          → ( "!" | "-" ) unary
//                | primary ;
// primary        → NUMBER | STRING | "true" | "false" | "nil"
//                | "(" expression ")" ;
//                | IDENTIFIER
struct Parser {
    tokens: Vec<Token>,
    current: usize,
    parse_error: bool,
}

#[derive(Debug)]
pub(crate) struct ParserError {}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParserError{{}}")
    }
}

impl std::error::Error for ParserError {
    fn description(&self) -> &str {
        "Parser error"
    }
}

fn error(line: usize, token_str: String, message: &str) {
    eprintln!("[Parse Error line: {}, token: {}] {}", line, token_str, message);
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            parse_error: false,
        }
    }

    fn parse(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut statements = vec![];
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        if self.parse_error {
            Err(ParserError {})
        } else {
            Ok(statements)
        }
    }

    // TODO: It would be nice if this returned Option<...>: the item that passed the .check()
    fn matches_token(&mut self, targets: &[TokenType]) -> bool {
        for target in targets {
            if self.check(target) {
                self.advance();
                return true;
            }
        }
        return false;
    }

    // TODO: Should also return Option<...>?
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

    fn consume(&mut self, expected: TokenType, message: &str) -> Result<Option<&Token>, ParserError> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            error(token.line, token.token_type.pretty_print(), message);
            Err(ParserError {})
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<Option<IdentifierToken>, ParserError> {
        if self.check(&TokenType::Identifier(String::new())) {
            Ok(self.advance().map(|token| match &token.token_type {
                TokenType::Identifier(identifier) => IdentifierToken::new(identifier.clone(), token.line),
                _ => panic!("Checked for identifier, but now advance() doesn't return identifier?"),
            }))
        } else {
            let token = self.peek();
            error(token.line, token.token_type.pretty_print(), message);
            Err(ParserError {})
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

    fn binary_op_missing_lhs_error_production<F>(&mut self, mut next_grammar: F) -> EResult
    where
        F: FnMut(&mut Self) -> EResult,
    {
        let token = self.peek();
        error(token.line, token.token_type.pretty_print(), "Expected expression, found binary operator");
        // Throwaway the extra RHS. Forward the hard error if there was one.
        let _ = next_grammar(self)?;
        self.expression() // Restart at the bottom of the hierarchy.
    }

    fn left_associative_binary_op<F>(&mut self, targets: &[TokenType], mut next_grammar: F) -> EResult
    where
        F: FnMut(&mut Self) -> EResult,
    {
        let mut expr = next_grammar(self)?;

        // TODO: Is there a better way to do alignment?
        // Panic isn't bad since it's more like "it should be impossible to see this",
        // but we could still make it more robust by reporting a syntax error, even if we
        // think it would be impossible to ever observe. Would love to make it unrepresentable,
        // though!
        while self.matches_token(targets) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let line = operator.line;
            let op: BinaryOp = match operator.token_type {
                TokenType::Comma => BinaryOp::Comma,
                TokenType::Minus => BinaryOp::Minus,
                TokenType::Plus => BinaryOp::Plus,
                TokenType::Slash => BinaryOp::Slash,
                TokenType::Star => BinaryOp::Star,
                TokenType::BangEqual => BinaryOp::BangEqual,
                TokenType::EqualEqual => BinaryOp::EqualEqual,
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                _ => panic!(
                    "Expected binary operator: checked for {:?}, got {:?}",
                    targets, operator.token_type
                ),
            };
            let operator: BinaryToken = BinaryToken::new(op, line);
            let right = next_grammar(self)?;
            expr = Binary::lift(expr, operator, right)
        }

        Ok(expr)
    }

    fn declaration(&mut self) -> SResult {
        let out = if self.matches_token(&[TokenType::Var]) {
            self.var_declaration()
        } else {
            self.statement()
        };

        match out {
            Ok(stmt) => Ok(stmt),
            Err(e) => {
                self.synchronize();
                Err(e)
            }
        }
    }

    fn var_declaration(&mut self) -> SResult {
        let name =
            self.consume_identifier("Expect variable name.")?.expect("Empty previous even after advancing?");

        let initializer = if self.matches_token(&[TokenType::Equal]) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(TokenType::Semicolon, "Expect ';' after variable declaration.")?;

        Ok(Var::lift(name, initializer))
    }

    fn statement(&mut self) -> SResult {
        if self.matches_token(&[TokenType::Print]) {
            self.print_statement()
        } else if self.matches_token(&[TokenType::LeftBrace]) {
            self.block()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> SResult {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Print::lift(expression))
    }

    fn block(&mut self) -> SResult {
        let mut statements = vec![];

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(Block::lift(statements))
    }

    fn expression_statement(&mut self) -> SResult {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(StmtExpression::lift(expression))
    }

    fn expression(&mut self) -> EResult {
        self.assignment()
    }

    fn assignment(&mut self) -> EResult {
        let expression = self.comma()?;

        if self.matches_token(&[TokenType::Equal]) {
            // This token is ~guaranteed to be an Equal token, but we only need it
            // for error reporting.
            let (line, token_str) = {
                let token = self.previous().expect("Empty previous after advance");
                (token.line, token.token_type.pretty_print())
            };

            let value = self.assignment()?;

            match expression {
                Expr::Variable(variable) => {
                    let name = variable.name;
                    return Ok(Assign::lift(name, value));
                }
                _ => {
                    error(line, token_str, "Invalid assignment target.");
                }
            }
        }

        Ok(expression)
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
        let unary_token_types = [TokenType::Bang, TokenType::Minus];
        if self.matches_token(&unary_token_types) {
            let operator = self.previous().expect("Empty previous even after advancing?").clone();
            let line = operator.line;
            let op: UnaryOp = match operator.token_type {
                TokenType::Bang => UnaryOp::Bang,
                TokenType::Minus => UnaryOp::Minus,
                _ => panic!(
                    "Expected unary operator: checked for {:?}, got {:?}",
                    unary_token_types, operator.token_type
                ),
            };
            let operator: UnaryToken = UnaryToken::new(op, line);
            let expression = self.unary()?;
            Ok(Unary::lift(operator, expression))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> EResult {
        // let mut binary_op_error_production = ||

        if self.matches_token(&[TokenType::False]) {
            Ok(LiteralExpr::lift(ParsedLiteral::False))
        } else if self.matches_token(&[TokenType::True]) {
            Ok(LiteralExpr::lift(ParsedLiteral::True))
        } else if self.matches_token(&[TokenType::Nil]) {
            Ok(LiteralExpr::lift(ParsedLiteral::Nil))
        } else if self.matches_token(&[TokenType::Number(0.0), TokenType::String(String::new())]) {
            // Note the 0.0 and String::new() values don't matter.
            // See implementation of matches_token for more.
            let token_type = &self.previous().expect("Empty previous even after advancing?").token_type;
            match &token_type {
                TokenType::String(s) => Ok(LiteralExpr::lift(ParsedLiteral::String(s.clone()))),
                TokenType::Number(x) => Ok(LiteralExpr::lift(ParsedLiteral::Number(*x))),
                _ => {
                    panic!("Checked for string or number, but now not finding one? Got {:?}", token_type)
                }
            }
        } else if self.matches_token(&[TokenType::Identifier(String::new())]) {
            let prev = self.previous().expect("Empty previous even after advancing?");
            match &prev.token_type {
                TokenType::Identifier(identifier) => {
                    Ok(Variable::lift(IdentifierToken::new(identifier.clone(), prev.line)))
                }
                _ => panic!("Matched on identifier, but then previous() didn't return identifier"),
            }
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
            let token = self.peek();
            error(token.line, token.token_type.pretty_print(), "Expected expression.");
            Err(ParserError {})
        }
    }
}

pub(crate) fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, ParserError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_tokens;

    fn unary_token(op: UnaryOp) -> UnaryToken {
        UnaryToken::new(op, 1)
    }

    fn binary_token(op: BinaryOp) -> BinaryToken {
        BinaryToken::new(op, 1)
    }

    fn literal_num(n: f64) -> Expr {
        LiteralExpr::lift(ParsedLiteral::Number(n))
    }

    fn literal_str(s: &str) -> Expr {
        LiteralExpr::lift(ParsedLiteral::String(s.to_string()))
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
            single_expr(Binary::lift(literal_num(1.0), binary_token(BinaryOp::Comma), literal_num(2.0)))
        )
    }

    #[test]
    fn test_parse_complex() {
        let expected = single_expr(Binary::lift(
            Unary::lift(unary_token(UnaryOp::Minus), literal_num(123.0)),
            binary_token(BinaryOp::Star),
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
            Unary::lift(unary_token(UnaryOp::Minus), literal_num(123.0)),
            binary_token(BinaryOp::EqualEqual),
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

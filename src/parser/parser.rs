use std::io::Write;

use crate::{
    ast::{
        Assign, Binary, Block, Break, Call, Expr, Function, Grouping, If, LiteralExpr, Logical, Print,
        Return, Stmt, StmtExpression, Unary, Var, Variable, While,
    },
    parser::{
        ErrorTrackingToken,
        parsed_token::{
            BinaryOp, BinaryToken, IdentifierToken, LogicalOp, LogicalToken, ParsedLiteral, UnaryOp,
            UnaryToken,
        },
    },
    scanner::{Token, TokenLike, TokenType},
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
// declaration    → funDecl
//                | varDecl
//                | statement ;
// funDecl        → "fun" function ;
// function       → IDENTIFIER "(" parameters? ")" block ;
// parameters     → IDENTIFIER ( "," IDENTIFIER )* ;
// varDecl        → "var" IDENTIFIER ( "=" expression )? ";" ;
// statement      → exprStmt
//                | forStmt
//                | ifStmt
//                | printStmt
//                | returnStmt
//                | whileStmt
//                | breakStmt
//                | block ;
// exprStmt       → expression ";" ;
// forStmt        → "for" "(" (varDecl | exprStmt | ";")
//                  expression? ";"
//                  expression? ")" statement ;
// ifStmt         → "if" "(" expression ")" statement
//                ( "else" statement )? ;
// printStmt      → "print" expression ";" ;
// returnStmt     → "return" expression? ";" ;
// whileStmt      → "while" "(" expression ")" statement" ;
// breakStmt      → "break" ";" ;
// block          → "{" declaration* "}" ;
//
//                        Expressions
//
// expression     → assignment ;
// assignment     → IDENTIFIER "=" assignment
//                | logic_or ;
// logic_or       → logic_and ( "or" logic_and )* ;
// logic_and      → comma ( "and" comma )* ;
// comma          → equality ( "," equality )* ;
// equality       → comparison ( ( "!=" | "==" ) comparison )* ;
// comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term           → factor ( ( "-" | "+" ) factor )* ;
// factor         → unary ( ( "/" | "*" ) unary )* ;
// unary          → ( "!" | "-" ) unary | call ;
// call           → primary ( "(" arguments? ")" )* ;
// arguments      → expression ( "," expression )* ;
// primary        → NUMBER | STRING | "true" | "false" | "nil"
//                | "(" expression ")" ;
//                | IDENTIFIER
struct Parser<'a, W: Write> {
    tokens: &'a [Token],
    current: usize,
    parse_error: bool,
    error_writer: &'a mut W,
    loop_level: u8,
    function_level: u8,
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

impl<'a, W: Write> Parser<'a, W> {
    pub(crate) fn new(tokens: &'a [Token], error_writer: &'a mut W) -> Self {
        Parser {
            tokens,
            current: 0,
            parse_error: false,
            error_writer,
            loop_level: 0,
            function_level: 0,
        }
    }

    // Helpers

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

    fn error(&mut self, line: usize, token_str: String, message: &str) {
        writeln!(self.error_writer, "[Parse Error line: {}, token: {}] {}", line, token_str, message)
            .expect("Error writing error...");
    }

    // TODO: It would be nice if this returned Option<...>: the item that passed the .check()
    /// Checks if the next token matches any of the targets, and if so, consumes it.
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
    /// Check if the next token is of the given type. Does not consume it either way.
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

    /// Consume the next token and attempts to return it.
    /// `None` may be returned if the full program is just <EOF>
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
            self.error(token.line, token.token_type.pretty_print(), message);
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
            self.error(token.line, token.token_type.pretty_print(), message);
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
        self.error(token.line, token.token_type.pretty_print(), "Expected expression, found binary operator");
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
            let operator = self.previous().expect("Empty previous even after advancing?");
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

    // Statements

    fn declaration(&mut self) -> SResult {
        let out = if self.matches_token(&[TokenType::Fun]) {
            self.function("function")
        } else if self.matches_token(&[TokenType::Var]) {
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

    fn function(&mut self, kind: &str) -> SResult {
        let name = IdentifierToken::new_from_token(
            self.consume(TokenType::Identifier(String::new()), &format!("Expect {kind} name."))?
                .expect("Missing token after consuming it?"),
        );

        self.consume(TokenType::LeftParen, &format!("Expect '(' after {kind} name."))?;

        let mut params = vec![];

        if !self.check(&TokenType::RightParen) {
            loop {
                if params.len() >= 255 {
                    let token = self.peek();
                    self.error(token.line, token.token_display(), "Can't have more than 255 parameters.");
                }

                params.push(IdentifierToken::new_from_token(
                    self.consume(TokenType::Identifier(String::new()), "Expect parameter name.")?
                        .expect("Missing token after consuming it?"),
                ));

                if !self.matches_token(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, &format!("Expect ')' after {kind} parameters."))?;

        self.consume(TokenType::LeftBrace, &format!("Expect '{{' before {kind} body."))?;

        let body = self.returnable_statement(Self::block)?;

        Ok(Function::lift(name, params, body))
    }

    fn returnable_statement<F>(&mut self, mut f: F) -> SResult
    where
        F: FnMut(&mut Self) -> SResult,
    {
        self.increment_function_level()?;
        let out = f(self);
        self.decrement_function_level();
        out
    }

    fn increment_function_level(&mut self) -> Result<(), ParserError> {
        match self.function_level.checked_add(1) {
            Some(i) => {
                self.function_level = i;
                Ok(())
            }
            None => {
                let token = &self.tokens[self.current.checked_sub(1).unwrap_or(0)];
                self.error(
                    token.line,
                    token.token_type.pretty_print(),
                    "Only 256 levels of function nesting are supported.",
                );
                Err(ParserError {})
            }
        }
    }

    fn decrement_function_level(&mut self) {
        match self.function_level.checked_sub(1) {
            Some(i) => {
                self.function_level = i;
            }
            None => {
                panic!("Function level falling below zero, this should be impossible.");
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
        if self.matches_token(&[TokenType::For]) {
            self.looping_statement(Self::for_statement)
        } else if self.matches_token(&[TokenType::If]) {
            self.if_statement()
        } else if self.matches_token(&[TokenType::Print]) {
            self.print_statement()
        } else if self.matches_token(&[TokenType::Return]) {
            self.return_statement()
        } else if self.matches_token(&[TokenType::While]) {
            self.looping_statement(Self::while_statement)
        } else if self.matches_token(&[TokenType::Break]) {
            self.break_statement()
        } else if self.matches_token(&[TokenType::LeftBrace]) {
            self.block()
        } else {
            self.expression_statement()
        }
    }

    fn looping_statement<F>(&mut self, mut f: F) -> SResult
    where
        F: FnMut(&mut Self) -> SResult,
    {
        self.increment_loop_level()?;
        let out = f(self);
        self.decrement_loop_level();
        out
    }

    fn increment_loop_level(&mut self) -> Result<(), ParserError> {
        match self.loop_level.checked_add(1) {
            Some(i) => {
                self.loop_level = i;
                Ok(())
            }
            None => {
                let token = &self.tokens[self.current.checked_sub(1).unwrap_or(0)];
                self.error(
                    token.line,
                    token.token_type.pretty_print(),
                    "Only 256 levels of loop nesting are supported.",
                );
                Err(ParserError {})
            }
        }
    }

    fn decrement_loop_level(&mut self) {
        match self.loop_level.checked_sub(1) {
            Some(i) => {
                self.loop_level = i;
            }
            None => {
                panic!("Loop level falling below zero, this should be impossible.");
            }
        }
    }

    fn for_statement(&mut self) -> SResult {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'")?;

        let initializer = if self.matches_token(&[TokenType::Semicolon]) {
            None
        } else if self.matches_token(&[TokenType::Var]) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !self.check(&TokenType::Semicolon) {
            self.expression()?
        } else {
            // If no condition, then it is just `true`
            LiteralExpr::lift(ParsedLiteral::True)
        };
        self.consume(TokenType::Semicolon, "Expect ';' after for loop condition")?;

        let increment = if !self.check(&TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RightParen, "Expect ')' after for loop clauses")?;

        let body = if let Some(increment_expr) = increment {
            Block::lift(vec![self.statement()?, StmtExpression::lift(increment_expr)])
        } else {
            self.statement()?
        };

        let body = While::lift(condition, body);

        let body = if let Some(initializer_stmt) = initializer {
            Block::lift(vec![initializer_stmt, body])
        } else {
            body
        };

        Ok(body)
    }

    fn if_statement(&mut self) -> SResult {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' 'if' condition")?;

        let then_branch = self.statement()?;

        let else_branch = if self.matches_token(&[TokenType::Else]) {
            Some(self.statement()?)
        } else {
            None
        };

        Ok(If::lift(condition, then_branch, else_branch))
    }

    fn print_statement(&mut self) -> SResult {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Print::lift(expression))
    }

    fn return_statement(&mut self) -> SResult {
        let return_token = self.previous().expect("Empty previous after advancing?");

        if self.function_level > 0 {
            let error_tracking_token = ErrorTrackingToken::new("return".to_string(), return_token.line);
            let value = if !self.check(&TokenType::Semicolon) {
                Some(self.expression()?)
            } else {
                None
            };
            self.consume(TokenType::Semicolon, "Expect ';' after return.")?;
            Ok(Return::lift(error_tracking_token, value))
        } else {
            self.error(
                return_token.line,
                return_token.token_display(),
                "return statement must occur inside function body",
            );
            Err(ParserError {})
        }
    }

    fn while_statement(&mut self) -> SResult {
        self.consume(TokenType::LeftParen, "Expect '(' before if's condition.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after if's condition.")?;

        let body = self.statement()?;

        Ok(While::lift(condition, body))
    }

    fn break_statement(&mut self) -> SResult {
        if self.loop_level > 0 {
            self.consume(TokenType::Semicolon, "Expect ';' after break")?;
            Ok(Break::lift())
        } else {
            let token = &self.tokens[self.current.checked_sub(1).unwrap_or(0)];
            self.error(token.line, token.token_type.pretty_print(), "Expect 'break' inside loop.");
            Err(ParserError {})
        }
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

    // Expressions

    fn expression(&mut self) -> EResult {
        self.assignment()
    }

    fn assignment(&mut self) -> EResult {
        let expression = self.logical_or()?;

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
                    self.error(line, token_str, "Invalid assignment target.");
                }
            }
        }

        Ok(expression)
    }

    fn logical_or(&mut self) -> EResult {
        let mut expr = self.logical_and()?;

        while self.matches_token(&[TokenType::Or]) {
            let line = self.previous().expect("Empty previous after advance").line;
            let right = self.logical_and()?;
            expr = Logical::lift(expr, LogicalToken::new(LogicalOp::Or, line), right);
        }

        Ok(expr)
    }

    fn logical_and(&mut self) -> EResult {
        let mut expr = self.comma()?;

        while self.matches_token(&[TokenType::And]) {
            let line = self.previous().expect("Empty previous after advance").line;
            let right = self.comma()?;
            expr = Logical::lift(expr, LogicalToken::new(LogicalOp::And, line), right);
        }

        Ok(expr)
    }

    fn comma(&mut self) -> EResult {
        // self.left_associative_binary_op(&[TokenType::Comma], Self::equality)
        self.equality()
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
            self.call()
        }
    }

    fn call(&mut self) -> EResult {
        let mut expr = self.primary()?;

        // This is kind of an obtuse way of writing it, but the
        // Crafting Interpreters author says it's in preparation
        // for handling object properties later on.
        loop {
            if self.matches_token(&[TokenType::LeftParen]) {
                let open_paren_line = self.previous().expect("No previous after matching").line;
                expr = self.finish_call(expr, open_paren_line)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, expr: Expr, open_paren_line: usize) -> EResult {
        let mut arguments = vec![];

        if !self.check(&TokenType::RightParen) {
            // TODO: Can we support trailing commas? Those are really nice.
            loop {
                if arguments.len() > 255 {
                    let token = self.peek();

                    self.error(
                        token.line,
                        token.token_type.pretty_print(),
                        "Can't have more than 255 arguments.",
                    );
                }
                arguments.push(self.expression()?);
                if !self.matches_token(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        let close_paren_line = self
            .consume(TokenType::RightParen, "Expected ')' after arguments")?
            .expect("No token after consuming it?")
            .line;
        Ok(Call::lift(
            expr,
            ErrorTrackingToken::new("(".to_string(), open_paren_line),
            arguments,
            ErrorTrackingToken::new(")".to_string(), close_paren_line),
        ))
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
            self.error(token.line, token.token_type.pretty_print(), "Expected expression.");
            Err(ParserError {})
        }
    }
}

pub(crate) fn parse<W: Write>(tokens: Vec<Token>, mut error_writer: W) -> Result<Vec<Stmt>, ParserError> {
    let mut parser = Parser::new(&tokens, &mut error_writer);
    parser.parse()
}

pub(crate) fn parse_expression<W: Write>(
    tokens: Vec<Token>,
    mut error_writer: W,
) -> Result<Expr, ParserError> {
    let mut parser = Parser::new(&tokens, &mut error_writer);
    let out = parser.expression();
    if let Ok(expr) = out
        && parser.is_at_end()
    {
        Ok(expr)
    } else {
        Err(ParserError {})
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ReplParseOutput {
    Statements(Vec<Stmt>),
    Expr(Expr),
}

pub(crate) fn parse_for_repl<W: Write>(
    tokens: Vec<Token>,
    mut error_writer: W,
) -> Result<ReplParseOutput, ParserError> {
    let mut statement_parsing_errors = vec![];
    // let mut parser = Parser::new(&tokens, &mut statement_parsing_errors);
    match Parser::new(&tokens, &mut statement_parsing_errors).parse() {
        Ok(statements) => Ok(ReplParseOutput::Statements(statements)),
        Err(statement_error) => match parse_expression(tokens, std::io::sink()) {
            Ok(expr) => Ok(ReplParseOutput::Expr(expr)),
            Err(_) => {
                write!(
                    error_writer,
                    "{}",
                    String::from_utf8(statement_parsing_errors)
                        .expect("Error re-reading error message as utf8")
                )
                .expect("Error writing error.");
                Err(statement_error)
            }
        },
    }
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
            parse(scan_tokens(&"123;").expect("Error scanning"), std::io::stderr()).expect("Error parsing"),
            single_expr(literal_num(123.0))
        );

        assert_eq!(
            parse(scan_tokens(&r#""123";"#).expect("Error scanning"), std::io::stderr())
                .expect("Error parsing"),
            single_expr(literal_str("123"))
        );
    }

    // #[test]
    // fn test_comma() {
    //     assert_eq!(
    //         parse(scan_tokens(&"1, 2;").expect("Error scanning."), std::io::stderr())
    //             .expect("Error parsing."),
    //         single_expr(Binary::lift(literal_num(1.0), binary_token(BinaryOp::Comma), literal_num(2.0)))
    //     )
    // }

    #[test]
    fn test_parse_complex() {
        let expected = single_expr(Binary::lift(
            Unary::lift(unary_token(UnaryOp::Minus), literal_num(123.0)),
            binary_token(BinaryOp::Star),
            Grouping::lift(literal_num(45.67)),
        ));

        assert_eq!(
            parse(scan_tokens(&"-123 * (45.67);").expect("Error scanning"), std::io::stderr())
                .expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(
                scan_tokens(&"-123.0 /* comment */ * (45.67);").expect("Error scanning"),
                std::io::stderr()
            )
            .expect("Error parsing"),
            expected
        );

        assert_eq!(
            parse(
                scan_tokens(&"-123 /* comment */ * (45.67);  //").expect("Error scanning"),
                std::io::stderr()
            )
            .expect("Error parsing"),
            expected
        );

        let expected = single_expr(Binary::lift(
            Unary::lift(unary_token(UnaryOp::Minus), literal_num(123.0)),
            binary_token(BinaryOp::EqualEqual),
            Grouping::lift(literal_num(45.67)),
        ));

        assert_eq!(
            parse(scan_tokens(&"-123 == (45.67);").expect("Error scanning"), std::io::stderr())
                .expect("Error parsing"),
            expected
        );
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse(scan_tokens(&"+123;").expect("Error scanning"), std::io::sink()).is_err());
        assert!(parse(scan_tokens(&"class;").expect("Error scanning"), std::io::sink()).is_err());
        assert!(parse(scan_tokens(&"break;").expect("Error scanning"), std::io::sink()).is_err());
    }

    #[test]
    fn test_statement() {
        let parsed = parse(scan_tokens(&"var x = 5;").expect("Error scanning"), std::io::stderr())
            .expect("Error parsing");
        assert_eq!(
            vec![Var::lift(
                IdentifierToken::new("x".into(), 1),
                Some(LiteralExpr::lift(ParsedLiteral::Number(5.0)))
            )],
            parsed
        );

        let parsed =
            parse(scan_tokens(&"var x;").expect("Error scanning"), std::io::stderr()).expect("Error parsing");
        assert_eq!(vec![Var::lift(IdentifierToken::new("x".into(), 1), None)], parsed);
    }

    #[test]
    fn test_statements() {
        let parsed = parse(scan_tokens(&"var x = 5; print x;").expect("Error scanning"), std::io::stderr())
            .expect("Error parsing");
        assert_eq!(
            vec![
                Var::lift(
                    IdentifierToken::new("x".into(), 1),
                    Some(LiteralExpr::lift(ParsedLiteral::Number(5.0)))
                ),
                Print::lift(Variable::lift(IdentifierToken::new("x".into(), 1)))
            ],
            parsed
        );
    }

    #[test]
    fn test_while_loop_parses_to_while_loop() {
        let parsed = parse(
            scan_tokens(&"var x = 0; while (x < 5) {print x; x = x + 1;}").expect("Error scanning"),
            std::io::stderr(),
        )
        .expect("Error parsing");
        assert_eq!(
            vec![
                Var::lift(
                    IdentifierToken::new("x".into(), 1),
                    Some(LiteralExpr::lift(ParsedLiteral::Number(0.0)))
                ),
                While::lift(
                    Binary::lift(
                        Variable::lift(IdentifierToken::new("x".into(), 1)),
                        BinaryToken::new(BinaryOp::Less, 1),
                        LiteralExpr::lift(ParsedLiteral::Number(5.0))
                    ),
                    Block::lift(vec![
                        Print::lift(Variable::lift(IdentifierToken::new("x".into(), 1))),
                        StmtExpression::lift(Assign::lift(
                            IdentifierToken::new("x".into(), 1),
                            Binary::lift(
                                Variable::lift(IdentifierToken::new("x".into(), 1)),
                                BinaryToken::new(BinaryOp::Plus, 1),
                                LiteralExpr::lift(ParsedLiteral::Number(1.0)),
                            ),
                        )),
                    ]),
                )
            ],
            parsed
        );
    }

    #[test]
    fn test_for_loop_parses_to_while_loop() {
        let parsed = parse(
            scan_tokens(&"for (var x = 0; x < 5; x = x + 1) {print x;}").expect("Error scanning"),
            std::io::stderr(),
        )
        .expect("Error parsing");
        assert_eq!(
            vec![Block::lift(vec![
                Var::lift(
                    IdentifierToken::new("x".into(), 1),
                    Some(LiteralExpr::lift(ParsedLiteral::Number(0.0)))
                ),
                While::lift(
                    Binary::lift(
                        Variable::lift(IdentifierToken::new("x".into(), 1)),
                        BinaryToken::new(BinaryOp::Less, 1),
                        LiteralExpr::lift(ParsedLiteral::Number(5.0))
                    ),
                    Block::lift(vec![
                        Block::lift(vec![Print::lift(Variable::lift(IdentifierToken::new("x".into(), 1)))]),
                        StmtExpression::lift(Assign::lift(
                            IdentifierToken::new("x".into(), 1),
                            Binary::lift(
                                Variable::lift(IdentifierToken::new("x".into(), 1)),
                                BinaryToken::new(BinaryOp::Plus, 1),
                                LiteralExpr::lift(ParsedLiteral::Number(1.0)),
                            ),
                        )),
                    ]),
                )
            ])],
            parsed
        );
    }

    #[test]
    fn test_break() {
        let parsed = parse(scan_tokens(&"while (true) break;").expect("Error scanning"), std::io::stderr())
            .expect("Error parsing");

        assert_eq!(vec![While::lift(LiteralExpr::lift(ParsedLiteral::True), Break::lift())], parsed)
    }

    #[test]
    fn test_parse_for_repl() {
        let parsed = parse_for_repl(scan_tokens(&"var x = 5;").expect("Error scanning"), std::io::stderr())
            .expect("Error parsing");
        if let ReplParseOutput::Expr(_) = parsed {
            assert!(false, "Input was valid statement, but got parsed as expression.")
        }

        let parsed = parse_for_repl(scan_tokens(&"1").expect("Error scanning"), std::io::stderr())
            .expect("Error parsing");
        assert_eq!(parsed, ReplParseOutput::Expr(literal_num(1.0)));
    }
}

use crate::scanner::token::{Token, TokenType};
// use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

#[derive(Debug)]
struct ScanError {
    line: usize,
    message: String,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Scan Error line: {}] {}", self.line, self.message)
    }
}

static KEYWORD_MAP: LazyLock<HashMap<&'static str, TokenType>> = LazyLock::new(|| {
    HashMap::from([
        ("and", TokenType::And),
        ("class", TokenType::Class),
        ("else", TokenType::Else),
        ("false", TokenType::False),
        ("for", TokenType::For),
        ("fun", TokenType::Fun),
        ("if", TokenType::If),
        ("nil", TokenType::Nil),
        ("or", TokenType::Or),
        ("print", TokenType::Print),
        ("return", TokenType::Return),
        ("super", TokenType::Super),
        ("this", TokenType::This),
        ("true", TokenType::True),
        ("var", TokenType::Var),
        ("while", TokenType::While),
        ("break", TokenType::Break),
    ])
});

struct StateMachineError {
    message: String,
}

impl StateMachineError {
    fn to_scan_error(self, line: usize) -> ScanError {
        ScanError {
            line,
            message: self.message,
        }
    }
}

#[derive(Clone, Copy)]
struct LexemeStart(usize);
struct BlockCommentNesting(usize);

enum StateMachine {
    Root,
    Bang(LexemeStart),
    Equal(LexemeStart),
    Less(LexemeStart),
    Greater(LexemeStart),
    Slash(LexemeStart),
    InsideSingleLineComment,
    InsideBlockComment(BlockCommentNesting),
    InsideBlockCommentSawStar(BlockCommentNesting),
    InsideBlockCommentSawSlash(BlockCommentNesting),
    InsideString(LexemeStart),
    NumberBeforeDecimal(LexemeStart),
    NumberWithDecimal(LexemeStart),
    InsideIdentifier(LexemeStart),
}

type StateMachineResult<'a> = Result<([Option<Token<'a>>; 2], StateMachine), StateMachineError>;

struct ProcessInput<'a> {
    char: char,
    char_start: usize,
    char_end: usize,
    line: usize,
    source: &'a str,
}

impl StateMachine {
    fn new() -> Self {
        Self::Root
    }

    fn process<'a>(self: &Self, process_input: ProcessInput<'a>) -> StateMachineResult<'a> {
        use StateMachine::*;

        match self {
            Root => Self::process_top_level(process_input)
                .map(|(t, next_state)| ([t, None], next_state)),
            Bang(lexeme_start) => Self::process_look_for_equal(
                process_input,
                *lexeme_start,
                TokenType::BangEqual,
                TokenType::Bang,
            ),
            Equal(lexeme_start) => Self::process_look_for_equal(
                process_input,
                *lexeme_start,
                TokenType::EqualEqual,
                TokenType::Equal,
            ),
            Less(lexeme_start) => Self::process_look_for_equal(
                process_input,
                *lexeme_start,
                TokenType::LessEqual,
                TokenType::Less,
            ),
            Greater(lexeme_start) => Self::process_look_for_equal(
                process_input,
                *lexeme_start,
                TokenType::GreaterEqual,
                TokenType::Greater,
            ),
            Slash(lexeme_start) => Self::process_slash(process_input, *lexeme_start),
            InsideSingleLineComment => {
                Ok(([None, None], Self::process_line_comment(process_input.char)))
            }
            InsideBlockComment(BlockCommentNesting(n)) => Ok((
                [None, None],
                Self::process_block_comment(process_input.char, *n, false, false),
            )),
            InsideBlockCommentSawStar(BlockCommentNesting(n)) => Ok((
                [None, None],
                Self::process_block_comment(process_input.char, *n, true, false),
            )),
            InsideBlockCommentSawSlash(BlockCommentNesting(n)) => Ok((
                [None, None],
                Self::process_block_comment(process_input.char, *n, false, true),
            )),
            InsideString(lexeme_start) => Self::process_string(process_input, *lexeme_start),
            NumberBeforeDecimal(lexeme_start) => {
                Self::process_number(process_input, *lexeme_start, false)
            }
            NumberWithDecimal(lexeme_start) => {
                Self::process_number(process_input, *lexeme_start, true)
            }
            Self::InsideIdentifier(lexeme_start) => {
                Self::process_identifier(process_input, *lexeme_start)
            }
        }
    }

    fn process_top_level<'a>(
        process_input: ProcessInput<'a>,
    ) -> Result<(Option<Token<'a>>, StateMachine), StateMachineError> {
        match process_input.char {
            // One character tokens
            '(' => Ok((
                Some(TokenType::LeftParen.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            ')' => Ok((
                Some(TokenType::RightParen.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '{' => Ok((
                Some(TokenType::LeftBrace.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '}' => Ok((
                Some(TokenType::RightBrace.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            ',' => Ok((
                Some(TokenType::Comma.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '.' => Ok((
                Some(TokenType::Dot.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '-' => Ok((
                Some(TokenType::Minus.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '+' => Ok((
                Some(TokenType::Plus.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            ';' => Ok((
                Some(TokenType::Semicolon.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            '*' => Ok((
                Some(TokenType::Star.to_token(
                    process_input.line,
                    &process_input.source[process_input.char_start..process_input.char_end],
                )),
                StateMachine::Root,
            )),
            // Tokens that may match the next equals
            '!' => Ok((
                None,
                StateMachine::Bang(LexemeStart(process_input.char_start)),
            )),
            '=' => Ok((
                None,
                StateMachine::Equal(LexemeStart(process_input.char_start)),
            )),
            '<' => Ok((
                None,
                StateMachine::Less(LexemeStart(process_input.char_start)),
            )),
            '>' => Ok((
                None,
                StateMachine::Greater(LexemeStart(process_input.char_start)),
            )),
            // May be a comment
            '/' => Ok((
                None,
                StateMachine::Slash(LexemeStart(process_input.char_start)),
            )),
            // Whitespace
            ' ' | '\r' | '\t' | '\n' => Ok((None, StateMachine::Root)),
            // String literals
            '"' => Ok((
                None,
                StateMachine::InsideString(LexemeStart(process_input.char_start)),
            )),
            // Number literals
            '0'..='9' => Ok((
                None,
                StateMachine::NumberBeforeDecimal(LexemeStart(process_input.char_start)),
            )),
            // Identifiers
            'a'..='z' | 'A'..='Z' | '_' => Ok((
                None,
                StateMachine::InsideIdentifier(LexemeStart(process_input.char_start)),
            )),
            // Invalid characters
            _ => Err(StateMachineError {
                message: format!("Invalid character encountered: {}", process_input.char),
            }),
        }
    }

    fn process_look_for_equal<'a>(
        process_input: ProcessInput<'a>,
        lexeme_start: LexemeStart,
        if_equal: TokenType<'a>,
        otherwise: TokenType<'a>,
    ) -> StateMachineResult<'a> {
        if process_input.char == '=' {
            Ok((
                [
                    Some(if_equal.to_token(
                        process_input.line,
                        &process_input.source[lexeme_start.0..process_input.char_end],
                    )),
                    None,
                ],
                StateMachine::Root,
            ))
        } else {
            let line = process_input.line;
            let slice = &process_input.source[lexeme_start.0..process_input.char_start];
            let (maybe_token, next_state) = Self::process_top_level(process_input)?;
            Ok((
                [Some(otherwise.to_token(line, slice)), maybe_token],
                next_state,
            ))
        }
    }

    fn process_slash<'a>(
        process_input: ProcessInput<'a>,
        lexeme_start: LexemeStart,
    ) -> StateMachineResult<'a> {
        match process_input.char {
            '/' => Ok(([None, None], StateMachine::InsideSingleLineComment)),
            '*' => Ok((
                [None, None],
                StateMachine::InsideBlockComment(BlockCommentNesting(0)),
            )),
            _ => {
                let slash_token = TokenType::Slash.to_token(
                    process_input.line,
                    &process_input.source[lexeme_start.0..process_input.char_start],
                );
                let (maybe_token, next_state) = Self::process_top_level(process_input)?;
                Ok(([Some(slash_token), maybe_token], next_state))
            }
        }
    }

    fn process_line_comment(char: char) -> StateMachine {
        if char == '\n' {
            StateMachine::Root
        } else {
            StateMachine::InsideSingleLineComment
        }
    }

    fn process_block_comment(
        char: char,
        nesting_level: usize,
        saw_star: bool,
        saw_slash: bool,
    ) -> StateMachine {
        if saw_star && char == '/' {
            if nesting_level > 0 {
                StateMachine::InsideBlockComment(BlockCommentNesting(nesting_level - 1))
            } else {
                StateMachine::Root
            }
        } else if saw_slash && char == '*' {
            StateMachine::InsideBlockComment(BlockCommentNesting(nesting_level + 1))
        } else if char == '*' {
            StateMachine::InsideBlockCommentSawStar(BlockCommentNesting(nesting_level))
        } else if char == '/' {
            StateMachine::InsideBlockCommentSawSlash(BlockCommentNesting(nesting_level))
        } else {
            StateMachine::InsideBlockComment(BlockCommentNesting(nesting_level))
        }
    }

    fn process_string<'a>(
        process_input: ProcessInput<'a>,
        lexeme_start: LexemeStart,
    ) -> StateMachineResult<'a> {
        if process_input.char == '"' {
            let string_slice = &process_input.source[lexeme_start.0 + 1..process_input.char_start];
            let lexeme_slice = &process_input.source[lexeme_start.0..process_input.char_end];
            Ok((
                [
                    Some(
                        TokenType::String(string_slice).to_token(process_input.line, lexeme_slice),
                    ),
                    None,
                ],
                StateMachine::Root,
            ))
        } else {
            Ok(([None, None], StateMachine::InsideString(lexeme_start)))
        }
    }

    fn process_number<'a>(
        process_input: ProcessInput<'a>,
        lexeme_start: LexemeStart,
        has_seen_decimal: bool,
    ) -> StateMachineResult<'a> {
        match (process_input.char, has_seen_decimal) {
            (('0'..='9'), true) => {
                Ok(([None, None], StateMachine::NumberWithDecimal(lexeme_start)))
            }
            (('0'..='9'), false) => Ok((
                [None, None],
                StateMachine::NumberBeforeDecimal(lexeme_start),
            )),
            ('.', false) => Ok(([None, None], StateMachine::NumberWithDecimal(lexeme_start))),
            _ => {
                // Handle the case of (1) observing a '.' when we've already seen one, and
                // (2) observing any other non-number character.
                let number_str = &process_input.source[lexeme_start.0..process_input.char_start];
                let number_token =
                    TokenType::Number(number_str.parse().map_err(|_| StateMachineError {
                        message: format!("Unable to parse number from {number_str}"),
                    })?)
                    .to_token(process_input.line, number_str);

                let (maybe_token, next_state) = Self::process_top_level(process_input)?;
                Ok(([Some(number_token), maybe_token], next_state))
            }
        }
    }

    fn process_identifier<'a>(
        process_input: ProcessInput<'a>,
        lexeme_start: LexemeStart,
    ) -> StateMachineResult<'a> {
        match process_input.char {
            'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                Ok(([None, None], StateMachine::InsideIdentifier(lexeme_start)))
            }
            _ => {
                let lexeme = &process_input.source[lexeme_start.0..process_input.char_start];
                let token_type = match KEYWORD_MAP.get(lexeme) {
                    Some(token_type) => *token_type,
                    None => TokenType::Identifier,
                };
                let token = token_type.to_token(process_input.line, lexeme);
                let (maybe_token, next_state) = Self::process_top_level(process_input)?;
                Ok(([Some(token), maybe_token], next_state))
            }
        }
    }

    fn terminate_scanning<'a>(
        self: &Self,
        source: &'a str,
        line: usize,
    ) -> (Option<Token<'a>>, Option<StateMachineError>) {
        match self {
            // It's "ok" to end scanning in these states. (Most will cause issues downstream.)
            StateMachine::Root
            | StateMachine::Bang(_)
            | StateMachine::Equal(_)
            | StateMachine::Less(_)
            | StateMachine::Greater(_)
            | StateMachine::Slash(_)
            | StateMachine::InsideSingleLineComment
            | StateMachine::InsideBlockComment(_)
            | StateMachine::InsideBlockCommentSawStar(_)
            | StateMachine::InsideBlockCommentSawSlash(_) => (None, None),
            // Ending scanning in these states requires some final clean up
            Self::NumberBeforeDecimal(lexeme_start) | Self::NumberWithDecimal(lexeme_start) => {
                let number_str = &source[lexeme_start.0..];
                if number_str.ends_with('.') {
                    (
                        None,
                        Some(StateMachineError {
                            message: format!("Numbers cannot end with trailing '.': {number_str}"),
                        }),
                    )
                } else {
                    let maybe_number = number_str.parse().map_err(|_| StateMachineError {
                        message: format!("Unable to parse number from {number_str}"),
                    });
                    match maybe_number {
                        Ok(number) => {
                            let token = TokenType::Number(number).to_token(line, number_str);
                            (Some(token), None)
                        }
                        Err(e) => (None, Some(e)),
                    }
                }
            }
            Self::InsideIdentifier(lexeme_start) => {
                let lexeme = &source[lexeme_start.0..];
                let token_type = match KEYWORD_MAP.get(lexeme) {
                    Some(token_type) => *token_type,
                    None => TokenType::Identifier,
                };
                let token = token_type.to_token(line, lexeme);

                (Some(token), None)
            }
            // It's a scanning error to end scanning in these states.
            StateMachine::InsideString(_) => (
                None,
                Some(StateMachineError {
                    message: "Unterminated string.".to_string(),
                }),
            ),
        }
    }
}

pub(crate) struct Scanner<'a> {
    source: &'a str,
    state: StateMachine,
    line: usize,
    chars: std::str::CharIndices<'a>,
    next_result: Option<(Token<'a>, Option<Token<'a>>)>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn new(source: &'a str) -> Scanner<'a> {
        Scanner {
            source,
            state: StateMachine::new(),
            line: 1, // 1-indexed line numbers
            chars: source.char_indices(),
            next_result: None,
        }
    }

    pub(crate) fn scan_token(&mut self) -> Result<Token<'a>, ()> {
        let maybe_next_result = self.next_result.clone();
        if let Some((result, maybe_next_result)) = maybe_next_result {
            self.next_result = if let Some(next_result) = maybe_next_result {
                Some((next_result, None))
            } else {
                None
            };
            Ok(result)
        } else {
            loop {
                match self.chars.next() {
                    Some((char_start, char)) => {
                        if char == '\n' {
                            self.line += 1;
                        }

                        let process_results = self.state.process(ProcessInput {
                            char,
                            char_start,
                            char_end: self.source.ceil_char_boundary(char_start + 1),
                            line: self.line,
                            source: self.source,
                        });
                        match process_results {
                            Ok((maybe_tokens, new_state)) => {
                                self.state = new_state;
                                match maybe_tokens {
                                    [None, None] => {}
                                    [None, Some(token)] => break Ok(token),
                                    [Some(token), None] => break Ok(token),
                                    [Some(token1), Some(token2)] => {
                                        self.next_result = Some((token2, None));
                                        break Ok(token1);
                                    }
                                }
                            }
                            Err(e) => {
                                self.state = StateMachine::new();
                                eprintln!("{}", e.to_scan_error(self.line));
                                break Err(());
                            }
                        }
                    }
                    None => {
                        // Finished iterating through source code, need to perform final clean up.

                        let (maybe_token, maybe_error) =
                            self.state.terminate_scanning(self.source, self.line);

                        let eof = TokenType::Eof.to_token(self.line, &self.source[0..0]);

                        match (maybe_token, maybe_error) {
                            (None, None) => {
                                break Ok(eof);
                            }
                            (None, Some(e)) => {
                                self.next_result = Some((eof, None));
                                eprintln!("{}", e.to_scan_error(self.line));
                                break Err(());
                            }
                            (Some(token), None) => {
                                self.next_result = Some((eof, None));
                                break Ok(token);
                            }
                            (Some(token), Some(e)) => {
                                self.next_result = Some((token, Some(eof)));
                                eprintln!("{}", e.to_scan_error(self.line));
                                break Err(());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EOF_LINE_1: Token<'static> = TokenType::Eof.to_token(1, "");

    fn scan_to_completion<'a>(source: &'a str) -> Result<Vec<Token<'a>>, ()> {
        let mut scanner = Scanner::new(source);
        let mut tokens = vec![];
        loop {
            let token = scanner.scan_token()?;
            let is_eof = token.token_type == TokenType::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    #[test]
    fn test_scans_one_token_at_a_time() {
        let source = "(\n)";
        let mut scanner = Scanner::new(source);

        assert_eq!(
            scanner.scan_token(),
            Ok(TokenType::LeftParen.to_token(1, "("))
        );
        assert_eq!(
            scanner.scan_token(),
            Ok(TokenType::RightParen.to_token(2, ")"))
        );
    }

    #[test]
    fn test_scan_error_does_not_block_rest() {
        let source = "1.";
        let mut scanner = Scanner::new(source);
        assert_eq!(scanner.scan_token(), Err(()));
        assert_eq!(scanner.scan_token(), Ok(EOF_LINE_1));

        // Lox only supports ascii identifiers.
        let source = "print ß;";
        let mut scanner = Scanner::new(source);
        assert_eq!(
            scanner.scan_token(),
            Ok(TokenType::Print.to_token(1, "print"))
        );
        assert_eq!(scanner.scan_token(), Err(()));
        assert_eq!(
            scanner.scan_token(),
            Ok(TokenType::Semicolon.to_token(1, ";"))
        );
        assert_eq!(scanner.scan_token(), Ok(EOF_LINE_1));
    }

    #[test]
    fn test_error() {
        scan_to_completion("%").expect_err("Failed to error while scanning invalid source");
        scan_to_completion("asdf%").expect_err("Failed to error while scanning invalid source");
        scan_to_completion("1234%").expect_err("Failed to error while scanning invalid source");
    }

    #[test]
    fn test_keywords() {
        for (key, val) in KEYWORD_MAP.iter() {
            let source = &format!("({key})");
            let tokens = scan_to_completion(source).unwrap();
            assert_eq!(
                tokens,
                vec![
                    TokenType::LeftParen.to_token(1, "("),
                    val.to_token(1, key),
                    TokenType::RightParen.to_token(1, ")"),
                    EOF_LINE_1
                ]
            );
        }
    }

    #[test]
    fn test_identifier() {
        let tokens = scan_to_completion("a ").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Identifier.to_token(1, "a"), EOF_LINE_1]
        );

        let tokens = scan_to_completion("a").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Identifier.to_token(1, "a"), EOF_LINE_1]
        );

        let tokens = scan_to_completion("a5").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Identifier.to_token(1, "a5"), EOF_LINE_1]
        );

        let tokens = scan_to_completion("_az_AZ_09_").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Identifier.to_token(1, "_az_AZ_09_"), EOF_LINE_1]
        );
    }

    #[test]
    fn test_number() {
        let tokens = scan_to_completion("123  \r\t   456 \t\t0.5 ").unwrap();
        assert_eq!(
            tokens,
            vec![
                TokenType::Number(123.0).to_token(1, "123"),
                TokenType::Number(456.0).to_token(1, "456"),
                TokenType::Number(0.5).to_token(1, "0.5"),
                EOF_LINE_1
            ]
        )
    }

    #[test]
    fn test_numbers_at_eof() {
        let tokens = scan_to_completion("9").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Number(9.0).to_token(1, "9"), EOF_LINE_1]
        );

        let tokens = scan_to_completion("9.0").unwrap();
        assert_eq!(
            tokens,
            vec![TokenType::Number(9.0).to_token(1, "9.0"), EOF_LINE_1]
        );

        assert!(scan_to_completion("9.").is_err());
    }

    #[test]
    fn test_string() {
        let s = "\"Hello, World!\"";
        let tokens = scan_to_completion(s).unwrap();

        assert_eq!(
            tokens,
            vec![
                TokenType::String("Hello, World!").to_token(1, s),
                EOF_LINE_1
            ]
        );
    }

    #[test]
    fn test_with_unicode_string() {
        // While lox does not support arbitrary unicode identifiers, I've decided
        // that my implementation _does_ allow them in strings. I'm not sure whether or
        // not the C implementation from the book does, to be honest.
        let s = "\"Hello, µWorld!\"";
        let tokens = scan_to_completion(s).unwrap();

        assert_eq!(
            tokens,
            vec![
                TokenType::String("Hello, µWorld!").to_token(1, s),
                EOF_LINE_1
            ]
        );
    }

    #[test]
    fn test_line_comment() {
        assert_eq!(scan_to_completion("// 1234").unwrap(), vec![EOF_LINE_1]);
    }

    #[test]
    fn test_line_comment_2() {
        assert_eq!(
            scan_to_completion("// 1234\n(").unwrap(),
            vec![
                TokenType::LeftParen.to_token(2, "("),
                TokenType::Eof.to_token(2, "")
            ]
        );
    }

    #[test]
    fn test_block_comments() {
        assert_eq!(scan_to_completion("/* 1234").unwrap(), vec![EOF_LINE_1]);
        assert_eq!(scan_to_completion("/* 1234 */").unwrap(), vec![EOF_LINE_1]);

        assert_eq!(
            scan_to_completion("(/* 1234 */)").unwrap(),
            vec![
                TokenType::LeftParen.to_token(1, "("),
                TokenType::RightParen.to_token(1, ")"),
                EOF_LINE_1
            ]
        );
    }

    #[test]
    fn test_block_comments_nesting() {
        assert_eq!(
            scan_to_completion("/* /* 1234 */ 5678").unwrap(),
            vec![EOF_LINE_1]
        );
        assert_eq!(
            scan_to_completion("(/* /* 1234 */ 5678 */)").unwrap(),
            vec![
                TokenType::LeftParen.to_token(1, "("),
                TokenType::RightParen.to_token(1, ")"),
                EOF_LINE_1
            ]
        );
    }

    #[test]
    fn test_scan_simple() {
        let tokens = scan_to_completion("(").unwrap();

        assert_eq!(
            tokens,
            vec![TokenType::LeftParen.to_token(1, "("), EOF_LINE_1]
        );
    }

    #[test]
    fn test_scan_tokens_larger_input() {
        let tokens = scan_to_completion(
            r#"// this is a comment
            (( )){} // grouping stuff
            !*+-/=<> <= == // operators
            /**/
            "#,
        );

        assert!(tokens.is_ok());

        let tokens = tokens.unwrap();

        assert_eq!(
            tokens,
            vec![
                Token {
                    token_type: TokenType::LeftParen,
                    line: 2,
                    lexeme: "(",
                },
                Token {
                    token_type: TokenType::LeftParen,
                    line: 2,
                    lexeme: "(",
                },
                Token {
                    token_type: TokenType::RightParen,
                    line: 2,
                    lexeme: ")",
                },
                Token {
                    token_type: TokenType::RightParen,
                    line: 2,
                    lexeme: ")",
                },
                Token {
                    token_type: TokenType::LeftBrace,
                    line: 2,
                    lexeme: "{",
                },
                Token {
                    token_type: TokenType::RightBrace,
                    line: 2,
                    lexeme: "}",
                },
                Token {
                    token_type: TokenType::Bang,
                    line: 3,
                    lexeme: "!",
                },
                Token {
                    token_type: TokenType::Star,
                    line: 3,
                    lexeme: "*",
                },
                Token {
                    token_type: TokenType::Plus,
                    line: 3,
                    lexeme: "+",
                },
                Token {
                    token_type: TokenType::Minus,
                    line: 3,
                    lexeme: "-",
                },
                Token {
                    token_type: TokenType::Slash,
                    line: 3,
                    lexeme: "/",
                },
                Token {
                    token_type: TokenType::Equal,
                    line: 3,
                    lexeme: "=",
                },
                Token {
                    token_type: TokenType::Less,
                    line: 3,
                    lexeme: "<",
                },
                Token {
                    token_type: TokenType::Greater,
                    line: 3,
                    lexeme: ">",
                },
                Token {
                    token_type: TokenType::LessEqual,
                    line: 3,
                    lexeme: "<=",
                },
                Token {
                    token_type: TokenType::EqualEqual,
                    line: 3,
                    lexeme: "==",
                },
                Token {
                    token_type: TokenType::Eof,
                    line: 5,
                    lexeme: "",
                },
            ]
        )
    }
}

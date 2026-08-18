use crate::scanner::token::{Token, TokenType};
use anyhow::{Result, anyhow};
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

enum StateMachine {
    Root,
    Bang,
    Equal,
    Less,
    Greater,
    Slash,
    InsideSingleLineComment,
    InsideBlockComment(usize),
    InsideBlockCommentSawStar(usize),
    InsideBlockCommentSawSlash(usize),
    InsideString(String),
    NumberStart(String),
    NumberWithDecimal(String),
    InsideIdentifier(String),
}

type StateMachineResult = Result<([Option<TokenType>; 2], StateMachine), StateMachineError>;

impl StateMachine {
    fn new() -> Self {
        Self::Root
    }

    fn process(self: Self, char: char) -> StateMachineResult {
        use StateMachine::*;

        match self {
            Root => Self::process_top_level(char).map(|(t, next_state)| ([t, None], next_state)),
            Bang => Self::process_look_for_equal(char, TokenType::BangEqual, TokenType::Bang),
            Equal => Self::process_look_for_equal(char, TokenType::EqualEqual, TokenType::Equal),
            Less => Self::process_look_for_equal(char, TokenType::LessEqual, TokenType::Less),
            Greater => Self::process_look_for_equal(char, TokenType::GreaterEqual, TokenType::Greater),
            Slash => Self::process_slash(char),
            InsideSingleLineComment => Ok(([None, None], Self::process_line_comment(char))),
            InsideBlockComment(n) => Ok(([None, None], Self::process_block_comment(char, n, false, false))),
            InsideBlockCommentSawStar(n) => {
                Ok(([None, None], Self::process_block_comment(char, n, true, false)))
            }
            InsideBlockCommentSawSlash(n) => {
                Ok(([None, None], Self::process_block_comment(char, n, false, true)))
            }
            InsideString(s) => Self::process_string(char, s),
            NumberStart(s) => Self::process_number(char, s, false),
            NumberWithDecimal(s) => Self::process_number(char, s, true),
            Self::InsideIdentifier(s) => Self::process_identifier(char, s),
        }
    }

    fn process_top_level(char: char) -> Result<(Option<TokenType>, StateMachine), StateMachineError> {
        match char {
            // One character tokens
            '(' => Ok((Some(TokenType::LeftParen), StateMachine::Root)),
            ')' => Ok((Some(TokenType::RightParen), StateMachine::Root)),
            '{' => Ok((Some(TokenType::LeftBrace), StateMachine::Root)),
            '}' => Ok((Some(TokenType::RightBrace), StateMachine::Root)),
            ',' => Ok((Some(TokenType::Comma), StateMachine::Root)),
            '.' => Ok((Some(TokenType::Dot), StateMachine::Root)),
            '-' => Ok((Some(TokenType::Minus), StateMachine::Root)),
            '+' => Ok((Some(TokenType::Plus), StateMachine::Root)),
            ';' => Ok((Some(TokenType::Semicolon), StateMachine::Root)),
            '*' => Ok((Some(TokenType::Star), StateMachine::Root)),
            // Tokens that may match the next equals
            '!' => Ok((None, StateMachine::Bang)),
            '=' => Ok((None, StateMachine::Equal)),
            '<' => Ok((None, StateMachine::Less)),
            '>' => Ok((None, StateMachine::Greater)),
            // May be a comment
            '/' => Ok((None, StateMachine::Slash)),
            // Whitespace
            ' ' | '\r' | '\t' | '\n' => Ok((None, StateMachine::Root)),
            // String literals
            '"' => Ok((None, StateMachine::InsideString(String::new()))),
            // Number literals
            '0'..='9' => Ok((None, StateMachine::NumberStart(char.to_string()))),
            // Identifiers
            'a'..='z' | 'A'..='Z' | '_' => Ok((None, StateMachine::InsideIdentifier(char.to_string()))),
            // Invalid characters
            _ => Err(StateMachineError {
                message: format!("Invalid character encountered: {}", char),
            }),
        }
    }

    fn process_look_for_equal(char: char, if_equal: TokenType, otherwise: TokenType) -> StateMachineResult {
        if char == '=' {
            Ok(([Some(if_equal), None], StateMachine::Root))
        } else {
            let (maybe_token, next_state) = Self::process_top_level(char)?;
            Ok(([Some(otherwise), maybe_token], next_state))
        }
    }

    fn process_slash(char: char) -> StateMachineResult {
        match char {
            '/' => Ok(([None, None], StateMachine::InsideSingleLineComment)),
            '*' => Ok(([None, None], StateMachine::InsideBlockComment(0))),
            _ => {
                let (maybe_token, next_state) = Self::process_top_level(char)?;
                Ok(([Some(TokenType::Slash), maybe_token], next_state))
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
                StateMachine::InsideBlockComment(nesting_level - 1)
            } else {
                StateMachine::Root
            }
        } else if saw_slash && char == '*' {
            StateMachine::InsideBlockComment(nesting_level + 1)
        } else if char == '*' {
            StateMachine::InsideBlockCommentSawStar(nesting_level)
        } else if char == '/' {
            StateMachine::InsideBlockCommentSawSlash(nesting_level)
        } else {
            StateMachine::InsideBlockComment(nesting_level)
        }
    }

    fn process_string(char: char, mut s: String) -> StateMachineResult {
        if char == '"' {
            s.shrink_to_fit();
            Ok(([Some(TokenType::String(s)), None], StateMachine::Root))
        } else {
            s.push(char);
            Ok(([None, None], StateMachine::InsideString(s)))
        }
    }

    fn process_number(char: char, mut s: String, has_seen_decimal: bool) -> StateMachineResult {
        match char {
            '0'..='9' => {
                s.push(char);
                Ok((
                    [None, None],
                    if has_seen_decimal {
                        StateMachine::NumberWithDecimal(s)
                    } else {
                        StateMachine::NumberStart(s)
                    },
                ))
            }
            '.' => {
                if has_seen_decimal {
                    // We could hard code it here since we know it's a dot, but... eh.
                    let (maybe_token, next_state) = Self::process_top_level(char)?;
                    Ok((
                        [
                            Some(TokenType::Number(s.parse().map_err(|_| StateMachineError {
                                message: format!("Unable to parse number from {s}"),
                            })?)),
                            maybe_token,
                        ],
                        next_state,
                    ))
                } else {
                    s.push(char);
                    Ok(([None, None], StateMachine::NumberWithDecimal(s)))
                }
            }
            _ => {
                let (maybe_token, next_state) = Self::process_top_level(char)?;
                Ok((
                    [
                        Some(TokenType::Number(s.parse().map_err(|_| StateMachineError {
                            message: format!("Unable to parse number from {s}"),
                        })?)),
                        maybe_token,
                    ],
                    next_state,
                ))
            }
        }
    }

    fn process_identifier(char: char, mut s: String) -> StateMachineResult {
        match char {
            'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                s.push(char);
                Ok(([None, None], StateMachine::InsideIdentifier(s)))
            }
            _ => {
                let token_type = match KEYWORD_MAP.get(s.as_str()) {
                    Some(token_type) => token_type.clone(),
                    None => TokenType::Identifier(s),
                };
                let (maybe_token, next_state) = Self::process_top_level(char)?;
                Ok(([Some(token_type), maybe_token], next_state))
            }
        }
    }

    fn terminate_scanning(self: Self) -> (Option<TokenType>, Option<StateMachineError>) {
        match self {
            // It's "ok" to end scanning in these states. (Most will cause issues downstream.)
            StateMachine::Root
            | StateMachine::Bang
            | StateMachine::Equal
            | StateMachine::Less
            | StateMachine::Greater
            | StateMachine::Slash
            | StateMachine::InsideSingleLineComment
            | StateMachine::InsideBlockComment(_)
            | StateMachine::InsideBlockCommentSawStar(_)
            | StateMachine::InsideBlockCommentSawSlash(_) => (None, None),
            // Ending scanning in these states requires some final clean up
            Self::NumberStart(s) => match Self::process_number(' ', s, false) {
                Ok(([first, second], _)) => {
                    debug_assert!(second.is_none());
                    (first, None)
                }
                Err(e) => (None, Some(e)),
            },
            Self::NumberWithDecimal(s) => match Self::process_number(' ', s, true) {
                Ok(([first, second], _)) => {
                    debug_assert!(second.is_none());
                    (first, None)
                }
                Err(e) => (None, Some(e)),
            },
            Self::InsideIdentifier(s) => match Self::process_identifier(' ', s) {
                Ok(([first, second], _)) => {
                    debug_assert!(second.is_none());
                    (first, None)
                }
                Err(e) => (None, Some(e)),
            },
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

pub(crate) fn scan_tokens(source: &str) -> Result<Vec<Token>> {
    let mut character_scanning_state = StateMachine::new();
    let mut tokens = vec![];
    let mut errors = vec![];
    let mut line = 1;

    for char in source.chars() {
        match character_scanning_state.process(char) {
            Ok((maybe_tokens, new_state)) => {
                character_scanning_state = new_state;
                for maybe_token in maybe_tokens {
                    if let Some(token_type) = maybe_token {
                        tokens.push(Token {
                            token_type,
                            line,
                        });
                    }
                }
            }
            Err(e) => {
                character_scanning_state = StateMachine::new();
                errors.push(e.to_scan_error(line));
            }
        }

        if char == '\n' {
            line += 1;
        }
    }

    let (maybe_final_token, maybe_error) = character_scanning_state.terminate_scanning();

    if let Some(token_type) = maybe_final_token {
        tokens.push(Token {
            token_type,
            line,
        });
    }

    if let Some(e) = maybe_error {
        errors.push(e.to_scan_error(line));
    }

    tokens.push(Token {
        token_type: TokenType::Eof,
        line,
    });

    if errors.len() > 0 {
        for error in errors {
            eprintln!("{}", error);
        }
        Err(anyhow!("Failed to scan code."))
    } else {
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Converts list of token types into list of tokens, assuming they all
    /// exist on line 1. Also appends an EOF token to the end.
    fn one_liner(token_types: Vec<TokenType>) -> Vec<Token> {
        one_liner_on_line(token_types, 1)
    }

    fn one_liner_on_line(token_types: Vec<TokenType>, line: usize) -> Vec<Token> {
        token_types
            .into_iter()
            .chain(vec![TokenType::Eof])
            .map(|token_type| Token {
                token_type,
                line,
            })
            .collect()
    }

    #[test]
    fn test_error() {
        scan_tokens("%").expect_err("Failed to error while scanning invalid source");
        scan_tokens("asdf%").expect_err("Failed to error while scanning invalid source");
        scan_tokens("1234%").expect_err("Failed to error while scanning invalid source");
    }

    #[test]
    fn test_keywords() {
        for (key, val) in KEYWORD_MAP.iter() {
            let tokens = scan_tokens(&format!("({key})")).unwrap();
            assert_eq!(tokens, one_liner(vec![TokenType::LeftParen, val.clone(), TokenType::RightParen]));
        }
    }

    #[test]
    fn test_identifier() {
        let tokens = scan_tokens("a ").unwrap();
        assert_eq!(tokens, one_liner(vec![TokenType::Identifier("a".to_string())]));

        let tokens = scan_tokens("a").unwrap();
        assert_eq!(tokens, one_liner(vec![TokenType::Identifier("a".to_string())]));

        let tokens = scan_tokens("a5").unwrap();
        assert_eq!(tokens, one_liner(vec![TokenType::Identifier("a5".to_string())]));

        let tokens = scan_tokens("_az_AZ_09_").unwrap();
        assert_eq!(tokens, one_liner(vec![TokenType::Identifier("_az_AZ_09_".to_string())]));
    }

    #[test]
    fn test_number() {
        let tokens = scan_tokens("123 456 0.5 ").unwrap();
        assert_eq!(
            tokens,
            one_liner(vec![TokenType::Number(123.0), TokenType::Number(456.0), TokenType::Number(0.5)])
        )
    }

    #[test]
    fn test_number_at_eof() {
        let tokens = scan_tokens("9").unwrap();
        assert_eq!(tokens, one_liner(vec![TokenType::Number(9.0)]))
    }

    #[test]
    fn test_string() {
        let tokens = scan_tokens("\"Hello, World!\"()").unwrap();

        assert_eq!(
            tokens,
            one_liner(vec![
                TokenType::String("Hello, World!".to_string()),
                TokenType::LeftParen,
                TokenType::RightParen
            ])
        );
    }

    #[test]
    fn test_line_comment() {
        assert_eq!(scan_tokens("// 1234").unwrap(), one_liner(vec![]));
    }

    #[test]
    fn test_line_comment_2() {
        assert_eq!(scan_tokens("// 1234\n(").unwrap(), one_liner_on_line(vec![TokenType::LeftParen], 2));
    }

    #[test]
    fn test_block_comments() {
        assert_eq!(scan_tokens("/* 1234").unwrap(), one_liner(vec![]));
        assert_eq!(scan_tokens("/* 1234 */").unwrap(), one_liner(vec![]));

        assert_eq!(
            scan_tokens("(/* 1234 */)").unwrap(),
            one_liner(vec![TokenType::LeftParen, TokenType::RightParen])
        );
    }

    #[test]
    fn test_block_comments_nesting() {
        assert_eq!(scan_tokens("/* /* 1234 */ 5678").unwrap(), one_liner(vec![]));
        assert_eq!(
            scan_tokens("(/* /* 1234 */ 5678 */)").unwrap(),
            one_liner(vec![TokenType::LeftParen, TokenType::RightParen])
        );
    }

    #[test]
    fn test_scan_simple() {
        let tokens = scan_tokens("(").unwrap();

        assert_eq!(tokens, one_liner(vec![TokenType::LeftParen]));
    }

    #[test]
    fn test_scan_tokens_larger_input() {
        let tokens = scan_tokens(
            r#"// this is a comment
            (( )){} // grouping stuff
            !*+-/=<> <= == // operators
            "#,
        );

        assert!(tokens.is_ok());

        let tokens = tokens.unwrap();

        assert_eq!(
            tokens,
            vec![
                Token {
                    token_type: TokenType::LeftParen,
                    line: 2
                },
                Token {
                    token_type: TokenType::LeftParen,
                    line: 2
                },
                Token {
                    token_type: TokenType::RightParen,
                    line: 2
                },
                Token {
                    token_type: TokenType::RightParen,
                    line: 2
                },
                Token {
                    token_type: TokenType::LeftBrace,
                    line: 2
                },
                Token {
                    token_type: TokenType::RightBrace,
                    line: 2
                },
                Token {
                    token_type: TokenType::Bang,
                    line: 3
                },
                Token {
                    token_type: TokenType::Star,
                    line: 3
                },
                Token {
                    token_type: TokenType::Plus,
                    line: 3
                },
                Token {
                    token_type: TokenType::Minus,
                    line: 3
                },
                Token {
                    token_type: TokenType::Slash,
                    line: 3
                },
                Token {
                    token_type: TokenType::Equal,
                    line: 3
                },
                Token {
                    token_type: TokenType::Less,
                    line: 3
                },
                Token {
                    token_type: TokenType::Greater,
                    line: 3
                },
                Token {
                    token_type: TokenType::LessEqual,
                    line: 3
                },
                Token {
                    token_type: TokenType::EqualEqual,
                    line: 3
                },
                Token {
                    token_type: TokenType::Eof,
                    line: 4
                },
            ]
        )
    }
}

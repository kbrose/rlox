#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    //OneOrTwoCharacterTokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    //Literals.
    Identifier,
    String,
    Number,

    //Keywords.
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Break,

    // End of file
    Eof,
}

impl TokenType {
    pub(super) const fn to_token<'a>(self: Self, line: u32, lexeme: &'a str) -> Token<'a> {
        Token {
            token_type: self,
            line,
            lexeme,
        }
    }

    pub(crate) fn to_debug_num(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct Token<'a> {
    pub(crate) token_type: TokenType,
    pub(crate) line: u32,
    pub(crate) lexeme: &'a str,
}

impl<'a> Token<'a> {
    fn line(&self) -> u32 {
        self.line
    }
}

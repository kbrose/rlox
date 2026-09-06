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
    token_type: TokenType,
    line: u32,
    lexeme: &'a str,
}

impl<'a> Token<'a> {
    #[inline]
    pub(crate) fn token_type(&self) -> TokenType {
        self.token_type
    }

    #[inline]
    pub(crate) fn line(&self) -> u32 {
        self.line
    }

    #[inline]
    pub(crate) fn lexeme(&self) -> &'a str {
        self.lexeme
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum TokenType<'a> {
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
    String(&'a str),
    Number(f64),

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

impl<'a> TokenType<'a> {
    pub(super) const fn to_token(self: Self, line: usize, lexeme: &'a str) -> Token<'a> {
        Token {
            token_type: self,
            line,
            lexeme,
        }
    }

    pub(crate) fn to_debug_num(&self) -> u8 {
        match self {
            Self::LeftParen => 0,
            Self::RightParen => 1,
            Self::LeftBrace => 2,
            Self::RightBrace => 3,
            Self::Comma => 4,
            Self::Dot => 5,
            Self::Minus => 6,
            Self::Plus => 7,
            Self::Semicolon => 8,
            Self::Slash => 9,
            Self::Star => 10,
            Self::Bang => 11,
            Self::BangEqual => 12,
            Self::Equal => 13,
            Self::EqualEqual => 14,
            Self::Greater => 15,
            Self::GreaterEqual => 16,
            Self::Less => 17,
            Self::LessEqual => 18,
            Self::Identifier => 19,
            Self::String(_) => 20,
            Self::Number(_) => 21,
            Self::And => 22,
            Self::Class => 23,
            Self::Else => 24,
            Self::False => 25,
            Self::Fun => 26,
            Self::For => 27,
            Self::If => 28,
            Self::Nil => 29,
            Self::Or => 30,
            Self::Print => 31,
            Self::Return => 32,
            Self::Super => 33,
            Self::This => 34,
            Self::True => 35,
            Self::Var => 36,
            Self::While => 37,
            Self::Break => 38,
            Self::Eof => 39,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Token<'a> {
    pub(crate) token_type: TokenType<'a>,
    pub(crate) line: usize,
    pub(crate) lexeme: &'a str,
}

impl<'a> Token<'a> {
    fn line(&self) -> usize {
        self.line
    }
}

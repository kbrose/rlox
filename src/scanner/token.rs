#[derive(Debug, PartialEq, Clone)]
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
    Identifier(String),
    String(String),
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

    // End of file
    Eof,
}

impl TokenType {
    pub(crate) fn pretty_print(self: &Self) -> String {
        match &self {
            TokenType::LeftParen => "(".to_string(),
            TokenType::RightParen => ")".to_string(),
            TokenType::LeftBrace => "{".to_string(),
            TokenType::RightBrace => "}".to_string(),
            TokenType::Comma => ",".to_string(),
            TokenType::Dot => ".".to_string(),
            TokenType::Minus => "-".to_string(),
            TokenType::Plus => "+".to_string(),
            TokenType::Semicolon => ";".to_string(),
            TokenType::Slash => "/".to_string(),
            TokenType::Star => "*".to_string(),
            TokenType::Bang => "!".to_string(),
            TokenType::BangEqual => "!=".to_string(),
            TokenType::Equal => "=".to_string(),
            TokenType::EqualEqual => "==".to_string(),
            TokenType::Greater => ">".to_string(),
            TokenType::GreaterEqual => ">=".to_string(),
            TokenType::Less => "<".to_string(),
            TokenType::LessEqual => "<=".to_string(),
            TokenType::Identifier(s) => s.clone(),
            TokenType::String(s) => format!("\"{}\"", s),
            TokenType::Number(x) => format!("{x}"),
            TokenType::And => "and".to_string(),
            TokenType::Class => "class ".to_string(),
            TokenType::Else => "else".to_string(),
            TokenType::False => "false".to_string(),
            TokenType::Fun => "fun".to_string(),
            TokenType::For => "for".to_string(),
            TokenType::If => "if".to_string(),
            TokenType::Nil => "nil".to_string(),
            TokenType::Or => "or".to_string(),
            TokenType::Print => "print".to_string(),
            TokenType::Return => "return".to_string(),
            TokenType::Super => "super".to_string(),
            TokenType::This => "this".to_string(),
            TokenType::True => "true".to_string(),
            TokenType::Var => "var".to_string(),
            TokenType::While => "while".to_string(),
            TokenType::Eof => "<EOF>".to_string(),
        }
    }
}

pub(crate) trait TokenLike {
    fn line(&self) -> usize;
    fn token_display(&self) -> String;
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Token {
    pub(crate) token_type: TokenType,
    pub(crate) line: usize,
}

impl TokenLike for Token {
    fn line(&self) -> usize {
        self.line
    }

    fn token_display(&self) -> String {
        self.token_type.pretty_print()
    }
}

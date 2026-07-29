// The scanner gives us plain Tokens. After parsing, though, we
// can get a little more specific in a lot of cases. For expressions
// and statements that need to track their tokens, it will be a lot
// more type-safe if those expressions/statements can only hold tokens
// of the correct type.

use crate::scanner::TokenLike;

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct IdentifierToken {
    identifier: String,
    line: usize,
}
impl IdentifierToken {
    pub(crate) fn new(identifier: String, line: usize) -> Self {
        Self {
            identifier,
            line,
        }
    }
    pub(crate) fn pretty_print(&self) -> String {
        self.identifier.clone()
    }
    pub(crate) fn identifier(&self) -> &str {
        &self.identifier
    }
}
impl TokenLike for IdentifierToken {
    fn line(&self) -> usize {
        self.line
    }
    fn token_display(&self) -> String {
        self.pretty_print()
    }
}

// Note that there will be redundancy between UnaryOp and BinaryOp:
// the `-` character can resolve to either one depending on the syntax.
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum UnaryOp {
    Minus,
    Bang,
}
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct UnaryToken {
    op: UnaryOp,
    line: usize,
}
impl UnaryToken {
    pub(crate) fn new(op: UnaryOp, line: usize) -> Self {
        Self {
            op,
            line,
        }
    }
    pub(crate) fn op(&self) -> &UnaryOp {
        &self.op
    }
    pub(crate) fn pretty_print(&self) -> String {
        match self.op {
            UnaryOp::Minus => "-".to_string(),
            UnaryOp::Bang => "!".to_string(),
        }
    }
}
impl TokenLike for UnaryToken {
    fn line(&self) -> usize {
        self.line
    }
    fn token_display(&self) -> String {
        self.pretty_print()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum BinaryOp {
    Plus,
    Minus,
    Slash,
    Star,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    BangEqual,
    EqualEqual,
    Comma,
}
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct BinaryToken {
    op: BinaryOp,
    line: usize,
}
impl BinaryToken {
    pub(crate) fn new(op: BinaryOp, line: usize) -> Self {
        Self {
            op,
            line,
        }
    }
    pub(crate) fn op(&self) -> &BinaryOp {
        &self.op
    }
    pub(crate) fn pretty_print(&self) -> String {
        match self.op {
            BinaryOp::Plus => "+".to_string(),
            BinaryOp::Minus => "-".to_string(),
            BinaryOp::Slash => "/".to_string(),
            BinaryOp::Star => "*".to_string(),
            BinaryOp::Greater => ">".to_string(),
            BinaryOp::GreaterEqual => ">=".to_string(),
            BinaryOp::Less => "<".to_string(),
            BinaryOp::LessEqual => "<=".to_string(),
            BinaryOp::BangEqual => "!=".to_string(),
            BinaryOp::EqualEqual => "==".to_string(),
            BinaryOp::Comma => ",".to_string(),
        }
    }
}
impl TokenLike for BinaryToken {
    fn line(&self) -> usize {
        self.line
    }
    fn token_display(&self) -> String {
        self.pretty_print()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum LogicalOp {
    And,
    Or,
}
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct LogicalToken {
    op: LogicalOp,
    line: usize,
}
impl LogicalToken {
    pub(crate) fn new(op: LogicalOp, line: usize) -> Self {
        Self {
            op,
            line,
        }
    }
    pub(crate) fn op(&self) -> &LogicalOp {
        &self.op
    }
    pub(crate) fn pretty_print(&self) -> String {
        match self.op {
            LogicalOp::And => "and".into(),
            LogicalOp::Or => "or".into(),
        }
    }
}
impl TokenLike for LogicalToken {
    fn line(&self) -> usize {
        self.line
    }
    fn token_display(&self) -> String {
        self.pretty_print()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ParsedLiteral {
    Nil,
    True,
    False,
    Number(f64),
    String(String),
}

impl ParsedLiteral {
    pub(crate) fn pretty_print(&self) -> String {
        match self {
            ParsedLiteral::Nil => "nil".to_string(),
            ParsedLiteral::True => "true".to_string(),
            ParsedLiteral::False => "false".to_string(),
            ParsedLiteral::Number(n) => n.to_string(),
            ParsedLiteral::String(s) => s.clone(),
        }
    }
}

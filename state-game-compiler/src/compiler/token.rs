mod token;

pub use token::{lookup_keyword, AssignmentToken, KeywordToken, OperatorToken, PunctuationToken, SpecialToken, CommentToken};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Int(String),
    Float(String),
    String(String),
    Identifier(String),
    //Comment(CommentToken),
    Operator(OperatorToken),
    Assignment(AssignmentToken),
    Punctuation(PunctuationToken),
    Keyword(KeywordToken),
    Special(SpecialToken),
}

impl Token {
    /// Convenience constructor for Identifier tokens.
    pub fn identifier(s: impl Into<String>) -> Self {
        Token::Identifier(s.into())
    }

    /// Convenience constructor for Int tokens.
    pub fn int(s: impl Into<String>) -> Self {
        Token::Int(s.into())
    }

    /// Convenience constructor for Float tokens.
    pub fn float(s: impl Into<String>) -> Self {
        Token::Float(s.into())
    }

    /// Convenience constructor for String tokens.
    pub fn string(s: impl Into<String>) -> Self {
        Token::String(s.into())
    }
}
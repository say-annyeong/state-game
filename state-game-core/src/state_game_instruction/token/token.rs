use std::fmt::{Display, Formatter, Result};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum OperatorToken {
    // Arithmetic
    Plus,       // +
    Minus,      // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %

    // Comparison
    Equal,      // ==
    NotEqual,   // !=
    LessThan,   // <
    GreaterThan,// >
    LessEqual,  // <=
    GreaterEqual,// >=

    // Logical
    And,        // &&
    Or,         // ||
    Not,        // !
}

impl Display for OperatorToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let op_str = match self {
            OperatorToken::Plus => "+",
            OperatorToken::Minus => "-",
            OperatorToken::Multiply => "*",
            OperatorToken::Divide => "/",
            OperatorToken::Modulo => "%",
            OperatorToken::Equal => "==",
            OperatorToken::NotEqual => "!=",
            OperatorToken::LessThan => "<",
            OperatorToken::GreaterThan => ">",
            OperatorToken::LessEqual => "<=",
            OperatorToken::GreaterEqual => ">=",
            OperatorToken::And => "&&",
            OperatorToken::Or => "||",
            OperatorToken::Not => "!",
        };
        write!(f, "{}", op_str)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum AssignmentToken {
    Assign,         // =
    //PlusAssign,     // +=
    //MinusAssign,    // -=
    //MultiplyAssign, // *=
    //DivideAssign,   // /=
    //ModuloAssign,   // %=
    // Add other assignment operators if needed (e.g., bitwise?)
}

impl Display for AssignmentToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let assign_str = match self {
            AssignmentToken::Assign => "=",
            /*
            AssignmentToken::PlusAssign => "+=",
            AssignmentToken::MinusAssign => "-=",
            AssignmentToken::MultiplyAssign => "*=",
            AssignmentToken::DivideAssign => "/=",
            AssignmentToken::ModuloAssign => "%=",
            */
        };
        write!(f, "{}", assign_str)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum PunctuationToken {
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Dot,          // .
    Colon,        // :
    Semicolon,    // ;
    Arrow,        // ->
}

impl Display for PunctuationToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let punc_str = match self {
            PunctuationToken::LeftParen => "(",
            PunctuationToken::RightParen => ")",
            PunctuationToken::LeftBrace => "{",
            PunctuationToken::RightBrace => "}",
            PunctuationToken::LeftBracket => "[",
            PunctuationToken::RightBracket => "]",
            PunctuationToken::Comma => ",",
            PunctuationToken::Dot => ".",
            PunctuationToken::Colon => ":",
            PunctuationToken::Semicolon => ";",
            PunctuationToken::Arrow => "->",
        };
        write!(f, "{}", punc_str)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum KeywordToken {
    Let,
    Fn,
    If,
    Elif,
    Else,
    For,
    While,
    Return,
    Import,
    From,
    As,
    Struct,
    True,
    False,
    Null,
    Break,
    Export,
    Impl,
    // Add other keywords if necessary
}

impl Display for KeywordToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let keyword_str = match self {
            KeywordToken::Let => "let",
            KeywordToken::Fn => "fn",
            KeywordToken::If => "if",
            KeywordToken::Elif => "elif",
            KeywordToken::Else => "else",
            KeywordToken::For => "for",
            KeywordToken::While => "while",
            KeywordToken::Return => "return",
            KeywordToken::Import => "import",
            KeywordToken::From => "from",
            KeywordToken::As => "as",
            KeywordToken::Struct => "struct",
            KeywordToken::True => "true",
            KeywordToken::False => "false",
            KeywordToken::Null => "null",
            KeywordToken::Break => "break",
            KeywordToken::Export => "export",
            KeywordToken::Impl => "impl",
        };
        write!(f, "{}", keyword_str)
    }
}

// Helper function to check if a string is a keyword
pub fn lookup_keyword(s: &str) -> Option<KeywordToken> {
    match s {
        "let" => Some(KeywordToken::Let),
        "fn" => Some(KeywordToken::Fn),
        "if" => Some(KeywordToken::If),
        "elif" => Some(KeywordToken::Elif),
        "else" => Some(KeywordToken::Else),
        "for" => Some(KeywordToken::For),
        "while" => Some(KeywordToken::While),
        "return" => Some(KeywordToken::Return),
        "import" => Some(KeywordToken::Import),
        "from" => Some(KeywordToken::From),
        "as" => Some(KeywordToken::As),
        "struct" => Some(KeywordToken::Struct),
        "true" => Some(KeywordToken::True),
        "false" => Some(KeywordToken::False),
        "null" => Some(KeywordToken::Null),
        "break" => Some(KeywordToken::Break),
        "export" => Some(KeywordToken::Export),
        "impl" => Some(KeywordToken::Impl),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum SpecialToken {
    Eof,      // End of File/Input
    Illegal,  // Represents an unrecognized character or sequence
}

impl Display for SpecialToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            SpecialToken::Eof => write!(f, "EOF"),
            SpecialToken::Illegal => write!(f, "Illegal"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum CommentToken {
    LineComment(String),
    BlockComment(String),
    DocumentLineComment(String),
    DocumentBlockComment(String),
    InnerDocumentComment(String),
}

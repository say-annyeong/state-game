use std::iter::Peekable;
use std::str::Chars;
use crate::compiler::token::{Position, Token, lookup_keyword, AssignmentToken, OperatorToken, PunctuationToken, SpecialToken};

#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    input: Peekable<Chars<'a>>,
    current_position: usize, // Tracks the byte position in the original input string
    line: usize,
    column: usize,
    // Keep track of the start of the *next* token
    start_line: usize,
    start_column: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Tokenizer {
            input: input.chars().peekable(),
            current_position: 0,
            line: 1,
            column: 1, // Start column at 1
            start_line: 1,
            start_column: 1,
        }
    }

    // Helper to advance the iterator and update position/line/column
    fn next_char(&mut self) -> Option<char> {
        match self.input.next() {
            Some(ch) => {
                self.current_position += ch.len_utf8();
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                Some(ch)
            }
            None => None,
        }
    }

    // Helper to peek at the next character without consuming
    fn peek_char(&mut self) -> Option<&char> {
        self.input.peek()
    }

    // Helper to create a Position struct for the *just consumed* token
    fn current_token_pos(&self, start_line: usize, start_column: usize) -> Position {
        Position {
            start_line,
            start_column,
            end_line: self.line,
            end_column: self.column, // `column` points to the *next* char
        }
    }

    // Call this *before* consuming the first char of the next token
    fn mark_token_start(&mut self) {
        self.start_line = self.line;
        self.start_column = self.column;
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(&ch) if ch.is_whitespace() => {
                    self.next_char(); // Consume whitespace
                    self.mark_token_start(); // Reset start after whitespace
                }
                Some(&'/') => {
                    // Peek ahead to see if it's a comment without consuming '/' yet
                    let mut ahead = self.input.clone();
                    ahead.next(); // Advance past the first '/' in the cloned iterator
                    match ahead.peek() {
                        Some('/') => { // It's a // comment
                            self.next_char(); // Consume first '/'
                            self.next_char(); // Consume second '/'
                            // Consume rest of the line
                            while let Some(&c) = self.peek_char() {
                                if c == '\n' {
                                    // Don't consume newline, let next loop handle it
                                    break;
                                }
                                self.next_char();
                            }
                            self.mark_token_start(); // Reset start after // comment
                            // Continue outer loop to handle potential whitespace/comments after this one
                        }
                        Some('*') => { // It's a /* comment
                            self.next_char(); // Consume first '/'
                            self.next_char(); // Consume '*'
                            let mut nesting = 1;
                            while let Some(c) = self.next_char() {
                                if c == '*' {
                                    if let Some(&'/') = self.peek_char() {
                                        self.next_char(); // Consume '/'
                                        nesting -= 1;
                                        if nesting == 0 {
                                            break; // End of multi-line comment
                                        }
                                    }
                                } else if c == '/' {
                                    if let Some(&'*') = self.peek_char() {
                                        self.next_char(); // Consume '*'
                                        nesting += 1; // Start of nested comment
                                    }
                                }
                                // Handle EOF within comment? Should break loop naturally. Add error?
                            }
                            self.mark_token_start(); // Reset start after /* comment
                            if nesting > 0 {
                                // TODO: Record an error for unclosed multi-line comment
                                eprintln!("Warning: Unclosed multi-line comment");
                            }
                            // Continue outer loop
                        }
                        _ => {
                            // First '/' is not followed by '/' or '*', so it's not a comment.
                            // Break the skipping loop, let '/' be handled by next_token.
                            break;
                        }
                    }
                }
                _ => break, // Not whitespace or comment start '/'
            }
        }
    }

    fn read_identifier(&mut self, first_char: char) -> String {
        let mut ident = String::new();
        ident.push(first_char);
        while let Some(&ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.next_char().unwrap()); // Safe unwrap: peeked first
            } else {
                break;
            }
        }
        ident
    }

    // Basic number reader (integer for now)
    // TODO: Add float, hex, binary support
    fn read_number(&mut self, first_char: char) -> String {
        let mut number = String::new();
        number.push(first_char);

        // Consume initial integer part
        while let Some(&ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                number.push(self.next_char().unwrap()); // Safe unwrap: peeked first
            } else {
                break; // Not a digit
            }
        }

        // Check for decimal part
        if let Some(&'.') = self.peek_char() {
            // Peek ahead one more character
            let mut iter_clone = self.input.clone();
            iter_clone.next(); // Advance past the '.' in the clone

            if let Some(&next_ch) = iter_clone.peek() {
                if next_ch.is_ascii_digit() {
                    // It's a valid float decimal part
                    number.push(self.next_char().unwrap()); // Consume the '.'

                    // Consume digits after the decimal point
                    while let Some(&ch) = self.peek_char() {
                        if ch.is_ascii_digit() {
                            number.push(self.next_char().unwrap());
                        } else {
                            break; // Not a digit
                        }
                    }
                } // Else: '.' is not followed by a digit, so treat number as integer and leave '.'
            } // Else: EOF after '.', treat as integer
        } // Else: Not a '.' after integer part

        number
    }

    // Basic string reader
    // TODO: Handle escape sequences
    fn read_string(&mut self, quote_type: char) -> Result<String, String> {
        let mut s = String::new();
        let _start_line = self.line; // Keep original start line for error msg
        let _start_col = self.column; // Keep original start col for error msg

        loop {
            match self.next_char() {
                Some(ch) => {
                    if ch == quote_type {
                        return Ok(s); // End of string
                    } else if ch == '\\' {
                        // Handle escape sequences (basic implementation)
                        match self.next_char() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('r') => s.push('\r'),
                            Some('\\') => s.push('\\'),
                            Some('\'') => s.push('\''),
                            Some('"') => s.push('"'),
                            Some('`') => s.push('`'), // Allow escaping backtick?
                            Some(other) => { // Invalid escape
                                // Option 1: Treat as literal backslash + char
                                // s.push('\\');
                                // s.push(other);
                                // Option 2: Return an error
                                return Err(format!("Invalid escape sequence: \\{}", other));
                            }
                            None => return Err("Unterminated string literal: EOF after backslash".to_string()),
                        }
                    }
                    else if ch == '\n' && quote_type != '`' {
                        // Newlines are only allowed in backtick strings
                        return Err(format!(
                            "Unterminated string literal: newline encountered (started at {}:{})",
                            _start_line, _start_col
                        ));
                    }
                    else {
                        s.push(ch);
                    }
                }
                None => {
                    // Use the captured start line/col for the error message
                    return Err(format!(
                        "Unterminated string literal: EOF encountered (started at {}:{})",
                        _start_line, _start_col
                    )); // End of file before closing quote
                }
            }
        }
    }
    
    pub fn next_token(&mut self) -> (Token, Position) {
        self.skip_whitespace_and_comments();

        // Mark the beginning of the potential token *before* consuming the first char
        self.mark_token_start();
        let start_line = self.start_line;
        let start_col = self.start_column;

        match self.next_char() {
            Some(ch) => {
                let token = match ch {
                    // Punctuation
                    '(' => Token::Punctuation(PunctuationToken::LeftParen),
                    ')' => Token::Punctuation(PunctuationToken::RightParen),
                    '{' => Token::Punctuation(PunctuationToken::LeftBrace),
                    '}' => Token::Punctuation(PunctuationToken::RightBrace),
                    '[' => Token::Punctuation(PunctuationToken::LeftBracket),
                    ']' => Token::Punctuation(PunctuationToken::RightBracket),
                    ',' => Token::Punctuation(PunctuationToken::Comma),
                    '.' => Token::Punctuation(PunctuationToken::Dot),
                    ':' => Token::Punctuation(PunctuationToken::Colon),
                    ';' => Token::Punctuation(PunctuationToken::Semicolon),

                    // Operators & Assignments (potentially multi-char)
                    '+' => match self.peek_char() {
                        //Some(&'=') => { self.next_char(); Token::Assignment(Assignment::PlusAssign) }
                        _ => Token::Operator(OperatorToken::Plus),
                    },
                    '-' => match self.peek_char() {
                        //Some(&'=') => { self.next_char(); Token::Assignment(Assignment::MinusAssign) }
                        Some(&'>') => { self.next_char(); Token::Punctuation(PunctuationToken::Arrow) }
                        _ => Token::Operator(OperatorToken::Minus),
                    },
                    '*' => match self.peek_char() {
                        //Some(&'=') => { self.next_char(); Token::Assignment(Assignment::MultiplyAssign) }
                        _ => Token::Operator(OperatorToken::Multiply),
                    },
                    '/' => match self.peek_char() {
                        // Comments are handled in skip_whitespace_and_comments
                        //Some(&'=') => { self.next_char(); Token::Assignment(Assignment::DivideAssign) }
                        _ => Token::Operator(OperatorToken::Divide), // Should not be reachable if comments handled first
                    },
                    '%' => match self.peek_char() {
                        //Some(&'=') => { self.next_char(); Token::Assignment(Assignment::ModuloAssign) }
                        _ => Token::Operator(OperatorToken::Modulo),
                    },
                    '=' => match self.peek_char() {
                        Some(&'=') => { self.next_char(); Token::Operator(OperatorToken::Equal) }
                        _ => Token::Assignment(AssignmentToken::Assign),
                    },
                    '!' => match self.peek_char() {
                        Some(&'=') => { self.next_char(); Token::Operator(OperatorToken::NotEqual) }
                        _ => Token::Operator(OperatorToken::Not),
                    },
                    '<' => match self.peek_char() {
                        Some(&'=') => { self.next_char(); Token::Operator(OperatorToken::LessEqual) }
                        _ => Token::Operator(OperatorToken::LessThan),
                    },
                    '>' => match self.peek_char() {
                        Some(&'=') => { self.next_char(); Token::Operator(OperatorToken::GreaterEqual) }
                        _ => Token::Operator(OperatorToken::GreaterThan),
                    },
                    '&' => match self.peek_char() {
                        Some(&'&') => { self.next_char(); Token::Operator(OperatorToken::And) }
                        _ => Token::Special(SpecialToken::Illegal), // Single '&' is illegal? Or bitwise? Define later.
                    },
                    '|' => match self.peek_char() {
                        Some(&'|') => { self.next_char(); Token::Operator(OperatorToken::Or) }
                        _ => Token::Special(SpecialToken::Illegal), // Single '|' is illegal? Or bitwise? Define later.
                    },


                    // String Literals
                    '"' | '\'' | '`' => {
                        match self.read_string(ch) {
                            Ok(s) => Token::String(s),
                            Err(e) => {
                                // Handle error - maybe log it? For now, return Illegal
                                eprintln!("Lexer Error: {}", e); // Log error to stderr
                                Token::Special(SpecialToken::Illegal)
                            }
                        }
                    }

                    // Numbers
                    d if d.is_ascii_digit() => {
                        let num_str = self.read_number(d);
                        if num_str.contains('.') {
                            Token::Float(num_str)
                        } else {
                            Token::Int(num_str)
                        }
                        // TODO: Add hex/binary prefix check (0x, 0b) here
                    }

                    // Identifiers or Keywords
                    id_start if id_start.is_alphabetic() || id_start == '_' => {
                        let ident = self.read_identifier(id_start);
                        match lookup_keyword(&ident) {
                            Some(keyword) => Token::Keyword(keyword),
                            None => Token::Identifier(ident),
                        }
                    }

                    // Unrecognized character
                    _ => Token::Special(SpecialToken::Illegal),
                };
                let pos = self.current_token_pos(start_line, start_col); // Calculate pos *after* token is formed
                (token, pos) // Return the tuple
            }
            None => (Token::Special(SpecialToken::Eof), self.current_token_pos(start_line, start_col)), // EOF position
        }
    }
}

// Implement Iterator to make it easy to loop through tokens
impl<'a> Iterator for Tokenizer<'a> {
    type Item = (Token, Position); // Yield token AND position

    fn next(&mut self) -> Option<Self::Item> {
        // next_token now directly returns Option<(Token, Position)> effectively
        let (token, position) = self.next_token();

        match token {
            Token::Special(SpecialToken::Eof) => None, // Stop iteration at EOF
            _ => Some((token, position)), // Return the token and its position
        }
    }
}
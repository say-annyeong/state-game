use super::abstract_syntax_tree_generator::{
    BinaryOperator, BlockStatement, Expression, FieldDefinition, Identifier, IfAlternative,
    ImportDeclaration, ImportSource, Literal, MethodDefinition, Program, Statement, StructDefinition,
    TypeAnnotation, UnaryOperator, ExportDeclaration, ImplMethodDefinition, /* other AST nodes */
};
use super::token::{AssignmentToken, KeywordToken, OperatorToken, PunctuationToken, SpecialToken, Token, Position};
use super::tokenizer::Tokenizer;
use std::iter::Peekable;

// Basic error type for parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    // Found Token (Debug string), Expected description, Position
    UnexpectedToken(String, String, Position),
    // Expected description, Position of EOF
    UnexpectedEof(String, Position),
    // General error message, Position where it occurred (best guess)
    Other(String, Position),
}

// Wrapper for the tokenizer iterator to simplify peeking
type TokenStreamItem = (Token, Position);
type TokenStreamIterator<'a> = Peekable<Tokenizer<'a>>;

pub struct Parser<'a> {
    tokenizer: TokenStreamIterator<'a>,
    // Store the position of the *last consumed* token for error reporting
    last_pos: Position,
    errors: Vec<ParseError>,
}

// --- Helper Macros ---
// Consumes the next token if it matches the pattern, otherwise returns Err.
macro_rules! consume_token {
    ($self:ident, $pattern:pat => $expr:expr, $expected_msg:expr) => {
         {
             // Peek first to get position before consuming
             let pos = match $self.peek_token_pos() {
                Some(p) => *p, // Deref the borrowed position
                None => $self.last_pos, // Use last known position if EOF
             };
             match $self.next_token() {
                 Some($pattern) => Ok($expr),
                 Some(other) => Err(ParseError::UnexpectedToken(
                     format!("{:?}", other),
                     $expected_msg.to_string(),
                     pos // Use the position from peek or last_pos
                 )),
                 None => Err(ParseError::UnexpectedEof(
                     $expected_msg.to_string(),
                     pos // Use the position from peek or last_pos
                 )),
             }
         }
    };
     // Simpler version without extracting value
     ($self:ident, $pattern:pat, $expected_msg:expr) => {
        consume_token!($self, $pattern => (), $expected_msg)
    };
}

// Peeks at the next token, if it matches pattern, consumes it and returns true, otherwise false.
macro_rules! consume_optional_token {
     ($self:ident, $pattern:pat) => {
         if let Some($pattern) = $self.peek_token() {
             $self.next_token(); // Consume
             true
         } else {
             false
         }
     };
 }

// --- Precedence Enum (for future Pratt parser) ---
#[derive(PartialEq, PartialOrd, Ord, Eq, Debug, Clone, Copy)]
enum Precedence {
    Lowest,
    Assign,      // =
    LogicalOr,   // ||
    LogicalAnd,  // &&
    Equality,    // == !=
    Comparison,  // < > <= >=
    Term,        // + -
    Factor,      // * / %
    Unary,       // - !
    Call,        // . () []
}

// Helper to get precedence (defined *before* Parser impl)
fn get_token_precedence(op: &OperatorToken) -> Precedence {
    match op {
        OperatorToken::Equal | OperatorToken::NotEqual => Precedence::Equality,
        OperatorToken::LessThan | OperatorToken::GreaterThan | OperatorToken::LessEqual | OperatorToken::GreaterEqual => Precedence::Comparison,
        OperatorToken::Plus | OperatorToken::Minus => Precedence::Term,
        OperatorToken::Multiply | OperatorToken::Divide | OperatorToken::Modulo => Precedence::Factor,
        OperatorToken::And => Precedence::LogicalAnd,
        OperatorToken::Or => Precedence::LogicalOr,
        OperatorToken::Not => Precedence::Unary, // Assign unary precedence
    }
}

// Helper to get token precedence from a borrowed Token
fn get_peeked_token_precedence(token: &Token) -> Precedence {
    match token {
        Token::Operator(op) => get_token_precedence(op),
        Token::Punctuation(PunctuationToken::LeftParen) => Precedence::Call,
        Token::Punctuation(PunctuationToken::Dot) => Precedence::Call,
        Token::Punctuation(PunctuationToken::LeftBracket) => Precedence::Call,
        Token::Assignment(AssignmentToken::Assign) => Precedence::Assign,
        _ => Precedence::Lowest,
    }
}

impl<'a> Parser<'a> {
    pub fn new(tokenizer: Tokenizer<'a>) -> Self {
        Parser {
            tokenizer: tokenizer.peekable(),
            last_pos: Position::default(),
            errors: Vec::new(),
        }
    }

    // --- Token Helpers --- Updated for (Token, Position) ---
    fn peek_token(&mut self) -> Option<&Token> {
        self.tokenizer.peek().map(|(token, _pos)| token)
    }

    fn peek_token_pos(&mut self) -> Option<&Position> {
        self.tokenizer.peek().map(|(_token, pos)| pos)
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.tokenizer.next() {
            Some((token, pos)) => {
                self.last_pos = pos; // Update last position
                Some(token)
            }
            None => None,
        }
    }

    // Checks the token *kind*, ignoring position
    fn check_peek(&mut self, expected: &Token) -> bool {
        self.peek_token().map_or(false, |t| t == expected)
    }

    // --- Error Reporting ---
    fn record_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    // Consumes the current token if it matches predicate, else records error
    fn expect_and_consume<F>(&mut self, predicate: F, expected_msg: &str) -> Result<(), ParseError>
    where F: FnOnce(&Token) -> bool {
        match self.peek_token() {
            Some(token) if predicate(token) => {
                self.next_token(); // Consume
                Ok(())
            }
            Some(other) => Err(ParseError::UnexpectedToken(format!("{:?}", other), expected_msg.to_string(), self.last_pos)),
            None => Err(ParseError::UnexpectedEof(expected_msg.to_string(), self.last_pos)),
        }
    }


    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    // --- Main Parsing Logic ---
    pub fn parse_program(&mut self) -> Program {
        let mut statements = Vec::new();
        while self.peek_token().is_some()
            && !self.check_peek(&Token::Special(SpecialToken::Eof))
        {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    // Record the first error encountered
                    self.record_error(e);
                    // Stop parsing immediately after the first error
                    break;
                }
            }
        }
        Program { statements }
    }

    // --- Statement Parsers ---
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek_token() {
            Some(Token::Keyword(KeywordToken::Let)) => self.parse_let_declaration(),
            Some(Token::Keyword(KeywordToken::Return)) => self.parse_return_statement(),
            Some(Token::Keyword(KeywordToken::Fn)) => self.parse_fn_declaration(),
            Some(Token::Keyword(KeywordToken::If)) => self.parse_if_statement(),
            Some(Token::Keyword(KeywordToken::While)) => self.parse_while_statement(),
            Some(Token::Keyword(KeywordToken::For)) => self.parse_for_statement(),
            Some(Token::Keyword(KeywordToken::Struct)) => self.parse_struct_declaration(),
            Some(Token::Keyword(KeywordToken::Import)) | Some(Token::Keyword(KeywordToken::From)) => {
                self.parse_import_statement()
            }
            Some(Token::Keyword(KeywordToken::Break)) => self.parse_break_statement(),
            Some(Token::Keyword(KeywordToken::Export)) => self.parse_export_statement(),
            Some(Token::Keyword(KeywordToken::Impl)) => self.parse_impl_block(),
            Some(Token::Punctuation(PunctuationToken::LeftBrace)) => {
                // Allow standalone block? Let's disallow for now.
                Err(ParseError::UnexpectedToken(format!("{:?}", self.peek_token().unwrap()), "Expected statement start KeywordToken or expression".to_string(), self.last_pos))
            }
            // Handle empty statements (lone semicolons)
            Some(Token::Punctuation(PunctuationToken::Semicolon)) => {
                self.next_token(); // Consume the semicolon
                // Return an empty expression statement
                Ok(Statement::ExpressionStatement(Expression::Literal(Literal::Null)))
            }
            Some(_) => self.parse_expression_statement(),
            None => Err(ParseError::UnexpectedEof("Expected statement".to_string(), self.last_pos)),
        }
    }

    fn parse_let_declaration(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'let'
        let name = self.parse_identifier()?;

        let mut type_annotation = None;
        if consume_optional_token!(self, Token::Punctuation(PunctuationToken::Colon)) {
            type_annotation = Some(self.parse_type_annotation()?);
        }

        let mut initializer = None;
        if consume_optional_token!(self, Token::Assignment(AssignmentToken::Assign)) {
            initializer = Some(self.parse_expression(Precedence::Lowest)?);
        }

        // Optional semicolon
        consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));

        Ok(Statement::LetDeclaration {
            name,
            type_annotation,
            initializer,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'return'
        let mut value = None;
        if !self.check_peek(&Token::Punctuation(PunctuationToken::Semicolon))
            && !self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace))
        {
            value = Some(self.parse_expression(Precedence::Lowest)?);
        }
        // Optional semicolon
        consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
        Ok(Statement::ReturnStatement { value })
    }

    fn parse_fn_declaration(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'fn'
        let name = self.parse_identifier()?;

        consume_token!(self, Token::Punctuation(PunctuationToken::LeftParen), "Expected '(' after function name")?;

        let parameters = self.parse_parameter_list()?;

        let mut return_type = None;
        if consume_optional_token!(self, Token::Punctuation(PunctuationToken::Arrow)) {
            return_type = Some(self.parse_type_annotation()?);
        }

        let body = self.parse_block_statement()?;

        Ok(Statement::FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
        })
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<(Identifier, Option<TypeAnnotation>)>, ParseError> {
        let mut params = Vec::new();
        if self.check_peek(&Token::Punctuation(PunctuationToken::RightParen)) {
            self.next_token(); // Consume ')'
            return Ok(params);
        }

        loop {
            let param_name = self.parse_identifier()?;
            let mut param_type = None;
            if consume_optional_token!(self, Token::Punctuation(PunctuationToken::Colon)) {
                param_type = Some(self.parse_type_annotation()?);
            }
            params.push((param_name, param_type));

            if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                break;
            }
        }

        consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' or ',' in parameter list")?;
        Ok(params)
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'if'
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftParen), "Expected '(' after 'if'")?;
        let condition = self.parse_expression(Precedence::Lowest)?;
        consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' after if condition")?;
        let consequence = self.parse_block_statement()?;
        let alternative = self.parse_if_alternative()?;

        Ok(Statement::IfStatement {
            condition,
            consequence,
            alternative,
        })
    }

    fn parse_if_alternative(&mut self) -> Result<Option<IfAlternative>, ParseError> {
        if consume_optional_token!(self, Token::Keyword(KeywordToken::Elif)) {
            consume_token!(self, Token::Punctuation(PunctuationToken::LeftParen), "Expected '(' after 'elif'")?;
            let condition = self.parse_expression(Precedence::Lowest)?;
            consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' after elif condition")?;
            let consequence = self.parse_block_statement()?;
            let alternative = self.parse_if_alternative()?; // Recursive call for more elif/else
            Ok(Some(IfAlternative::Elif {
                condition,
                consequence,
                alternative: alternative.map(Box::new),
            }))
        } else if consume_optional_token!(self, Token::Keyword(KeywordToken::Else)) {
            let consequence = self.parse_block_statement()?;
            Ok(Some(IfAlternative::Else { consequence }))
        } else {
            Ok(None) // No elif or else
        }
    }

    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'while'
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftParen), "Expected '(' after 'while'")?;
        let condition = self.parse_expression(Precedence::Lowest)?;
        consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' after while condition")?;
        let body = self.parse_block_statement()?;
        Ok(Statement::WhileStatement { condition, body })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'for'
        let variable = self.parse_identifier()?;
        // Expect the identifier "in" - not a KeywordToken in the spec
        let in_token = self.next_token().ok_or_else(|| ParseError::UnexpectedEof("Expected 'in' after variable in for loop".to_string(), self.last_pos))?;
        match in_token {
            Token::Identifier(ref s) if s == "in" => { /* continue */ }
            _ => return Err(ParseError::UnexpectedToken(format!("{:?}", in_token), "Expected 'in'".to_string(), self.last_pos)),
        }

        let iterable = self.parse_expression(Precedence::Lowest)?;
        let body = self.parse_block_statement()?;
        Ok(Statement::ForStatement { variable, iterable, body })
    }

    fn parse_struct_declaration(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'struct'
        let name = self.parse_identifier()?;
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBrace), "Expected '{' after struct name")?;

        let mut fields = Vec::new();
        let methods = Vec::new(); // Removed mut

        while !self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace)) && self.peek_token().is_some() {
            // Check if it's a method definition (`fn`) or a field definition
            match self.peek_token() {
                Some(Token::Keyword(KeywordToken::Fn)) => {
                    // TODO: Implement Method parsing - requires handling `self`?
                    // For now, consume tokens related to a potential method to avoid infinite loop
                    self.record_error(ParseError::Other("Method parsing within struct not implemented yet.".to_string(), self.last_pos));
                    // Consume 'fn' and potentially identifier + block to recover somewhat
                    self.next_token(); // consume fn
                    if let Some(Token::Identifier(_)) = self.peek_token() { self.next_token(); }
                    if let Some(Token::Punctuation(PunctuationToken::LeftBrace)) = self.peek_token() {
                        match self.parse_block_statement() { // Attempt to parse block to consume it
                            Ok(_) => {},
                            Err(e) => self.record_error(e),
                        }
                    }

                    // methods.push(self.parse_method_definition()?); // Replace above recovery with this
                }
                Some(Token::Identifier(_)) => {
                    // Assume field definition: name: Type,
                    let field_name = self.parse_identifier()?;
                    consume_token!(self, Token::Punctuation(PunctuationToken::Colon), "Expected ':' after field name")?;
                    let field_type = self.parse_type_annotation()?;
                    consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)); // Optional comma
                    fields.push(FieldDefinition { name: field_name, type_annotation: field_type });
                }
                Some(other) => {
                    let e = ParseError::UnexpectedToken(format!("{:?}", other), "Expected field (identifier) or method ('fn') in struct body".to_string(), self.last_pos);
                    self.record_error(e);
                    self.next_token(); // Consume the unexpected token to attempt recovery
                }
                None => return Err(ParseError::UnexpectedEof("Expected field, method, or '}' in struct body".to_string(), self.last_pos)),
            }
        }

        consume_token!(self, Token::Punctuation(PunctuationToken::RightBrace), "Expected '}' to end struct definition")?;

        Ok(Statement::StructDeclaration(StructDefinition { name, fields, methods }))
    }

    fn parse_import_statement(&mut self) -> Result<Statement, ParseError> {
        if let Some(Token::Keyword(KeywordToken::Import)) = self.peek_token() {
            // Handles: import "path" [as alias];
            self.next_token(); // Consume 'import'
            let path = consume_token!(self, Token::String(s) => s, "Expected string literal for module path after 'import'")?;

            let mut alias = None;
            if consume_optional_token!(self, Token::Keyword(KeywordToken::As)) {
                alias = Some(self.parse_identifier()?);
            }
            consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
            Ok(Statement::ImportStatement(ImportDeclaration::ImportModule { path, alias }))

        } else if let Some(Token::Keyword(KeywordToken::From)) = self.peek_token() {
            // Handles: from "path" import ...; OR from std import ...;
            self.next_token(); // Consume 'from'

            // Determine the source: path string or 'std' identifier
            let source = match self.peek_token() {
                Some(Token::String(_)) => {
                    let path = consume_token!(self, Token::String(s) => s, "Expected string literal for module path after 'from'")?;
                    ImportSource::File(path)
                }
                Some(Token::Identifier(ident)) if ident == "std" => {
                    self.next_token(); // Consume 'std' identifier
                    ImportSource::Std
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(
                        format!("{:?}", self.peek_token()),
                        "Expected module path (string literal) or 'std' after 'from'".to_string(),
                        self.last_pos
                    ));
                }
            };

            consume_token!(self, Token::Keyword(KeywordToken::Import), "Expected 'import' after source in 'from' statement")?;

            let mut symbols = Vec::new();
            loop {
                let name = self.parse_identifier()?;
                let mut alias = None;
                if consume_optional_token!(self, Token::Keyword(KeywordToken::As)) {
                    alias = Some(self.parse_identifier()?);
                }
                symbols.push((name, alias));
                if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                    break;
                }
            }
            consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
            Ok(Statement::ImportStatement(ImportDeclaration::ImportSymbols { source, symbols }))
        } else {
            // Should not happen if called correctly from parse_statement
            Err(ParseError::Other("Internal error: parse_import_statement called unexpectedly".to_string(), self.last_pos))
        }
    }

    fn parse_break_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'break'
        // Optional semicolon
        consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
        Ok(Statement::BreakStatement)
    }

    // --- New Parser for Export Statements ---
    fn parse_export_statement(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'export'

        // Handle `export <name>;`
        let name = self.parse_identifier()?;
        consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
        Ok(Statement::ExportStatement(ExportDeclaration::Identifier(name)))

        // Removed previous match for `module` and specific file path handling
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParseError> {
        // Check if the next token is a semicolon, which would indicate an empty expression
        if self.check_peek(&Token::Punctuation(PunctuationToken::Semicolon)) {
            self.next_token(); // Consume the semicolon
            return Ok(Statement::ExpressionStatement(Expression::Literal(Literal::Null)));
        }

        let expression = self.parse_expression(Precedence::Lowest)?;
        // Optional semicolon
        consume_optional_token!(self, Token::Punctuation(PunctuationToken::Semicolon));
        Ok(Statement::ExpressionStatement(expression))
    }

    // --- Block and Helper Parsers ---
    fn parse_block_statement(&mut self) -> Result<BlockStatement, ParseError> {
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBrace), "Expected '{' to start block")?;
        let mut statements = Vec::new();
        while !self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace))
            && self.peek_token().is_some()
        {
            statements.push(self.parse_statement()?);
        }
        consume_token!(self, Token::Punctuation(PunctuationToken::RightBrace), "Expected '}' to end block")?;
        Ok(BlockStatement { statements })
    }

    fn parse_identifier(&mut self) -> Result<Identifier, ParseError> {
        consume_token!(self, Token::Identifier(s) => Identifier { name: s }, "Expected identifier")
    }

    // Basic version, handles simple `Ident` and `Ident<Type>`
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        let base_ident = self.parse_identifier()?;

        if base_ident.name == "void" {
            return Ok(TypeAnnotation::Void);
        }

        // Check for generic arguments (e.g., List<int>)
        if consume_optional_token!(self, Token::Operator(OperatorToken::LessThan)) {
            let mut arguments = Vec::new();
            if !self.check_peek(&Token::Operator(OperatorToken::GreaterThan)) {
                loop {
                    arguments.push(self.parse_type_annotation()?); // Recursive call
                    if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                        break;
                    }
                }
            }
            // Use expect_and_consume for better error on missing '>'
            self.expect_and_consume(|t| *t == Token::Operator(OperatorToken::GreaterThan), "Expected '>' after generic type arguments")?;

            Ok(TypeAnnotation::Generic { base: base_ident, arguments })
        } else {
            Ok(TypeAnnotation::Simple(base_ident))
        }
    }

    // --- Expression Parsing (Rudimentary) ---
    // Needs replacement with Pratt parsing for correctness
    fn parse_expression(&mut self, _precedence: Precedence) -> Result<Expression, ParseError> {
        // Prefix parsing - Match needs to yield Result<Expression, ParseError>
        let mut left_expr = match self.peek_token() {
            Some(Token::Int(_)) => {
                if let Some(Token::Int(s)) = self.next_token() { // Consume
                    Ok(Expression::Literal(Literal::Int(s)))
                } else { unreachable!("Token mismatch after peek") }
            }
            Some(Token::Float(_)) => {
                if let Some(Token::Float(s)) = self.next_token() { // Consume
                    Ok(Expression::Literal(Literal::Float(s)))
                } else { unreachable!("Token mismatch after peek") }
            }
            Some(Token::String(_)) => {
                if let Some(Token::String(s)) = self.next_token() { // Consume
                    Ok(Expression::Literal(Literal::String(s)))
                } else { unreachable!("Token mismatch after peek") }
            }
            Some(Token::Keyword(KeywordToken::True)) => { self.next_token(); Ok(Expression::Literal(Literal::Boolean(true))) }
            Some(Token::Keyword(KeywordToken::False)) => { self.next_token(); Ok(Expression::Literal(Literal::Boolean(false))) }
            Some(Token::Keyword(KeywordToken::Null)) => { self.next_token(); Ok(Expression::Literal(Literal::Null)) }
            Some(Token::Identifier(_)) => {
                // Check if it's an identifier followed by { (struct initializer)
                let ident_token = self.next_token().unwrap(); // Consume Ident
                if self.check_peek(&Token::Punctuation(PunctuationToken::LeftBrace)) {
                    // It's a struct initializer
                    let name = match ident_token {
                        Token::Identifier(s) => Identifier { name: s },
                        _ => unreachable!("Checked for Ident already"),
                    };
                    Ok(self.parse_struct_initializer(name)?) // Wrap in Ok
                } else {
                    // It's a regular identifier variable
                    match ident_token {
                        Token::Identifier(s) => Ok(Expression::Identifier(Identifier { name: s })),
                        _ => unreachable!("Checked for Ident already"),
                    }
                }
            }
            Some(Token::Punctuation(PunctuationToken::LeftParen)) => {
                self.next_token(); // Consume '('
                let expr = self.parse_expression(Precedence::Lowest)?;
                consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' after grouped expression")?;
                Ok(expr)
            }
            Some(Token::Operator(OperatorToken::Minus)) | Some(Token::Operator(OperatorToken::Not)) => {
                let op_token = self.next_token().unwrap(); // Consume OperatorToken
                let op = if op_token == Token::Operator(OperatorToken::Minus) { UnaryOperator::Negate } else { UnaryOperator::Not };
                let operand = self.parse_expression(Precedence::Unary)?; // Parse with higher precedence
                Ok(Expression::UnaryOperation { op, operand: Box::new(operand) })
            }
            // These calls return Result, so just pass them through
            Some(Token::Punctuation(PunctuationToken::LeftBracket)) => self.parse_list_initializer(),
            Some(Token::Punctuation(PunctuationToken::LeftBrace)) => self.parse_dict_initializer(),

            Some(other) => {
                let token_str = format!("{:?}", other.clone());
                Err(ParseError::UnexpectedToken(
                    token_str,
                    "Expected literal, identifier, '(', '-', '!', '[', or '{'".to_string(),
                    self.last_pos
                ))
            }
            None => Err(ParseError::UnexpectedEof("Expected expression".to_string(), self.last_pos)),
        }?; // Apply '?' to the Result produced by the match block

        // --- Infix/Postfix parsing loop --- (Revised for correct precedence climbing)
        loop {
            // Peek FIRST to check precedence before deciding to proceed
            let peeked_token = match self.peek_token() {
                Some(t) => t,
                None => break, // No more tokens
            };

            let peeked_precedence = get_peeked_token_precedence(peeked_token);

            // The core Pratt check: only continue if the next token binds tighter than the current context
            if _precedence >= peeked_precedence {
                break;
            }

            // Now consume the token since we know we're handling it
            // Use next_token() directly inside the match for clarity
            match self.next_token().unwrap() { // Safe unwrap: we peeked Some(t) just before
                Token::Operator(op) => {
                    // We already know op is not OperatorToken::Not from prefix handling
                    let binary_op = match op {
                        OperatorToken::Plus => BinaryOperator::Add,
                        OperatorToken::Minus => BinaryOperator::Subtract,
                        OperatorToken::Multiply => BinaryOperator::Multiply,
                        OperatorToken::Divide => BinaryOperator::Divide,
                        OperatorToken::Modulo => BinaryOperator::Modulo,
                        OperatorToken::Equal => BinaryOperator::Equal,
                        OperatorToken::NotEqual => BinaryOperator::NotEqual,
                        OperatorToken::LessThan => BinaryOperator::LessThan,
                        OperatorToken::GreaterThan => BinaryOperator::GreaterThan,
                        OperatorToken::LessEqual => BinaryOperator::LessEqual,
                        OperatorToken::GreaterEqual => BinaryOperator::GreaterEqual,
                        OperatorToken::And => BinaryOperator::And,
                        OperatorToken::Or => BinaryOperator::Or,
                        OperatorToken::Not => unreachable!("Not OperatorToken should be handled by prefix parser"),
                    };
                    // Parse right operand with the *current* OperatorToken's precedence as the context
                    let right_expr = self.parse_expression(peeked_precedence)?;
                    left_expr = Expression::BinaryOperation {
                        left: Box::new(left_expr),
                        op: binary_op,
                        right: Box::new(right_expr),
                    };
                }
                Token::Assignment(AssignmentToken::Assign) => {
                    // AssignmentToken is right-associative, parse RHS with slightly lower precedence
                    // We subtract 1 from the current precedence level for right-associativity.
                    let current_precedence = Precedence::Assign;
                    // Handle potential underflow if Lowest is 0. Ensure Lowest is truly the lowest.
                    let right_precedence = if current_precedence > Precedence::Lowest {
                        // Find the next lower precedence level (this is slightly hacky without explicit enum values)
                        // A better approach might be to define integer values for precedence levels.
                        // For now, just using Lowest seems the most robust way to handle right-associativity
                        // in the absence of easily decrementable precedence. Let's revert to that.
                        // current_precedence - 1; // This requires defining subtraction or integer values.
                        Precedence::Lowest // Reverting to Lowest - Less correct but avoids complex enum logic for now.
                        // NOTE: This makes AssignmentToken LEFT-associative (a = b = c -> (a = b) = c)
                        // We need a better precedence system to fix this properly. For example:
                        // let value = self.parse_expression(Precedence::Assign - 1)?; // If precedence were integers
                    } else {
                        Precedence::Lowest
                    };

                    // Parse the right side of the AssignmentToken (the value being assigned)
                    let value = self.parse_expression(Precedence::Lowest)?; // Keep as Lowest for left-associative behavior

                    // Check if left_expr is a valid L-value (Identifier, MemberAccess, etc.)
                    // This check is better done during semantic analysis or interpretation.
                    left_expr = Expression::Assignment {
                        target: Box::new(left_expr),
                        value: Box::new(value),
                    };
                }
                Token::Punctuation(PunctuationToken::LeftParen) => {
                    // Arguments are parsed with lowest precedence
                    let arguments = self.parse_expression_list(PunctuationToken::RightParen)?;
                    consume_token!(self, Token::Punctuation(PunctuationToken::RightParen), "Expected ')' after function arguments")?;
                    left_expr = Expression::FunctionCall {
                        callee: Box::new(left_expr),
                        arguments,
                    };
                }
                Token::Punctuation(PunctuationToken::Dot) => {
                    let member = self.parse_identifier()?;
                    left_expr = Expression::MemberAccess {
                        base: Box::new(left_expr),
                        member,
                    };
                }
                Token::Punctuation(PunctuationToken::LeftBracket) => {
                    // Index expression is parsed with lowest precedence
                    let index = self.parse_expression(Precedence::Lowest)?;
                    consume_token!(self, Token::Punctuation(PunctuationToken::RightBracket), "Expected ']' after index expression")?;
                    left_expr = Expression::IndexAccess {
                        base: Box::new(left_expr),
                        index: Box::new(index),
                    };
                }
                // This case should ideally not be reached if peeked_precedence logic is correct
                other => return Err(ParseError::Other(format!("Unexpected token {:?} in infix/postfix position after precedence check", other), self.last_pos)),
            }
        }

        Ok(left_expr)
    }

    // Helper for parsing comma-separated expressions until a closing delimiter
    fn parse_expression_list(&mut self, end_delimiter: PunctuationToken) -> Result<Vec<Expression>, ParseError> {
        let mut list = Vec::new();
        let end_token = Token::Punctuation(end_delimiter);

        if self.check_peek(&end_token) {
            // Empty list is handled by the caller consuming the end token
            return Ok(list);
        }

        loop {
            list.push(self.parse_expression(Precedence::Lowest)?);
            if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                break; // No comma, expect end delimiter next
            }
            if self.check_peek(&end_token) {
                // Allow trailing comma
                break;
            }
        }
        // Caller should consume the end_delimiter
        Ok(list)
    }

    fn parse_list_initializer(&mut self) -> Result<Expression, ParseError> {
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBracket), "Expected '[' for list initializer")?;
        let items = self.parse_expression_list(PunctuationToken::RightBracket)?;
        consume_token!(self, Token::Punctuation(PunctuationToken::RightBracket), "Expected ']' after list items")?;
        Ok(Expression::ListInitializer { items })
    }

    fn parse_dict_initializer(&mut self) -> Result<Expression, ParseError> {
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBrace), "Expected '{' for dict initializer")?;
        let mut pairs = Vec::new();
        let end_token = Token::Punctuation(PunctuationToken::RightBrace);

        if !self.check_peek(&end_token) {
            loop {
                let key = self.parse_expression(Precedence::Lowest)?; // Allow expressions as keys? Spec shows strings.
                consume_token!(self, Token::Punctuation(PunctuationToken::Colon), "Expected ':' after dict key")?;
                let value = self.parse_expression(Precedence::Lowest)?;
                pairs.push((key, value));

                if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                    break;
                }
                if self.check_peek(&end_token) {
                    // Allow trailing comma
                    break;
                }
            }
        }
        consume_token!(self, Token::Punctuation(PunctuationToken::RightBrace), "Expected '}' after dict items")?;
        Ok(Expression::DictInitializer { pairs })
    }

    // --- Helper for Struct Initializer --- Added
    fn parse_struct_initializer(&mut self, name: Identifier) -> Result<Expression, ParseError> {
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBrace), "Expected '{' after struct name for initializer")?;
        let mut fields = Vec::new();

        // Parse field initializers until '}'
        if !self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace)) {
            loop {
                let field_name = self.parse_identifier()?;
                consume_token!(self, Token::Punctuation(PunctuationToken::Colon), "Expected ':' after field name in struct initializer")?;
                let field_value = self.parse_expression(Precedence::Lowest)?;
                fields.push((field_name, field_value));

                if !consume_optional_token!(self, Token::Punctuation(PunctuationToken::Comma)) {
                    break; // No comma, expect '}'
                }
                if self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace)) {
                    // Allow trailing comma
                    break;
                }
            }
        }

        consume_token!(self, Token::Punctuation(PunctuationToken::RightBrace), "Expected '}' to end struct initializer")?;
        Ok(Expression::StructInitializer { name, fields })
    }

    // --- New Parser for Impl Blocks ---
    fn parse_impl_block(&mut self) -> Result<Statement, ParseError> {
        self.next_token(); // Consume 'impl'
        let struct_name = self.parse_identifier()?;
        consume_token!(self, Token::Punctuation(PunctuationToken::LeftBrace), "Expected '{' after impl <StructName>")?;

        let mut methods = Vec::new();
        while !self.check_peek(&Token::Punctuation(PunctuationToken::RightBrace)) && self.peek_token().is_some() {
            // Expect function definitions within the impl block
            if self.check_peek(&Token::Keyword(KeywordToken::Fn)) {
                methods.push(self.parse_impl_method_definition()?);
            } else {
                return Err(ParseError::UnexpectedToken(
                    format!("{:?}", self.peek_token()),
                    "Expected 'fn' for method definition inside impl block".to_string(),
                    self.last_pos
                ));
            }
        }

        consume_token!(self, Token::Punctuation(PunctuationToken::RightBrace), "Expected '}' to end impl block")?;

        Ok(Statement::ImplBlock {
            struct_name,
            methods,
        })
    }

    // --- New Helper for Parsing Method Definition inside Impl ---
    // Very similar to parse_fn_declaration, but returns ImplMethodDefinition
    fn parse_impl_method_definition(&mut self) -> Result<ImplMethodDefinition, ParseError> {
        self.next_token(); // Consume 'fn'
        let name = self.parse_identifier()?;

        consume_token!(self, Token::Punctuation(PunctuationToken::LeftParen), "Expected '(' after method name")?;

        // Parse parameters - check for 'self' as the first parameter
        let parameters = self.parse_parameter_list()?;
        // TODO: Add validation later to ensure 'self' (if present) is the first parameter
        // and potentially handle different self types (&self, &mut self)?

        let mut return_type = None;
        if consume_optional_token!(self, Token::Punctuation(PunctuationToken::Arrow)) {
            return_type = Some(self.parse_type_annotation()?);
        }

        let body = self.parse_block_statement()?;

        Ok(ImplMethodDefinition {
            name,
            parameters,
            return_type,
            body,
        })
    }

}

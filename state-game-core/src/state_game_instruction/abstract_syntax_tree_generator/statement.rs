use super::expression::Expression;
use super::{
    Identifier, // Defined in mod.rs
    TypeAnnotation, // Defined in mod.rs
    ImplMethodDefinition // Added ImplMethodDefinition
};
use super::import::ImportDeclaration;
use super::struct_definition::StructDefinition;

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    LetDeclaration {
        name: Identifier,
        type_annotation: Option<TypeAnnotation>,
        initializer: Option<Expression>,
    },
    FunctionDeclaration {
        name: Identifier,
        parameters: Vec<(Identifier, Option<TypeAnnotation>)>,
        return_type: Option<TypeAnnotation>,
        body: BlockStatement, // Requires BlockStatement definition
    },
    StructDeclaration(StructDefinition),
    IfStatement {
        condition: Expression,
        consequence: BlockStatement, // Requires BlockStatement definition
        alternative: Option<IfAlternative>, // Requires IfAlternative definition
    },
    ForStatement {
        variable: Identifier,
        iterable: Expression,
        body: BlockStatement, // Requires BlockStatement definition
    },
    WhileStatement {
        condition: Expression,
        body: BlockStatement, // Requires BlockStatement definition
    },
    ReturnStatement {
        value: Option<Expression>,
    },
    ExpressionStatement(Expression), // For expressions used as statements (e.g., function calls)
    ImportStatement(ImportDeclaration),
    // Add break statement support
    BreakStatement,
    ExportStatement(ExportDeclaration),
    ImplBlock { // Added
        struct_name: Identifier,
        methods: Vec<ImplMethodDefinition>,
    },
    // EmptyStatement // Optional, if needed for things like `;;` 
}

// Helper type for the body of functions, loops, if blocks etc.
#[derive(Debug, PartialEq, Clone)]
pub struct BlockStatement {
    pub statements: Vec<Statement>,
}

// Helper type for `elif` and `else` blocks
#[derive(Debug, PartialEq, Clone)]
pub enum IfAlternative {
    Elif {
        condition: Expression,
        consequence: BlockStatement,
        alternative: Option<Box<IfAlternative>>, // Chain elifs
    },
    Else {
        consequence: BlockStatement,
    },
}

// --- New Enum for Export Declarations ---
#[derive(Debug, PartialEq, Clone)]
pub enum ExportDeclaration {
    // Module(Identifier),         // Removed: export module <name>;
    Identifier(Identifier),     // export <name>; (Loads <name>.oxy or <name>/mod.oxy)
}

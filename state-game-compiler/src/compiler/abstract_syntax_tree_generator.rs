pub mod expression;
pub mod import;
pub mod literal;
pub mod statement;
pub mod struct_definition;

pub use expression::Expression;
pub use import::{ImportDeclaration, ImportSource};
pub use literal::Literal;
pub use statement::{BlockStatement, IfAlternative, Statement, ExportDeclaration};
pub use struct_definition::{FieldDefinition, MethodDefinition, StructDefinition};

// --- Shared Basic Types ---

/// Represents an identifier (e.g., variable name, function name).
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Identifier {
    pub name: String,
    // Potentially add span/location info here later
}

/// Represents a type annotation (e.g., `: int`, `: string`, `: List<int>`).
#[derive(Debug, PartialEq, Clone)]
pub enum TypeAnnotation {
    Simple(Identifier), // e.g., `int`, `string`, `MyStruct`
    Generic {
        base: Identifier, // e.g., `List`, `Dict`
        arguments: Vec<TypeAnnotation>, // e.g., `[int]`, `[string, int]`
    },
    // Add `void` type specifically? Or handle as a special Simple identifier?
    Void, // Explicit void type for function returns
}

// --- Operator Enums --- Moved here from expression.rs

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum BinaryOperator {
    Add, Subtract, Multiply, Divide, Modulo,
    Equal, NotEqual, LessThan, GreaterThan, LessEqual, GreaterEqual,
    And, Or,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum UnaryOperator {
    Not, Negate,
}

// --- New Enum for Method Definitions (inside ImplBlock) ---
// Slightly different from function declaration as it belongs to an impl block
#[derive(Debug, PartialEq, Clone)]
pub struct ImplMethodDefinition {
    pub name: Identifier,
    pub parameters: Vec<(Identifier, Option<TypeAnnotation>)>, // Includes 'self' if present
    pub return_type: Option<TypeAnnotation>,
    pub body: BlockStatement,
}

/// Represents the top-level structure of an Oxygen program (a list of statements).
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

use super::literal::Literal;
use super::{
    BinaryOperator, // Defined in mod.rs
    Identifier,     // Defined in mod.rs
    UnaryOperator,  // Defined in mod.rs
};
use super::statement::Statement; // Needed for block expressions if added later

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(Identifier),
    BinaryOperation {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOperation {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
    FunctionCall {
        callee: Box<Expression>, // Can be Identifier or MemberAccess
        arguments: Vec<Expression>,
    },
    ListInitializer {
        items: Vec<Expression>,
    },
    DictInitializer {
        pairs: Vec<(Expression, Expression)>, // Key-value pairs
    },
    IndexAccess {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    MemberAccess {
        base: Box<Expression>,
        member: Identifier, // The field or method name being accessed
    },
    Assignment {
        target: Box<Expression>, // L-value: Identifier, MemberAccess, IndexAccess
        value: Box<Expression>,
    },
    StructInitializer {
        name: Identifier,
        fields: Vec<(Identifier, Expression)>,
    },
    // TODO: Add BlockExpression if needed (e.g., for if/else expressions)
}

// Operator enums are defined in astgen/mod.rs

use super::statement::BlockStatement;
use super::{Identifier, TypeAnnotation};

/// Represents the definition of a struct.
#[derive(Debug, PartialEq, Clone)]
pub struct StructDefinition {
    pub name: Identifier,
    pub fields: Vec<FieldDefinition>,
    pub methods: Vec<MethodDefinition>,
}

/// Represents a field within a struct definition.
#[derive(Debug, PartialEq, Clone)]
pub struct FieldDefinition {
    pub name: Identifier,
    pub type_annotation: TypeAnnotation, // Assuming fields must have type annotations
}

/// Represents a method definition within a struct.
#[derive(Debug, PartialEq, Clone)]
pub struct MethodDefinition {
    pub name: Identifier,
    pub parameters: Vec<(Identifier, Option<TypeAnnotation>)>, // `self` might be implicit or explicit?
    pub return_type: Option<TypeAnnotation>,
    pub body: BlockStatement,
} 
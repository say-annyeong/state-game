use crate::runtime_task::instruction::FunctionIdentifier;
use crate::runtime_task::types::{ConcreteType, TypeExpression};

pub struct ExpressionFunctionMetadata {
    pub function_identifier: FunctionIdentifier,
    pub slot_types: Vec<TypeExpression>,
    pub input_types: Vec<TypeExpression>,
    pub output_types: Vec<TypeExpression>,
}

pub struct ConcreteFunctionMetadata {
    pub function_identifier: FunctionIdentifier,
    pub sub_function_identifier: FunctionIdentifier,
    pub slot_types: Vec<ConcreteType>,
    pub input_types: Vec<ConcreteType>,
    pub output_types: Vec<ConcreteType>,
}

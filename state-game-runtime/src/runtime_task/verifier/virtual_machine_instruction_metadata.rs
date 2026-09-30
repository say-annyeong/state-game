use crate::runtime_task::instruction::FunctionIdentifier;
use crate::runtime_task::types::{ConcreteType, TypeExpression};

pub struct ExpressionFunctionMetadata<'a> {
    pub function_identifier: FunctionIdentifier,
    pub slot_types: Vec<TypeExpression<'a>>,
    pub input_types: Vec<TypeExpression<'a>>,
    pub output_types: Vec<TypeExpression<'a>>,
}

pub struct ConcreteFunctionMetadata<'a> {
    pub function_identifier: FunctionIdentifier,
    pub sub_function_identifier: FunctionIdentifier,
    pub slot_types: Vec<ConcreteType<'a>>,
    pub input_types: Vec<ConcreteType<'a>>,
    pub output_types: Vec<ConcreteType<'a>>,
}

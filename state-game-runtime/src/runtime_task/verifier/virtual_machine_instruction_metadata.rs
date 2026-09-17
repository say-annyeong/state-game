use crate::runtime_task::instruction::FunctionIdentifier;
use crate::runtime_task::types::{ConcreteType, TypeExpression};

pub struct ExpressionInstructionMetadata<'a> {
    pub slot_types: Vec<TypeExpression<'a>>,
}

pub struct ConcreteInstructionMetadata<'a> {
    pub slot_types: Vec<ConcreteType<'a>>,
}

pub struct ExpressionFunctionMetadata<'a> {
    pub function_identifier: FunctionIdentifier,
    pub slot_types: Vec<TypeExpression<'a>>,
    pub input_types: Vec<TypeExpression<'a>>,
    pub output_types: Vec<TypeExpression<'a>>,
}

impl<'a> ExpressionFunctionMetadata<'a> {
    pub fn new(
        slot_types: Vec<TypeExpression<'a>>,
        input_types: Vec<TypeExpression<'a>>,
        output_types: Vec<TypeExpression<'a>>,
        function_identifier: FunctionIdentifier,
    ) -> Self {
        Self {
            function_identifier,
            slot_types,
            input_types,
            output_types,
        }
    }
}

pub struct ConcreteFunctionMetadata<'a> {
    pub function_identifier: FunctionIdentifier,
    pub sub_function_identifier: FunctionIdentifier,
    pub slot_types: Vec<ConcreteType<'a>>,
    pub input_types: Vec<ConcreteType<'a>>,
    pub output_types: Vec<ConcreteType<'a>>,
}

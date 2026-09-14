use crate::runtime_task::instruction::FunctionIdentifier;
use crate::runtime_task::types::{ConcreteType, TypeExpression};

pub struct InstructionMetadata<'a> {
    pub slot_types: Vec<TypeExpression<'a>>,
}

pub struct ExpressionFunctionMetadata<'a> {
    pub slot_types: Vec<TypeExpression<'a>>,
    pub input_types: Vec<TypeExpression<'a>>,
    pub output_types: Vec<TypeExpression<'a>>,
}

impl<'a> ExpressionFunctionMetadata<'a> {
    pub fn new(
        slot_types: Vec<TypeExpression<'a>>,
        input_types: Vec<TypeExpression<'a>>,
        output_types: Vec<TypeExpression<'a>>,
    ) -> Self {
        Self {
            slot_types,
            input_types,
            output_types,
        }
    }
}

pub struct ConcreteFunctionMetadata<'a> {
    pub slot_types: Vec<ConcreteType<'a>>,
    pub input_types: Vec<ConcreteType<'a>>,
    pub output_types: Vec<ConcreteType<'a>>,
}

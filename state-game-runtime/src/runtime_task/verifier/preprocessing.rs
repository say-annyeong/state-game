use std::sync::Arc;
use crate::{
    runtime_task::{
        instruction::{ConcreteInstruction, ExpressionInstruction},
        verifier::virtual_machine_instruction_metadata::{ConcreteFunctionMetadata, ExpressionFunctionMetadata}
    }
};
use crate::runtime_task::instruction::{Functions, SpecialFunctions};
use crate::runtime_task::types::{GenericIdentifier, PrimitiveType, TypeExpression};

struct Preprocessing<'a> {
    expression_function_metadata: &'a [(ExpressionFunctionMetadata<'a>, Arc<[ExpressionInstruction<'a>]>)]
}

impl<'a> Preprocessing<'a> {
    fn new(expression_function_metadata: &'a [(ExpressionFunctionMetadata<'a>, Arc<[ExpressionInstruction<'a>]>)]) -> Self {
        Self { expression_function_metadata }
    }

    fn preprocess(&self) -> Arc<[(ConcreteFunctionMetadata<'a>, Arc<[ConcreteInstruction<'a>]>)]> {
        for (expression_function_metadata, instructions) in self.expression_function_metadata {
            let ExpressionFunctionMetadata { input_types, output_types,  .. } = expression_function_metadata;
            let mut result_slot_types: Vec<(u64, TypeExpression)> = Vec::new();

            for (instruction_position, instruction) in instructions.iter().enumerate() {
                match instruction {
                    ExpressionInstruction::Bind { slot, type_name, .. } => {
                        result_slot_types.push((*slot, TypeExpression::Primitive(type_name.clone())));
                    }
                    ExpressionInstruction::Call { function_name, inputs, output, .. } => {
                        match function_name {
                            Functions::AddInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::SubInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::MulInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::DivInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::RemInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::RemEuclidInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::PowInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::AddFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::SubFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::MulFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::DivFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::PowFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::EqualInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::NotEqualInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::GreaterThanInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::LessThanInteger => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::GreaterThanFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::LessThanFloat => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Float)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::Not => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::And => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::Or => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::Xor => {
                                let input1 = inputs.get(0);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::Boolean)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::EqualString => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            Functions::StringLength => {
                                let input1 = inputs.get(0);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::StringGetChar => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::Integer)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Char)))
                            }
                            Functions::StringCombine => {
                                let input1 = inputs.get(0);
                                let input2 = inputs.get(1);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                if let Some(input2) = input2 {
                                    result_slot_types.push((*input2, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::String)))
                            }
                            Functions::StringToInteger => {
                                let input1 = inputs.get(0);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            Functions::StringToFloat => {
                                let input1 = inputs.get(0);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Float)))
                            }
                            Functions::StringToBoolean => {
                                let input1 = inputs.get(0);
                                let output = output;
                                if let Some(input1) = input1 {
                                    result_slot_types.push((*input1, TypeExpression::Primitive(PrimitiveType::String)))
                                }
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Boolean)))
                            }
                            _ => {}
                        }
                    }
                    ExpressionInstruction::SpecialCall { function_name, inputs, output, .. } => {
                        match function_name {
                            SpecialFunctions::GetInstructionPosition => {
                                result_slot_types.push((*output, TypeExpression::Primitive(PrimitiveType::Integer)))
                            }
                            SpecialFunctions::GetModificationNamespaceList => {
                                result_slot_types.push((*output, TypeExpression::Vector(&TypeExpression::Primitive(PrimitiveType::String))))
                            }
                            _ => {}
                        }
                    }
                    ExpressionInstruction::ConditionalJump { condition, .. } => {
                        result_slot_types.push((*condition, TypeExpression::Primitive(PrimitiveType::Boolean)))
                    }
                    _ => {}
                }
            }

            loop {
                let mut slot_types: Vec<(u64, TypeExpression)> = result_slot_types.clone();
                let mut generic_content: Vec<TypeExpression> = Vec::new();
                for instruction in instructions.iter() {
                    match instruction {
                        ExpressionInstruction::Call { function_name, inputs, output, generic_arguments } => {
                            match function_name {
                                Functions::VectorGet => {
                                    let input1 = inputs.get(0);
                                    let input2 = inputs.get(1);
                                    let output = output;
                                    if let Some(input1) = input1 {
                                        result_slot_types.push((*input1, TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(generic_content.len() as u64)))));
                                        generic_content.push()
                                    }
                                }
                                Functions::VectorNew => {

                                }
                                Functions::VectorPush => {

                                }
                                Functions::VectorPop => {
                                    
                                }
                                Functions::IsSome => {

                                }
                                Functions::IsNone => {

                                }
                                Functions::IsOk => {

                                }
                                Functions::IsErr => {

                                }
                                Functions::UnwrapSome => {

                                }
                                Functions::UnwrapOk => {

                                }
                                Functions::UnwrapErr => {

                                }
                                _ => {}
                            }
                        }
                        ExpressionInstruction::SpecialCall { function_name, inputs, output, .. } => {
                            match function_name {
                                SpecialFunctions::ReadGlobalMemory => {}
                                SpecialFunctions::WriteGlobalMemory => {}
                                SpecialFunctions::GetInputSlot => {}
                                SpecialFunctions::WriteOutputSlot => {}
                                _ => {}
                            }
                        }
                        ExpressionInstruction::DefinedCall { .. } => {}
                        ExpressionInstruction::ReturnDefinedCall { .. } => {}
                        _ => {}
                    }
                }

                if result_slot_types == slot_types {
                    break;
                }

                result_slot_types = slot_types;
            }
            result_slot_types.sort_by(|a, b| a.0.cmp(&b.0));

            let max_index = result_slot_types
                .iter()
                .map(|(slot_index, _)| *slot_index)
                .max()
                .unwrap_or(0);
        }

        loop {

        }

        todo!()
    }
}
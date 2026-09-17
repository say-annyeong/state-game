use std::{sync::Arc};

use crate::{
    runtime_task::{
        types::{
            ConcreteType,
            PrimitiveType,
            TypeExpression
        },
        instruction::{
            FUNCTION_REGISTRY,
            FunctionIdentifier,
            Functions,
            Instruction,
            Literal,
            SPECIAL_FUNCTIONS_REGISTRY,
            Slot,
            SpecialFunctions,
            ConcreteInstruction
        }
    },
    runtime_task::verifier::virtual_machine_instruction_metadata::{ConcreteFunctionMetadata}
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VerifyError<'a> {
    /// A slot was read before it was assigned.
    UnboundSlot {
        instruction_pointer: usize,
        slot: Slot,
    },
    /// The type of a slot did not match what was expected.
    TypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: TypeExpression<'a>,
        found: ConcreteType<'a>,
    },
    /// The literal value in a Bind cannot be parsed as the declared type.
    InvalidLiteral {
        instruction_pointer: usize,
        slot: Slot,
    },
    /// A Jump or ConditionalJump target is outside the instruction list.
    JumpOutOfBounds {
        instruction_pointer: usize,
        target: usize,
    },
    /// The wrong number of arguments was supplied to a Call.
    ArgumentCountMismatch {
        instruction_pointer: usize,
        expected: usize,
        found: usize,
    },
    CanNotFoundDefinedFunction {
        instruction_pointer: usize,
        function_identifier: FunctionIdentifier,
    },
    DefinedFunctionArgumentCountMismatch {
        instruction_pointer: usize,
        expected: usize,
        found: usize,
    },
    DefinedFunctionReturnCountMismatch {
        instruction_pointer: usize,
        expected: usize,
        found: usize,
    },
}

pub struct InstructionVerifier<'a> {
    function_instruction: Arc<[(ConcreteFunctionMetadata<'a>, Arc<[ConcreteInstruction<'a>]>)]>,
}

impl<'a> InstructionVerifier<'a> {
    pub fn new(
        function_instruction: Arc<[(ConcreteFunctionMetadata<'a>, Arc<[ConcreteInstruction<'a>]>)]>,
    ) -> Self {
        Self {
            function_instruction,
        }
    }

    /// Verifies the instruction stream and returns all errors found.
    /// An empty Vec means the program is well-typed.
    pub fn verify(&self) -> Vec<VerifyError> {
        let instructions = &self.function_instruction;
        let len = instructions.len();
        let mut errors = Vec::new();
        let mut types = Vec::new();

        for (concrete_function_metadata,  instructions) in instructions.iter() {
            for (instruction_pointer, instruction) in instructions.iter().enumerate() {
                match instruction {
                    Instruction::Bind {
                        slot,
                        type_name,
                        value,
                    } => {
                        Self::verify_bind(
                            instruction_pointer,
                            *slot,
                            *type_name,
                            value,
                            &mut errors,
                            &mut types,
                        );
                    }
                    Instruction::Call {
                        function_name,
                        generic_arguments,
                        inputs,
                        output,
                    } => {
                        Self::verify_call(
                            instruction_pointer,
                            *function_name,
                            generic_arguments,
                            inputs,
                            *output,
                            &mut errors,
                            &mut types,
                        );
                    }
                    Instruction::SpecialCall {
                        function_name,
                        generic_arguments,
                        inputs,
                        output,
                    } => {
                        Self::verify_special_call(
                            instruction_pointer,
                            *function_name,
                            generic_arguments,
                            inputs,
                            *output,
                            &mut errors,
                            &mut types,
                        );
                    }
                    Instruction::DefinedCall {
                        function_identifier,
                        generic_arguments,
                        inputs,
                        outputs,
                    } => {
                        self.verify_defined_call(
                            instruction_pointer,
                            function_identifier,
                            generic_arguments,
                            inputs,
                            outputs,
                            &mut errors,
                            &mut types,
                        );
                    }
                    Instruction::Jump { target_position } => {
                        if *target_position >= len {
                            errors.push(VerifyError::JumpOutOfBounds {
                                instruction_pointer,
                                target: *target_position,
                            });
                        }
                    }
                    Instruction::ConditionalJump {
                        condition,
                        true_target_position,
                        false_target_position,
                    } => {
                        match types.get(*condition as usize) {
                            None => errors.push(VerifyError::UnboundSlot {
                                instruction_pointer,
                                slot: *condition,
                            }),
                            Some(t) if *t != ConcreteType::Primitive(PrimitiveType::Boolean) => {
                                errors.push(VerifyError::TypeMismatch {
                                    instruction_pointer,
                                    slot: *condition,
                                    expected: TypeExpression::Primitive(PrimitiveType::Boolean),
                                    found: t.clone(),
                                });
                            }
                            _ => {}
                        }
                        if *true_target_position >= len {
                            errors.push(VerifyError::JumpOutOfBounds {
                                instruction_pointer,
                                target: *true_target_position,
                            });
                        }
                        if *false_target_position >= len {
                            errors.push(VerifyError::JumpOutOfBounds {
                                instruction_pointer,
                                target: *false_target_position,
                            });
                        }
                    }
                    Instruction::ReturnDefinedCall {
                        function_identifier,
                        outputs,
                    } => {
                        self.verify_return_defined_call(
                            instruction_pointer,
                            function_identifier,
                            outputs,
                            &mut errors,
                            &mut types,
                        );
                    }
                }
            }
        }

        errors
    }

    fn verify_bind<'b>(
        instruction_pointer: usize,
        slot: Slot,
        primitive_type: PrimitiveType,
        value: &Literal,
        errors: &mut Vec<VerifyError<'b>>,
        types: &mut Vec<ConcreteType<'b>>,
    ) {
        if !literal_matches_type(value, &primitive_type) {
            errors.push(VerifyError::InvalidLiteral {
                instruction_pointer,
                slot,
            });
        }
        bind_slot(
            types,
            errors,
            instruction_pointer,
            slot,
            ConcreteType::Primitive(primitive_type),
        );
    }

    fn verify_call<'b>(
        instruction_pointer: usize,
        function_name: Functions,
        generic_arguments: &Box<[ConcreteType]>,
        inputs: &Vec<Slot>,
        output: Slot,
        errors: &mut Vec<VerifyError<'b>>,
        types: &mut Vec<ConcreteType<'b>>,
    ) {
        let sig = &FUNCTION_REGISTRY.functions[function_name as usize];

        // argument count
        if inputs.len() != sig.inputs.len() {
            errors.push(VerifyError::ArgumentCountMismatch {
                instruction_pointer,
                expected: sig.inputs.len(),
                found: inputs.len(),
            });
        } else {
            // argument types
            for (arg_slot, expected_type) in inputs.iter().zip(sig.inputs.iter()) {
                match types.get(*arg_slot as usize) {
                    None => errors.push(VerifyError::UnboundSlot {
                        instruction_pointer,
                        slot: *arg_slot,
                    }),
                    Some(found_type) if !type_compatible(found_type, expected_type) => {
                        errors.push(VerifyError::TypeMismatch {
                            instruction_pointer,
                            slot: *arg_slot,
                            expected: expected_type.clone(),
                            found: found_type.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        bind_slot(types, errors, instruction_pointer, output, sig.output.clone());
    }

    fn verify_special_call<'b>(
        instruction_pointer: usize,
        function_name: SpecialFunctions,
        generic_arguments: &Box<[ConcreteType]>,
        inputs: &Vec<Slot>,
        output: Slot,
        errors: &mut Vec<VerifyError<'b>>,
        types: &mut Vec<ConcreteType<'b>>,
    ) {
        let sig = &SPECIAL_FUNCTIONS_REGISTRY.functions[function_name as usize];

        // argument count
        if inputs.len() != sig.inputs.len() {
            errors.push(VerifyError::ArgumentCountMismatch {
                instruction_pointer,
                expected: sig.inputs.len(),
                found: inputs.len(),
            });
        } else {
            // argument types
            for (arg_slot, expected_type) in inputs.iter().zip(sig.inputs.iter()) {
                match types.get(*arg_slot as usize) {
                    None => errors.push(VerifyError::UnboundSlot {
                        instruction_pointer,
                        slot: *arg_slot,
                    }),
                    Some(found_type) if !type_compatible(found_type, expected_type) => {
                        errors.push(VerifyError::TypeMismatch {
                            instruction_pointer,
                            slot: *arg_slot,
                            expected: expected_type.clone(),
                            found: found_type.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        bind_slot(types, errors, instruction_pointer, output, sig.output.clone());
    }

    fn verify_defined_call<'b>(
        &self,
        instruction_pointer: usize,
        function_identifier: &FunctionIdentifier,
        generic_arguments: &Box<[ConcreteType]>,
        inputs: &Vec<Slot>,
        outputs: &Vec<Slot>,
        errors: &mut Vec<VerifyError<'b>>,
        types: &mut Vec<ConcreteType<'b>>,
    ) {
        match self.defined_functions.get(function_identifier) {
            None => {
                errors.push(VerifyError::CanNotFoundDefinedFunction {
                    instruction_pointer,
                    function_identifier: *function_identifier,
                });
            }
            Some(sig) => {
                // ── inputs: slots read by the callee ─────────────
                if inputs.len() != sig.inputs.len() {
                    errors.push(VerifyError::DefinedFunctionArgumentCountMismatch {
                        instruction_pointer,
                        expected: sig.inputs.len(),
                        found: inputs.len(),
                    });
                } else {
                    for (slot, expected_type) in inputs.iter().zip(sig.inputs.iter()) {
                        match types.get(*slot as usize) {
                            None => errors.push(VerifyError::UnboundSlot {
                                instruction_pointer,
                                slot: *slot,
                            }),
                            Some(found_type) if !type_compatible(found_type, expected_type) => {
                                errors.push(VerifyError::TypeMismatch {
                                    instruction_pointer,
                                    slot: *slot,
                                    expected: expected_type.clone(),
                                    found: found_type.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }

                // ── destination_slots: caller slots written with output values ──
                // Count must match outputs; each slot is bound to the output type.
                if outputs.len() != sig.outputs.len() {
                    errors.push(VerifyError::DefinedFunctionReturnCountMismatch {
                        instruction_pointer,
                        expected: sig.outputs.len(),
                        found: outputs.len(),
                    });
                } else {
                    for (slot, output_type) in outputs.iter().zip(sig.outputs.iter()) {
                        bind_slot(
                            types,
                            errors,
                            instruction_pointer,
                            *slot,
                            output_type.clone(),
                        );
                    }
                }
            }
        }
    }

    fn verify_return_defined_call<'b>(
        &self,
        instruction_pointer: usize,
        function_identifier: &FunctionIdentifier,
        outputs: &Vec<Slot>,
        errors: &mut Vec<VerifyError<'b>>,
        types: &mut Vec<ConcreteType<'b>>,
    ) {
        match self.defined_functions.get(function_identifier) {
            None => {
                errors.push(VerifyError::CanNotFoundDefinedFunction {
                    instruction_pointer,
                    function_identifier: *function_identifier,
                });
            }
            Some(sig) => {
                if outputs.len() != sig.outputs.len() {
                    errors.push(VerifyError::DefinedFunctionReturnCountMismatch {
                        instruction_pointer,
                        expected: sig.outputs.len(),
                        found: outputs.len(),
                    });
                } else {
                    for (slot, output_type) in outputs.iter().zip(sig.outputs.iter()) {
                        bind_slot(
                            types,
                            errors,
                            instruction_pointer,
                            *slot,
                            output_type.clone(),
                        );
                    }
                }
            }
        }
    }
}

/// Write a type to a slot for the first time, or validate that a subsequent
/// write uses the same type as the originally established one.
///
/// The first write wins: it sets the canonical type for the slot.
/// Any later write that would change the type is recorded as `TypeMismatch`.
fn bind_slot<'a>(
    slots: &mut Vec<ConcreteType<'a>>,
    errors: &mut Vec<VerifyError<'a>>,
    instruction_pointer: usize,
    slot: Slot,
    attempted: ConcreteType<'a>,
) {
    match slots.get(slot as usize) {
        None => {
            slots.insert(slot as usize, attempted);
        }
        Some(original) if *original != attempted => {
            errors.push(VerifyError::TypeMismatch {
                instruction_pointer,
                slot,
                expected: original.clone(),
                found: attempted,
            });
        }
        _ => {} // same type — no-op
    }
}

/// Returns true when the literal variant matches the declared type.
fn literal_matches_type(literal: &Literal, ty: &PrimitiveType) -> bool {
    matches!(
        (literal, ty),
        (Literal::Integer(_), PrimitiveType::Integer)
            | (Literal::Float(_), PrimitiveType::Float)
            | (Literal::Boolean(_), PrimitiveType::Boolean)
            | (Literal::Char(_), PrimitiveType::Char)
            | (Literal::String(_), PrimitiveType::String)
    )
}

fn type_compatible(found: &ConcreteType, expected: &TypeExpression) -> bool {
    match (found, expected) {
        (_, TypeExpression::Generic(_)) => true,
        (ConcreteType::Primitive(found), TypeExpression::Primitive(expected)) => found == expected,
        (ConcreteType::Vector(found), TypeExpression::Vector(expected)) => type_compatible(found, expected),
        (ConcreteType::Option(found), TypeExpression::Option(expected)) => type_compatible(found, expected),
        (
            ConcreteType::Result {
                ok: found_ok,
                err: found_err,
            },
            TypeExpression::Result {
                ok: expected_ok,
                err: expected_err,
            },
        ) => type_compatible(found_ok, expected_ok) && type_compatible(found_err, expected_err),
        _ => false,
    }
}

use std::sync::Arc;

use crate::runtime_task::{
    instruction::{
        ConcreteInstruction, FunctionIdentifier, Instruction, Literal,
        SPECIAL_FUNCTIONS_REGISTRY, FUNCTION_REGISTRY, Slot,
    },
    types::{ConcreteType, PrimitiveType, TypeExpression},
    verifier::virtual_machine_instruction_metadata::ConcreteFunctionMetadata,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VerifyError<'a> {
    UnboundSlot { instruction_pointer: usize, slot: Slot },
    SlotOutOfBounds { instruction_pointer: usize, slot: Slot, slot_count: usize },
    TypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: ConcreteType<'a>,
        found: ConcreteType<'a>,
    },
    SignatureTypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: TypeExpression<'static>,
        found: ConcreteType<'a>,
    },
    InvalidLiteral { instruction_pointer: usize, slot: Slot },
    JumpOutOfBounds { instruction_pointer: usize, target: usize },
    ArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    GenericArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    CanNotFoundDefinedFunction { instruction_pointer: usize, function_identifier: FunctionIdentifier },
    DefinedFunctionArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    DefinedFunctionReturnCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
}

pub struct InstructionVerifier<'a> {
    functions: Arc<[(ConcreteFunctionMetadata<'a>, Arc<[ConcreteInstruction<'a>]>) ]>,
}

impl<'a> InstructionVerifier<'a> {
    pub fn new(functions: Arc<[(ConcreteFunctionMetadata<'a>, Arc<[ConcreteInstruction<'a>]>) ]>) -> Self {
        Self { functions }
    }

    /// Checks every function body against its concrete metadata and returns all errors.
    pub fn verify(&self) -> Vec<VerifyError<'a>> {
        let mut errors = Vec::new();
        for (metadata, instructions) in self.functions.iter() {
            self.verify_function(metadata, instructions, &mut errors);
        }
        errors
    }

    fn verify_function(
        &self,
        metadata: &ConcreteFunctionMetadata<'a>,
        instructions: &[ConcreteInstruction<'a>],
        errors: &mut Vec<VerifyError<'a>>,
    ) {
        let mut initialized = vec![false; metadata.slot_types.len()];
        for i in 0..metadata.input_types.len() {
            match (metadata.slot_types.get(i), initialized.get_mut(i)) {
                (Some(slot_type), Some(bound)) => {
                    *bound = true;
                    if slot_type != &metadata.input_types[i] {
                        errors.push(VerifyError::TypeMismatch {
                            instruction_pointer: 0,
                            slot: i as Slot,
                            expected: metadata.input_types[i].clone(),
                            found: slot_type.clone(),
                        });
                    }
                }
                _ => errors.push(VerifyError::SlotOutOfBounds {
                    instruction_pointer: 0,
                    slot: i as Slot,
                    slot_count: metadata.slot_types.len(),
                }),
            }
        }

        for (ip, instruction) in instructions.iter().enumerate() {
            match instruction {
                Instruction::Bind { slot, type_name, value } => {
                    if !literal_matches_type(value, type_name) {
                        errors.push(VerifyError::InvalidLiteral { instruction_pointer: ip, slot: *slot });
                    }
                    self.write_slot(metadata, &mut initialized, ip, *slot, &ConcreteType::Primitive(*type_name), errors);
                }
                Instruction::Call { function_name, generic_arguments, inputs, output } => {
                    let sig = &FUNCTION_REGISTRY.functions[*function_name as usize];
                    self.verify_generic_count(ip, sig.generics.len(), generic_arguments.len(), errors);
                    self.verify_inputs(metadata, &initialized, ip, inputs, sig.inputs, sig.generics, generic_arguments, errors);
                    if let Some(out_ty) = metadata.slot_types.get(*output as usize) {
                        if !type_compatible(out_ty, &sig.output, sig.generics, generic_arguments) {
                            errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: ip, slot: *output, expected: sig.output.clone(), found: out_ty.clone() });
                        }
                        self.write_slot(metadata, &mut initialized, ip, *output, out_ty, errors);
                    } else { self.bad_slot(metadata, ip, *output, errors); }
                }
                Instruction::SpecialCall { function_name, generic_arguments, inputs, output } => {
                    let sig = &SPECIAL_FUNCTIONS_REGISTRY.functions[*function_name as usize];
                    self.verify_generic_count(ip, sig.generics.len(), generic_arguments.len(), errors);
                    self.verify_inputs(metadata, &initialized, ip, inputs, sig.inputs, sig.generics, generic_arguments, errors);
                    if let Some(out_ty) = metadata.slot_types.get(*output as usize) {
                        if !type_compatible(out_ty, &sig.output, sig.generics, generic_arguments) {
                            errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: ip, slot: *output, expected: sig.output.clone(), found: out_ty.clone() });
                        }
                        self.write_slot(metadata, &mut initialized, ip, *output, out_ty, errors);
                    } else { self.bad_slot(metadata, ip, *output, errors); }
                }
                Instruction::DefinedCall { function_identifier, generic_arguments: _, inputs, outputs } => {
                    match self.find_function(*function_identifier) {
                        None => errors.push(VerifyError::CanNotFoundDefinedFunction { instruction_pointer: ip, function_identifier: *function_identifier }),
                        Some((callee, _)) => {
                            if inputs.len() != callee.input_types.len() {
                                errors.push(VerifyError::DefinedFunctionArgumentCountMismatch { instruction_pointer: ip, expected: callee.input_types.len(), found: inputs.len() });
                            } else {
                                for (slot, expected) in inputs.iter().zip(&callee.input_types) {
                                    self.read_slot(metadata, &initialized, ip, *slot, expected, errors);
                                }
                            }
                            if outputs.len() != callee.output_types.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch { instruction_pointer: ip, expected: callee.output_types.len(), found: outputs.len() });
                            } else {
                                for (slot, expected) in outputs.iter().zip(&callee.output_types) {
                                    self.write_slot(metadata, &mut initialized, ip, *slot, expected, errors);
                                }
                            }
                        }
                    }
                }
                Instruction::Jump { target_position } => {
                    if *target_position >= instructions.len() { errors.push(VerifyError::JumpOutOfBounds { instruction_pointer: ip, target: *target_position }); }
                }
                Instruction::ConditionalJump { condition, true_target_position, false_target_position } => {
                    self.read_slot(metadata, &initialized, ip, *condition, &ConcreteType::Primitive(PrimitiveType::Boolean), errors);
                    for target in [true_target_position, false_target_position] {
                        if *target >= instructions.len() { errors.push(VerifyError::JumpOutOfBounds { instruction_pointer: ip, target: *target }); }
                    }
                }
                Instruction::ReturnDefinedCall { function_identifier, outputs } => {
                    match self.find_function(*function_identifier) {
                        None => errors.push(VerifyError::CanNotFoundDefinedFunction { instruction_pointer: ip, function_identifier: *function_identifier }),
                        Some((callee, _)) => {
                            if outputs.len() != callee.output_types.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch { instruction_pointer: ip, expected: callee.output_types.len(), found: outputs.len() });
                            } else {
                                for (slot, expected) in outputs.iter().zip(&callee.output_types) {
                                    self.read_slot(metadata, &initialized, ip, *slot, expected, errors);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn find_function(&self, id: FunctionIdentifier) -> Option<(&ConcreteFunctionMetadata<'a>, &[ConcreteInstruction<'a>])> {
        self.functions.iter().find(|(metadata, _)| metadata.function_identifier == id)
            .map(|(metadata, body)| (metadata, body.as_ref()))
    }

    fn verify_generic_count(&self, ip: usize, expected: usize, found: usize, errors: &mut Vec<VerifyError<'a>>) {
        if expected != found { errors.push(VerifyError::GenericArgumentCountMismatch { instruction_pointer: ip, expected, found }); }
    }

    fn verify_inputs(
        &self, metadata: &ConcreteFunctionMetadata<'a>, initialized: &[bool], ip: usize,
        inputs: &[Slot], expected: &[TypeExpression<'static>],
        generics: &[crate::runtime_task::types::GenericParameter], arguments: &[ConcreteType<'a>],
        errors: &mut Vec<VerifyError<'a>>,
    ) {
        if inputs.len() != expected.len() {
            errors.push(VerifyError::ArgumentCountMismatch { instruction_pointer: ip, expected: expected.len(), found: inputs.len() });
            return;
        }
        for (slot, expected) in inputs.iter().zip(expected) {
            let Some(found) = metadata.slot_types.get(*slot as usize) else { self.bad_slot(metadata, ip, *slot, errors); continue; };
            if !initialized.get(*slot as usize).copied().unwrap_or(false) {
                errors.push(VerifyError::UnboundSlot { instruction_pointer: ip, slot: *slot });
            } else if !type_compatible(found, expected, generics, arguments) {
                errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: ip, slot: *slot, expected: expected.clone(), found: found.clone() });
            }
        }
    }

    fn read_slot(&self, metadata: &ConcreteFunctionMetadata<'a>, initialized: &[bool], ip: usize, slot: Slot, expected: &ConcreteType<'a>, errors: &mut Vec<VerifyError<'a>>) {
        let Some(found) = metadata.slot_types.get(slot as usize) else { self.bad_slot(metadata, ip, slot, errors); return; };
        if !initialized.get(slot as usize).copied().unwrap_or(false) {
            errors.push(VerifyError::UnboundSlot { instruction_pointer: ip, slot });
        } else if found != expected {
            errors.push(VerifyError::TypeMismatch { instruction_pointer: ip, slot, expected: expected.clone(), found: found.clone() });
        }
    }

    fn write_slot(&self, metadata: &ConcreteFunctionMetadata<'a>, initialized: &mut [bool], ip: usize, slot: Slot, attempted: &ConcreteType<'a>, errors: &mut Vec<VerifyError<'a>>) {
        let Some(expected) = metadata.slot_types.get(slot as usize) else { self.bad_slot(metadata, ip, slot, errors); return; };
        if expected != attempted {
            errors.push(VerifyError::TypeMismatch { instruction_pointer: ip, slot, expected: expected.clone(), found: attempted.clone() });
        }
        if let Some(bound) = initialized.get_mut(slot as usize) { *bound = true; }
    }

    fn bad_slot(&self, metadata: &ConcreteFunctionMetadata<'a>, ip: usize, slot: Slot, errors: &mut Vec<VerifyError<'a>>) {
        errors.push(VerifyError::SlotOutOfBounds { instruction_pointer: ip, slot, slot_count: metadata.slot_types.len() });
    }
}

fn literal_matches_type(literal: &Literal, ty: &PrimitiveType) -> bool {
    matches!((literal, ty),
        (Literal::Integer(_), PrimitiveType::Integer) | (Literal::Float(_), PrimitiveType::Float) |
        (Literal::Boolean(_), PrimitiveType::Boolean) | (Literal::Char(_), PrimitiveType::Char) |
        (Literal::String(_), PrimitiveType::String))
}

fn type_compatible(
    found: &ConcreteType<'_>, expected: &TypeExpression<'_>,
    generics: &[crate::runtime_task::types::GenericParameter], arguments: &[ConcreteType<'_>],
) -> bool {
    match expected {
        TypeExpression::Generic(id) => generics.iter().position(|generic| generic.id == *id)
            .and_then(|index| arguments.get(index)).is_some_and(|argument| found == argument),
        TypeExpression::Primitive(expected) => matches!(found, ConcreteType::Primitive(found) if found == expected),
        TypeExpression::Vector(expected) => matches!(found, ConcreteType::Vector(found) if type_compatible(found, expected, generics, arguments)),
        TypeExpression::Option(expected) => matches!(found, ConcreteType::Option(found) if type_compatible(found, expected, generics, arguments)),
        TypeExpression::Result { ok, err } => matches!(found, ConcreteType::Result { ok: found_ok, err: found_err } if type_compatible(found_ok, ok, generics, arguments) && type_compatible(found_err, err, generics, arguments)),
    }
}

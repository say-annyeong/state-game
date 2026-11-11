use std::sync::Arc;

use crate::runtime_task::{
    instruction::{
        ConcreteInstruction, FunctionIdentifier, FunctionSignature, Instruction, Literal,
        Slot,
    },
    types::{
        ConcreteType, ConcreteTypeInterner, GenericTypeInterner, PrimitiveType,
        TypeExpression,
    },
    verifier::virtual_machine_instruction_metadata::ConcreteFunctionMetadata,
    instruction::{FUNCTION_REGISTRY, SPECIAL_FUNCTIONS_REGISTRY}
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VerifyError {
    UnboundSlot { instruction_pointer: usize, slot: Slot },
    SlotOutOfBounds { instruction_pointer: usize, slot: Slot, slot_count: usize },
    TypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: ConcreteType,
        found: ConcreteType,
    },
    SignatureTypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: TypeExpression,
        found: ConcreteType,
    },
    InvalidLiteral { instruction_pointer: usize, slot: Slot },
    JumpOutOfBounds { instruction_pointer: usize, target: usize },
    ArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    GenericArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    CanNotFoundDefinedFunction { instruction_pointer: usize, function_identifier: FunctionIdentifier },
    DefinedFunctionArgumentCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
    DefinedFunctionReturnCountMismatch { instruction_pointer: usize, expected: usize, found: usize },
}

pub struct InstructionVerifier {
    functions: Arc<[(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)]>,
    /// Shared interner used to resolve nested TypeExpression IDs.
    generic_interner: GenericTypeInterner,
    concrete_interner: ConcreteTypeInterner,
}

impl InstructionVerifier {
    pub fn new(
        functions: Arc<[(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)]>,
        generic_interner: GenericTypeInterner,
        concrete_interner: ConcreteTypeInterner,
    ) -> Self {
        Self { functions, generic_interner, concrete_interner }
    }

    /// Checks every function body against its concrete metadata and returns all errors.
    pub fn verify(&self) -> Vec<VerifyError> {
        let mut errors = Vec::new();
        for (metadata, instructions) in self.functions.iter() {
            self.verify_function(metadata, instructions, &mut errors);
        }
        errors
    }

    fn verify_function(
        &self,
        metadata: &ConcreteFunctionMetadata,
        instructions: &[ConcreteInstruction],
        errors: &mut Vec<VerifyError>,
    ) {
        let mut initialized = vec![false; metadata.slot_types.len()];

        // Seed input slots as initialized.
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

        for (instruction_pointer, instruction) in instructions.iter().enumerate() {
            match instruction {
                Instruction::Bind { slot, type_name, value } => {
                    if !literal_matches_type(value, type_name) {
                        errors.push(VerifyError::InvalidLiteral { instruction_pointer: instruction_pointer, slot: *slot });
                    }
                    self.write_slot(metadata, &mut initialized, instruction_pointer, *slot, &ConcreteType::Primitive(*type_name), errors);
                }
                Instruction::Call { function_name, generic_arguments, inputs, output } => {
                    let mut temporary_interner = self.generic_interner.clone();
                    let signature = FUNCTION_REGISTRY.build(*function_name as usize, &mut temporary_interner)
                        .unwrap_or_else(|| FunctionSignature { generics: vec![], inputs: vec![], output: TypeExpression::Primitive(PrimitiveType::Unit) });
                    self.verify_generic_count(instruction_pointer, signature.generics.len(), generic_arguments.len(), errors);
                    self.verify_inputs(metadata, &initialized, instruction_pointer, inputs, &signature.inputs, &signature.generics, generic_arguments, errors);
                    if let Some(out_ty) = metadata.slot_types.get(*output as usize) {
                        if !self.type_compatible(out_ty, &signature.output, &signature.generics, generic_arguments) {
                            errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: instruction_pointer, slot: *output, expected: signature.output.clone(), found: out_ty.clone() });
                        }
                        self.write_slot(metadata, &mut initialized, instruction_pointer, *output, out_ty, errors);
                    } else {
                        self.bad_slot(metadata, instruction_pointer, *output, errors);
                    }
                }
                Instruction::SpecialCall { function_name, generic_arguments, inputs, output } => {
                    let mut temporary_interner = self.generic_interner.clone();
                    let signature = SPECIAL_FUNCTIONS_REGISTRY.build(*function_name as usize, &mut temporary_interner)
                        .unwrap_or_else(|| FunctionSignature { generics: vec![], inputs: vec![], output: TypeExpression::Primitive(PrimitiveType::Unit) });
                    self.verify_generic_count(instruction_pointer, signature.generics.len(), generic_arguments.len(), errors);
                    self.verify_inputs(metadata, &initialized, instruction_pointer, inputs, &signature.inputs, &signature.generics, generic_arguments, errors);
                    if let Some(out_ty) = metadata.slot_types.get(*output as usize) {
                        if !self.type_compatible(out_ty, &signature.output, &signature.generics, generic_arguments) {
                            errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: instruction_pointer, slot: *output, expected: signature.output.clone(), found: out_ty.clone() });
                        }
                        self.write_slot(metadata, &mut initialized, instruction_pointer, *output, out_ty, errors);
                    } else {
                        self.bad_slot(metadata, instruction_pointer, *output, errors);
                    }
                }
                Instruction::DefinedCall { function_identifier, generic_arguments: _, inputs, outputs } => {
                    match self.find_function(*function_identifier) {
                        None => errors.push(VerifyError::CanNotFoundDefinedFunction { instruction_pointer: instruction_pointer, function_identifier: *function_identifier }),
                        Some((callee, _)) => {
                            if inputs.len() != callee.input_types.len() {
                                errors.push(VerifyError::DefinedFunctionArgumentCountMismatch { instruction_pointer: instruction_pointer, expected: callee.input_types.len(), found: inputs.len() });
                            } else {
                                for (slot, expected) in inputs.iter().zip(&callee.input_types) {
                                    self.read_slot(metadata, &initialized, instruction_pointer, *slot, expected, errors);
                                }
                            }
                            if outputs.len() != callee.output_types.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch { instruction_pointer: instruction_pointer, expected: callee.output_types.len(), found: outputs.len() });
                            } else {
                                for (slot, expected) in outputs.iter().zip(&callee.output_types) {
                                    self.write_slot(metadata, &mut initialized, instruction_pointer, *slot, expected, errors);
                                }
                            }
                        }
                    }
                }
                Instruction::Jump { target_position } => {
                    if *target_position >= instructions.len() {
                        errors.push(VerifyError::JumpOutOfBounds { instruction_pointer: instruction_pointer, target: *target_position });
                    }
                }
                Instruction::ConditionalJump { condition, true_target_position, false_target_position } => {
                    self.read_slot(metadata, &initialized, instruction_pointer, *condition, &ConcreteType::Primitive(PrimitiveType::Boolean), errors);
                    for target in [true_target_position, false_target_position] {
                        if *target >= instructions.len() {
                            errors.push(VerifyError::JumpOutOfBounds { instruction_pointer: instruction_pointer, target: *target });
                        }
                    }
                }
                Instruction::ReturnDefinedCall { function_identifier, outputs } => {
                    match self.find_function(*function_identifier) {
                        None => errors.push(VerifyError::CanNotFoundDefinedFunction { instruction_pointer: instruction_pointer, function_identifier: *function_identifier }),
                        Some((callee, _)) => {
                            if outputs.len() != callee.output_types.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch { instruction_pointer: instruction_pointer, expected: callee.output_types.len(), found: outputs.len() });
                            } else {
                                for (slot, expected) in outputs.iter().zip(&callee.output_types) {
                                    self.read_slot(metadata, &initialized, instruction_pointer, *slot, expected, errors);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn find_function(&self, id: FunctionIdentifier) -> Option<(&ConcreteFunctionMetadata, &[ConcreteInstruction])> {
        self.functions.iter()
            .find(|(metadata, _)| metadata.function_identifier == id)
            .map(|(metadata, body)| (metadata, body.as_ref()))
    }

    fn verify_generic_count(&self, instruction_pointer: usize, expected: usize, found: usize, errors: &mut Vec<VerifyError>) {
        if expected != found {
            errors.push(VerifyError::GenericArgumentCountMismatch { instruction_pointer: instruction_pointer, expected, found });
        }
    }

    fn verify_inputs(
        &self,
        metadata: &ConcreteFunctionMetadata,
        initialized: &[bool],
        instruction_pointer: usize,
        inputs: &[Slot],
        expected: &[TypeExpression],
        generics: &[crate::runtime_task::types::GenericParameter],
        arguments: &[ConcreteType],
        errors: &mut Vec<VerifyError>,
    ) {
        if inputs.len() != expected.len() {
            errors.push(VerifyError::ArgumentCountMismatch { instruction_pointer: instruction_pointer, expected: expected.len(), found: inputs.len() });
            return;
        }
        for (slot, expected_ty) in inputs.iter().zip(expected) {
            let Some(found) = metadata.slot_types.get(*slot as usize) else {
                self.bad_slot(metadata, instruction_pointer, *slot, errors);
                continue;
            };
            if !initialized.get(*slot as usize).copied().unwrap_or(false) {
                errors.push(VerifyError::UnboundSlot { instruction_pointer: instruction_pointer, slot: *slot });
            } else if !self.type_compatible(found, expected_ty, generics, arguments) {
                errors.push(VerifyError::SignatureTypeMismatch { instruction_pointer: instruction_pointer, slot: *slot, expected: expected_ty.clone(), found: found.clone() });
            }
        }
    }

    fn read_slot(
        &self,
        metadata: &ConcreteFunctionMetadata,
        initialized: &[bool],
        instruction_pointer: usize,
        slot: Slot,
        expected: &ConcreteType,
        errors: &mut Vec<VerifyError>,
    ) {
        let Some(found) = metadata.slot_types.get(slot as usize) else {
            self.bad_slot(metadata, instruction_pointer, slot, errors);
            return;
        };
        if !initialized.get(slot as usize).copied().unwrap_or(false) {
            errors.push(VerifyError::UnboundSlot { instruction_pointer: instruction_pointer, slot });
        } else if found != expected {
            errors.push(VerifyError::TypeMismatch { instruction_pointer: instruction_pointer, slot, expected: expected.clone(), found: found.clone() });
        }
    }

    fn write_slot(
        &self,
        metadata: &ConcreteFunctionMetadata,
        initialized: &mut [bool],
        instruction_pointer: usize,
        slot: Slot,
        attempted: &ConcreteType,
        errors: &mut Vec<VerifyError>,
    ) {
        let Some(expected) = metadata.slot_types.get(slot as usize) else {
            self.bad_slot(metadata, instruction_pointer, slot, errors);
            return;
        };
        if expected != attempted {
            errors.push(VerifyError::TypeMismatch { instruction_pointer: instruction_pointer, slot, expected: expected.clone(), found: attempted.clone() });
        }
        if let Some(bound) = initialized.get_mut(slot as usize) {
            *bound = true;
        }
    }

    fn bad_slot(&self, metadata: &ConcreteFunctionMetadata, instruction_pointer: usize, slot: Slot, errors: &mut Vec<VerifyError>) {
        errors.push(VerifyError::SlotOutOfBounds { instruction_pointer: instruction_pointer, slot, slot_count: metadata.slot_types.len() });
    }

    /// Check if a `ConcreteType` matches a `TypeExpression` given generic bindings.
    ///
    /// `generics` — the formal generic parameters of the signature.
    /// `arguments` — the concrete types supplied for each generic, positionally.
    fn type_compatible(
        &self,
        found: &ConcreteType,
        expected: &TypeExpression,
        generics: &[crate::runtime_task::types::GenericParameter],
        arguments: &[ConcreteType],
    ) -> bool {
        match expected {
            TypeExpression::Generic(id) => {
                generics.iter()
                    .position(|generic_parameter| &generic_parameter.id == id)
                    .and_then(|position| arguments.get(position))
                    .is_some_and(|argument| found == argument)
            }
            TypeExpression::Primitive(p) => {
                matches!(found, ConcreteType::Primitive(f) if f == p)
            }
            TypeExpression::Vector(inner_gid) => {
                let ConcreteType::Vector(inner_ngid) = found else { return false; };
                let Some(inner_ct) = self.concrete_interner.get(*inner_ngid) else { return false; };
                let Some(inner_te) = self.generic_interner.get(*inner_gid) else { return false; };
                self.type_compatible(inner_ct, inner_te, generics, arguments)
            }
            TypeExpression::Option(inner_gid) => {
                let ConcreteType::Option(inner_ngid) = found else { return false; };
                let Some(inner_ct) = self.concrete_interner.get(*inner_ngid) else { return false; };
                let Some(inner_te) = self.generic_interner.get(*inner_gid) else { return false; };
                self.type_compatible(inner_ct, inner_te, generics, arguments)
            }
            TypeExpression::Result { ok: ok_gid, err: err_gid } => {
                let ConcreteType::Result { ok: ok_ngid, err: err_ngid } = found else { return false; };
                let Some(ok_ct) = self.concrete_interner.get(*ok_ngid) else { return false; };
                let Some(err_ct) = self.concrete_interner.get(*err_ngid) else { return false; };
                let Some(ok_te) = self.generic_interner.get(*ok_gid) else { return false; };
                let Some(err_te) = self.generic_interner.get(*err_gid) else { return false; };
                self.type_compatible(ok_ct, ok_te, generics, arguments)
                    && self.type_compatible(err_ct, err_te, generics, arguments)
            }
        }
    }
}

fn literal_matches_type(literal: &Literal, primitive_type: &PrimitiveType) -> bool {
    matches!((literal, primitive_type),
        (Literal::Integer(_), PrimitiveType::Integer) |
        (Literal::Float(_),   PrimitiveType::Float)   |
        (Literal::Boolean(_), PrimitiveType::Boolean) |
        (Literal::Char(_),    PrimitiveType::Char)    |
        (Literal::String(_),  PrimitiveType::String))
}

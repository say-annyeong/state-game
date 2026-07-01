use crate::virtual_machine::instruction::{DefinedFunctionSignature, FunctionIdentifier, FunctionRegistry, FunctionSignature, Functions, Instruction, Literal, Slot, SpecialFunctions, FUNCTION_REGISTRY, SPECIAL_FUNCTIONS_REGISTRY};
use crate::virtual_machine::types::Type;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VerifyError {
    /// A slot was read before it was assigned.
    UnboundSlot {
        instruction_pointer: usize,
        slot: Slot,
    },
    /// The type of a slot did not match what was expected.
    TypeMismatch {
        instruction_pointer: usize,
        slot: Slot,
        expected: Type,
        found: Type,
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

pub struct InstructionVerifier {
    instruction: Arc<[Instruction]>,
    defined_functions: Arc<HashMap<FunctionIdentifier, DefinedFunctionSignature>>,
}

impl InstructionVerifier {
    pub fn new(
        instruction: Arc<[Instruction]>,
        defined_functions: Arc<HashMap<FunctionIdentifier, DefinedFunctionSignature>>,
    ) -> Self {
        Self {
            instruction,
            defined_functions,
        }
    }

    /// Verifies the instruction stream and returns all errors found.
    /// An empty Vec means the program is well-typed.
    pub fn verify(&self) -> Vec<VerifyError> {
        let instructions = &self.instruction;
        let len = instructions.len();
        let mut errors = Vec::new();
        // slot → type assigned so far (forward pass)
        let mut slots: HashMap<Slot, Type> = HashMap::new();

        for (instruction_pointer, instruction) in instructions.iter().enumerate() {
            match instruction {
                // ── Bind ────────────────────────────────────────────────────
                Instruction::Bind {
                    slot,
                    type_name,
                    value,
                } => {
                    if !literal_matches_type(value, type_name) {
                        errors.push(VerifyError::InvalidLiteral {
                            instruction_pointer,
                            slot: *slot,
                        });
                    }
                    bind_slot(
                        &mut slots,
                        &mut errors,
                        instruction_pointer,
                        *slot,
                        type_name.clone(),
                    );
                }

                // ── Call ────────────────────────────────────────────────────
                Instruction::Call {
                    function_name,
                    inputs,
                    output,
                } => {
                    let sig = &FUNCTION_REGISTRY.functions[*function_name as usize];

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
                            match slots.get(arg_slot) {
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

                    bind_slot(
                        &mut slots,
                        &mut errors,
                        instruction_pointer,
                        *output,
                        sig.output.clone(),
                    );
                }

                // ── Jump ────────────────────────────────────────────────────
                Instruction::Jump { target_position } => {
                    if *target_position >= len {
                        errors.push(VerifyError::JumpOutOfBounds {
                            instruction_pointer,
                            target: *target_position,
                        });
                    }
                }

                // ── ConditionalJump ─────────────────────────────────────────
                Instruction::ConditionalJump {
                    condition,
                    true_target_position,
                    false_target_position,
                } => {
                    match slots.get(condition) {
                        None => errors.push(VerifyError::UnboundSlot {
                            instruction_pointer,
                            slot: *condition,
                        }),
                        Some(t) if *t != Type::Boolean => {
                            errors.push(VerifyError::TypeMismatch {
                                instruction_pointer,
                                slot: *condition,
                                expected: Type::Boolean,
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

                Instruction::SpecialCall {
                    function_name,
                    inputs,
                    output,
                } => {
                    let sig = &SPECIAL_FUNCTIONS_REGISTRY.functions[*function_name as usize];

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
                            match slots.get(arg_slot) {
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

                    bind_slot(
                        &mut slots,
                        &mut errors,
                        instruction_pointer,
                        *output,
                        sig.output.clone(),
                    );
                }

                // ── DefinedCall ─────────────────────────────────────────────
                Instruction::DefinedCall {
                    function_identifier,
                    inputs,
                    destination_slots,
                    source_slots,
                } => {
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
                                    match slots.get(slot) {
                                        None => errors.push(VerifyError::UnboundSlot {
                                            instruction_pointer,
                                            slot: *slot,
                                        }),
                                        Some(found_type)
                                            if !type_compatible(found_type, expected_type) =>
                                        {
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

                            // ── source_slots: output slots inside the callee ──
                            // Must be bound and type-match the declared output types.
                            if source_slots.len() != sig.destinations.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch {
                                    instruction_pointer,
                                    expected: sig.destinations.len(),
                                    found: source_slots.len(),
                                });
                            } else {
                                for (slot, expected_type) in
                                    source_slots.iter().zip(sig.destinations.iter())
                                {
                                    match slots.get(slot) {
                                        None => errors.push(VerifyError::UnboundSlot {
                                            instruction_pointer,
                                            slot: *slot,
                                        }),
                                        Some(found_type)
                                            if !type_compatible(found_type, expected_type) =>
                                        {
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
                            if destination_slots.len() != sig.destinations.len() {
                                errors.push(VerifyError::DefinedFunctionReturnCountMismatch {
                                    instruction_pointer,
                                    expected: sig.destinations.len(),
                                    found: destination_slots.len(),
                                });
                            } else {
                                for (slot, output_type) in
                                    destination_slots.iter().zip(sig.destinations.iter())
                                {
                                    bind_slot(
                                        &mut slots,
                                        &mut errors,
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
        }

        errors
    }
}

/// Write a type to a slot for the first time, or validate that a subsequent
/// write uses the same type as the originally established one.
///
/// The first write wins: it sets the canonical type for the slot.
/// Any later write that would change the type is recorded as `TypeMismatch`.
fn bind_slot(
    slots: &mut HashMap<Slot, Type>,
    errors: &mut Vec<VerifyError>,
    instruction_pointer: usize,
    slot: Slot,
    attempted: Type,
) {
    match slots.get(&slot) {
        None => {
            slots.insert(slot, attempted);
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
fn literal_matches_type(literal: &Literal, ty: &Type) -> bool {
    matches!(
        (literal, ty),
        (Literal::Integer(_), Type::Integer)
            | (Literal::Float(_), Type::Float)
            | (Literal::Boolean(_), Type::Boolean)
            | (Literal::Char(_), Type::Char)
            | (Literal::String(_), Type::String)
    )
}

/// Type compatibility check for function arguments.
///
/// `Type::Any` in an expected (registry) position accepts any concrete type:
///   - `Option<Any>`        accepts any `Option<T>`
///   - `Result<Any, Any>`   accepts any `Result<T, E>`
///   - `Result<T, Any>`     accepts `Result<T, E>` where ok-side matches T
///   - `Result<Any, E>`     accepts `Result<T, E>` where err-side matches E
///   - bare `Any`           accepts any type
fn type_compatible(found: &Type, expected: &Type) -> bool {
    match (found, expected) {
        // Bare Any wildcard
        (_, Type::Any) => true,
        // Option<Any> accepts any Option<_>
        (Type::Option(_), Type::Option(Type::Any)) => true,
        // Option<T>: recurse on inner type
        (Type::Option(f), Type::Option(e)) => type_compatible(f, e),
        // Result<Any, Any> accepts any Result<_, _>
        (Type::Result(_, _), Type::Result(Type::Any, Type::Any)) => true,
        // Result<T, Any>: ok side must match, err side is wildcard
        (Type::Result(fk, _), Type::Result(ek, Type::Any)) => type_compatible(fk, ek),
        // Result<Any, E>: err side must match, ok side is wildcard
        (Type::Result(_, fv), Type::Result(Type::Any, ev)) => type_compatible(fv, ev),
        // Result<T, E>: recurse on both sides
        (Type::Result(fk, fv), Type::Result(ek, ev)) => {
            type_compatible(fk, ek) && type_compatible(fv, ev)
        }
        // Vector<T>: recurse on inner type
        (Type::Vector(f), Type::Vector(e)) => type_compatible(f, e),
        // Everything else: exact equality
        _ => found == expected,
    }
}

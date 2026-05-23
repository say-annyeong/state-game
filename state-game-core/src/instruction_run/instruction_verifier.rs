use std::collections::HashMap;
use std::sync::Arc;
use crate::instruction_run::instruction::{FunctionRegistry, FunctionSignature, Functions, Instruction, Literal, Slot};
use crate::instruction_run::types::Type;

pub const FUNCTION_REGISTRY: FunctionRegistry<{ Functions::COUNT }> = FunctionRegistry {
    functions: [
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // AddInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // SubInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // MulInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // DivInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // ModInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Integer },                               // PowInteger
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Float },                                     // AddFloat
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Float },                                     // SubFloat
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Float },                                     // MulFloat
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Float },                                     // DivFloat
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Float },                                     // PowFloat
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Boolean },                               // EqualInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Boolean },                               // NotEqualInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Boolean },                               // GreaterThanInteger
        FunctionSignature { inputs: &[Type::Integer, Type::Integer], outputs: Type::Boolean },                               // LessThanInteger
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Boolean },                                   // GreaterThanFloat
        FunctionSignature { inputs: &[Type::Float, Type::Float], outputs: Type::Boolean },                                   // LessThanFloat
        FunctionSignature { inputs: &[Type::Boolean], outputs: Type::Boolean },                                              // Not
        FunctionSignature { inputs: &[Type::Boolean, Type::Boolean], outputs: Type::Boolean },                               // And
        FunctionSignature { inputs: &[Type::Boolean, Type::Boolean], outputs: Type::Boolean },                               // Or
        FunctionSignature { inputs: &[Type::Boolean, Type::Boolean], outputs: Type::Boolean },                               // Xor
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Boolean },                                 // EqualString
        FunctionSignature { inputs: &[Type::String], outputs: Type::Integer },                                               // StringLength
        FunctionSignature { inputs: &[Type::String, Type::Integer], outputs: Type::Char },                                   // StringGetChar
        FunctionSignature { inputs: &[Type::Vector(&Type::Integer), Type::Integer], outputs: Type::Integer },                // VectorGetInteger
        FunctionSignature { inputs: &[Type::Vector(&Type::Float), Type::Integer], outputs: Type::Float },                    // VectorGetFloat
        FunctionSignature { inputs: &[Type::Vector(&Type::String), Type::Integer], outputs: Type::String },                  // VectorGetString
        FunctionSignature { inputs: &[Type::Vector(&Type::Char), Type::Integer], outputs: Type::Char },                      // VectorGetChar
        FunctionSignature { inputs: &[Type::Vector(&Type::Boolean), Type::Integer], outputs: Type::Boolean },                // VectorGetBoolean
        FunctionSignature { inputs: &[Type::Integer], outputs: Type::Vector(&Type::Integer) },                               // VectorInitInteger
        FunctionSignature { inputs: &[Type::Float], outputs: Type::Vector(&Type::Float) },                                   // VectorInitFloat
        FunctionSignature { inputs: &[Type::String], outputs: Type::Vector(&Type::String) },                                 // VectorInitString
        FunctionSignature { inputs: &[Type::Char], outputs: Type::Vector(&Type::Char) },                                     // VectorInitChar
        FunctionSignature { inputs: &[Type::Boolean], outputs: Type::Vector(&Type::Boolean) },                               // VectorInitBoolean
        FunctionSignature { inputs: &[Type::Vector(&Type::Integer), Type::Integer], outputs: Type::Vector(&Type::Integer) }, // VectorPushInteger
        FunctionSignature { inputs: &[Type::Vector(&Type::Float), Type::Float], outputs: Type::Vector(&Type::Float) },       // VectorPushFloat
        FunctionSignature { inputs: &[Type::Vector(&Type::String), Type::String], outputs: Type::Vector(&Type::String) },    // VectorPushString
        FunctionSignature { inputs: &[Type::Vector(&Type::Char), Type::Char], outputs: Type::Vector(&Type::Char) },          // VectorPushChar
        FunctionSignature { inputs: &[Type::Vector(&Type::Boolean), Type::Boolean], outputs: Type::Vector(&Type::Boolean) }, // VectorPushBoolean
        FunctionSignature { inputs: &[Type::Vector(&Type::Integer)], outputs: Type::Vector(&Type::Integer) },                // VectorPopInteger
        FunctionSignature { inputs: &[Type::Vector(&Type::Float)], outputs: Type::Vector(&Type::Float) },                    // VectorPopFloat
        FunctionSignature { inputs: &[Type::Vector(&Type::String)], outputs: Type::Vector(&Type::String) },                  // VectorPopString
        FunctionSignature { inputs: &[Type::Vector(&Type::Char)], outputs: Type::Vector(&Type::Char) },                      // VectorPopChar
        FunctionSignature { inputs: &[Type::Vector(&Type::Boolean)], outputs: Type::Vector(&Type::Boolean) },                // VectorPopBoolean
    ]
};

#[derive(Debug, PartialEq)]
pub enum VerifyError {
    /// A slot was read before it was assigned.
    UnboundSlot { ip: usize, slot: Slot },
    /// The type of a slot did not match what was expected.
    TypeMismatch { ip: usize, slot: Slot, expected: Type, found: Type },
    /// The literal value in a Bind cannot be parsed as the declared type.
    InvalidLiteral { ip: usize, slot: Slot },
    /// A Jump or ConditionalJump target is outside the instruction list.
    JumpOutOfBounds { ip: usize, target: usize },
    /// The wrong number of arguments was supplied to a Call.
    ArgumentCountMismatch { ip: usize, expected: usize, found: usize },
    /// UnwrapSome used on a slot that is not Option<_>.
    NotAnOption { ip: usize, slot: Slot },
    /// UnwrapOk / UnwrapErr used on a slot that is not Result<_, _>.
    NotAResult { ip: usize, slot: Slot },
}

pub struct InstructionVerifier {
    instruction: Arc<[Instruction]>,
}

impl InstructionVerifier {
    pub fn new(instruction: Arc<[Instruction]>) -> Self {
        Self { instruction }
    }

    /// Verifies the instruction stream and returns all errors found.
    /// An empty Vec means the program is well-typed.
    pub fn verify(&self) -> Vec<VerifyError> {
        let instructions = &self.instruction;
        let len = instructions.len();
        let mut errors = Vec::new();
        // slot → type assigned so far (forward pass)
        let mut slots: HashMap<Slot, Type> = HashMap::new();

        for (ip, instruction) in instructions.iter().enumerate() {
            match instruction {
                // ── Bind ────────────────────────────────────────────────────
                Instruction::Bind { slot, type_name, value } => {
                    if !literal_matches_type(value, type_name) {
                        errors.push(VerifyError::InvalidLiteral { ip, slot: *slot });
                    }
                    slots.insert(*slot, type_name.clone());
                }

                // ── Call ────────────────────────────────────────────────────
                Instruction::Call { function_name, output, arguments } => {
                    let sig = &FUNCTION_REGISTRY.functions[*function_name as usize];

                    // argument count
                    if arguments.len() != sig.inputs.len() {
                        errors.push(VerifyError::ArgumentCountMismatch {
                            ip,
                            expected: sig.inputs.len(),
                            found: arguments.len(),
                        });
                    } else {
                        // argument types
                        for (arg_slot, expected_type) in arguments.iter().zip(sig.inputs.iter()) {
                            match slots.get(arg_slot) {
                                None => errors.push(VerifyError::UnboundSlot { ip, slot: *arg_slot }),
                                Some(found_type) if found_type != expected_type => {
                                    errors.push(VerifyError::TypeMismatch {
                                        ip,
                                        slot: *arg_slot,
                                        expected: expected_type.clone(),
                                        found: found_type.clone(),
                                    });
                                }
                                _ => {}
                            }
                        }
                    }

                    slots.insert(*output, sig.outputs.clone());
                }

                // ── Jump ────────────────────────────────────────────────────
                Instruction::Jump { target_position } => {
                    if *target_position >= len {
                        errors.push(VerifyError::JumpOutOfBounds { ip, target: *target_position });
                    }
                }

                // ── ConditionalJump ─────────────────────────────────────────
                Instruction::ConditionalJump { condition, true_target_position, false_target_position } => {
                    match slots.get(condition) {
                        None => errors.push(VerifyError::UnboundSlot { ip, slot: *condition }),
                        Some(t) if *t != Type::Boolean => {
                            errors.push(VerifyError::TypeMismatch {
                                ip,
                                slot: *condition,
                                expected: Type::Boolean,
                                found: t.clone(),
                            });
                        }
                        _ => {}
                    }
                    if *true_target_position >= len {
                        errors.push(VerifyError::JumpOutOfBounds { ip, target: *true_target_position });
                    }
                    if *false_target_position >= len {
                        errors.push(VerifyError::JumpOutOfBounds { ip, target: *false_target_position });
                    }
                }

                // ── UnwrapSome ──────────────────────────────────────────────
                Instruction::UnwrapSome { output, input } => {
                    match slots.get(input) {
                        None => errors.push(VerifyError::UnboundSlot { ip, slot: *input }),
                        Some(Type::Option(inner)) => {
                            slots.insert(*output, (*inner).clone());
                        }
                        Some(_) => errors.push(VerifyError::NotAnOption { ip, slot: *input }),
                    }
                }

                // ── UnwrapOk ────────────────────────────────────────────────
                Instruction::UnwrapOk { output, input } => {
                    match slots.get(input) {
                        None => errors.push(VerifyError::UnboundSlot { ip, slot: *input }),
                        Some(Type::Result(ok, _)) => {
                            slots.insert(*output, (*ok).clone());
                        }
                        Some(_) => errors.push(VerifyError::NotAResult { ip, slot: *input }),
                    }
                }

                // ── UnwrapErr ───────────────────────────────────────────────
                Instruction::UnwrapErr { output, input } => {
                    match slots.get(input) {
                        None => errors.push(VerifyError::UnboundSlot { ip, slot: *input }),
                        Some(Type::Result(_, err)) => {
                            slots.insert(*output, (*err).clone());
                        }
                        Some(_) => errors.push(VerifyError::NotAResult { ip, slot: *input }),
                    }
                }
            }
        }

        errors
    }
}

/// Returns true when the literal variant matches the declared type.
fn literal_matches_type(literal: &Literal, ty: &Type) -> bool {
    matches!(
        (literal, ty),
        (Literal::Integer(_), Type::Integer)
        | (Literal::Float(_),   Type::Float)
        | (Literal::Boolean(_), Type::Boolean)
        | (Literal::Char(_),    Type::Char)
        | (Literal::String(_),  Type::String)
    )
}
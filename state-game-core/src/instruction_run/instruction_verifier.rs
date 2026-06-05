use std::collections::HashMap;
use std::sync::Arc;
use crate::instruction_run::instruction::{FunctionRegistry, FunctionSignature, Functions, Instruction, Literal, Slot, SpecialFunctions};
use crate::instruction_run::types::Type;

pub(super) const FUNCTION_REGISTRY: FunctionRegistry<{ Functions::COUNT }> = FunctionRegistry {
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
        FunctionSignature { inputs: &[Type::Option(&Type::Any)], outputs: Type::Boolean },                                   // IsSome
        FunctionSignature { inputs: &[Type::Option(&Type::Any)], outputs: Type::Boolean },                                   // IsNone
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Any)], outputs: Type::Boolean },                       // IsOk
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Any)], outputs: Type::Boolean },                       // IsErr
        FunctionSignature { inputs: &[Type::Option(&Type::Integer)], outputs: Type::Integer },                               // UnwrapSomeInteger
        FunctionSignature { inputs: &[Type::Option(&Type::Float)], outputs: Type::Float },                                   // UnwrapSomeFloat
        FunctionSignature { inputs: &[Type::Option(&Type::String)], outputs: Type::String },                                 // UnwrapSomeString
        FunctionSignature { inputs: &[Type::Option(&Type::Char)], outputs: Type::Char },                                     // UnwrapSomeChar
        FunctionSignature { inputs: &[Type::Option(&Type::Boolean)], outputs: Type::Boolean },                               // UnwrapSomeBoolean
        FunctionSignature { inputs: &[Type::Result(&Type::Integer, &Type::Any)], outputs: Type::Integer },                   // UnwrapOkInteger
        FunctionSignature { inputs: &[Type::Result(&Type::Float, &Type::Any)], outputs: Type::Float },                       // UnwrapOkFloat
        FunctionSignature { inputs: &[Type::Result(&Type::String, &Type::Any)], outputs: Type::String },                     // UnwrapOkString
        FunctionSignature { inputs: &[Type::Result(&Type::Char, &Type::Any)], outputs: Type::Char },                         // UnwrapOkChar
        FunctionSignature { inputs: &[Type::Result(&Type::Boolean, &Type::Any)], outputs: Type::Boolean },                   // UnwrapOkBoolean
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Integer)], outputs: Type::Integer },                   // UnwrapErrInteger
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Float)], outputs: Type::Float },                       // UnwrapErrFloat
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::String)], outputs: Type::String },                     // UnwrapErrString
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Char)], outputs: Type::Char },                         // UnwrapErrChar
        FunctionSignature { inputs: &[Type::Result(&Type::Any, &Type::Boolean)], outputs: Type::Boolean },                   // UnwrapErrBoolean
    ]
};

pub(super) const SPECIAL_FUNCTIONS_REGISTRY: FunctionRegistry<{ SpecialFunctions::COUNT }> = FunctionRegistry {
    functions: [
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Option(&Type::Integer) }, // ReadGlobalMemoryInteger
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Option(&Type::Float) },   // ReadGlobalMemoryFloat
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Option(&Type::String)},   // ReadGlobalMemoryString
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Option(&Type::Char)},     // ReadGlobalMemoryChar
        FunctionSignature { inputs: &[Type::String, Type::String], outputs: Type::Option(&Type::Boolean)},  // ReadGlobalMemoryBoolean
        FunctionSignature { inputs: &[Type::String, Type::String, Type::Integer], outputs: Type::Void },    // WriteGlobalMemoryInteger
        FunctionSignature { inputs: &[Type::String, Type::String, Type::Float], outputs: Type::Void },      // WriteGlobalMemoryFloat
        FunctionSignature { inputs: &[Type::String, Type::String, Type::String], outputs: Type::Void },     // WriteGlobalMemoryString
        FunctionSignature { inputs: &[Type::String, Type::String, Type::Char], outputs: Type::Void },       // WriteGlobalMemoryChar
        FunctionSignature { inputs: &[Type::String, Type::String, Type::Boolean], outputs: Type::Void },    // WriteGlobalMemoryBoolean
        FunctionSignature { inputs: &[], outputs: Type::Integer },                                          // GetInstructionPosition
        FunctionSignature { inputs: &[], outputs: Type::Vector(&Type::String) },                            // GetModificationNamespaceList
    ]
};

#[derive(Debug, PartialEq)]
pub(super) enum VerifyError {
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
}

pub(super) struct InstructionVerifier {
    instruction: Arc<[Instruction]>,
}

impl InstructionVerifier {
    pub(super) fn new(instruction: Arc<[Instruction]>) -> Self {
        Self { instruction }
    }

    /// Verifies the instruction stream and returns all errors found.
    /// An empty Vec means the program is well-typed.
    pub(super) fn verify(&self) -> Vec<VerifyError> {
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
                                Some(found_type) if !type_compatible(found_type, expected_type) => {
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
                
                Instruction::SpecialCall { function_name, output, arguments } => {
                    let sig = &SPECIAL_FUNCTIONS_REGISTRY.functions[*function_name as usize];

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
                                Some(found_type) if !type_compatible(found_type, expected_type) => {
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

                Instruction::CallUserDefined { function_identifier: function_id, output, arguments } => {
                    continue //
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
        (Type::Result(fk, fv), Type::Result(ek, ev)) => type_compatible(fk, ek) && type_compatible(fv, ev),
        // Vector<T>: recurse on inner type
        (Type::Vector(f), Type::Vector(e)) => type_compatible(f, e),
        // Everything else: exact equality
        _ => found == expected,
    }
}

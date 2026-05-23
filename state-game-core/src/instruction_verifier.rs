use std::sync::Arc;
use lazy_static::lazy_static;
use crate::instruction::{FunctionRegistry, FunctionSignature, Functions, Instruction, Type};

lazy_static! {
    pub static ref FUNCTION_REGISTRY: FunctionRegistry<{ Functions::COUNT }> = FunctionRegistry {
        functions: [
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // AddInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // SubInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // MulInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // DivInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // ModInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Integer },                         // PowInteger
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Float },                               // AddFloat
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Float },                               // SubFloat
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Float },                               // MulFloat
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Float },                               // DivFloat
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Float },                               // PowFloat
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Boolean },                         // EqualInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Boolean },                         // NotEqualInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Boolean },                         // GreaterThanInteger
            FunctionSignature { inputs: Box::new([Type::Integer, Type::Integer]), outputs: Type::Boolean },                         // LessThanInteger
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Boolean },                             // GreaterThanFloat
            FunctionSignature { inputs: Box::new([Type::Float, Type::Float]), outputs: Type::Boolean },                             // LessThanFloat
            FunctionSignature { inputs: Box::new([Type::Boolean]), outputs: Type::Boolean },                                        // Not
            FunctionSignature { inputs: Box::new([Type::Boolean, Type::Boolean]), outputs: Type::Boolean },                         // And
            FunctionSignature { inputs: Box::new([Type::Boolean, Type::Boolean]), outputs: Type::Boolean },                         // Or
            FunctionSignature { inputs: Box::new([Type::Boolean, Type::Boolean]), outputs: Type::Boolean },                         // Xor
            FunctionSignature { inputs: Box::new([Type::String, Type::String]), outputs: Type::Boolean },                           // EqualString
            FunctionSignature { inputs: Box::new([Type::String]), outputs: Type::Integer },                                         // StringLength
            FunctionSignature { inputs: Box::new([Type::String, Type::Integer]), outputs: Type::Char },                             // StringGetChar
            FunctionSignature { inputs: Box::new([Type::Vector(Box::new(Type::Integer)), Type::Integer]), outputs: Type::Integer }, // VectorGetInteger
            FunctionSignature { inputs: Box::new([Type::Vector(Box::new(Type::Float)), Type::Integer]), outputs: Type::Float },     // VectorGetFloat
            FunctionSignature { inputs: Box::new([Type::Vector(Box::new(Type::String)), Type::Integer]), outputs: Type::String },   // VectorGetString
            FunctionSignature { inputs: Box::new([Type::Vector(Box::new(Type::Char)), Type::Integer]), outputs: Type::Char },       // VectorGetChar
            FunctionSignature { inputs: Box::new([Type::Vector(Box::new(Type::Boolean)), Type::Integer]), outputs: Type::Boolean }, // VectorGetBoolean
        ]
    };
}

pub struct InstructionVerifier {
    instruction: Arc<[Instruction]>,
}

impl InstructionVerifier {
    pub fn new(instruction: Arc<[Instruction]>) -> Self {
        Self { instruction }
    }

    pub fn verifier(&self) -> bool {
        let mut instruction_pointer = 0;


        true
    }
}
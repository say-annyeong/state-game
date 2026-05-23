mod types;

use lazy_static::lazy_static;
pub use crate::state_game_instruction::types::{Type, Value};

type Slot = u64;

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

pub enum Instruction {
    Bind {
        slot: Slot,
        type_name: Type,
        value: Literal
    },

    Call {
        function_name: Functions,
        output: Slot,
        arguments: Vec<Slot>
    },

    Jump {
        target_position: usize,
    },

    ConditionalJump {
        condition: Slot,
        true_target_position: usize,
        false_target_position: usize,
    },

    UnwrapSome {
        output: Slot,
        input: Slot,
    },

    UnwrapOk {
        output: Slot,
        input: Slot,
    },

    UnwrapErr {
        output: Slot,
        input: Slot,
    }
}

pub enum Literal {
    Integer(String),
    Float(String),
    String(String),
    Char(String),
    Boolean(String),
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Functions {
    AddInteger = 0,
    SubInteger = 1,
    MulInteger = 2,
    DivInteger = 3,
    ModInteger = 4,
    PowInteger = 5,
    AddFloat = 6,
    SubFloat = 7,
    MulFloat = 8,
    DivFloat = 9,
    PowFloat = 10,
    EqualInteger = 11,
    NotEqualInteger = 12,
    GreaterThanInteger = 13,
    LessThanInteger = 14,
    GreaterThanFloat = 15,
    LessThanFloat = 16,
    Not = 17,
    And = 18,
    Or = 19,
    Xor = 20,
    EqualString = 21,
    StringLength = 22,
    StringGetChar = 23,
    VectorGetInteger = 24,
    VectorGetFloat = 25,
    VectorGetString = 26,
    VectorGetChar = 27,
    VectorGetBoolean = 28,
}

impl Functions {
    pub const COUNT: usize = 29;
}

pub struct FunctionSignature {
    pub inputs:  Box<[Type]>,
    pub outputs: Type,
}

pub struct FunctionRegistry<const N: usize> {
    pub functions: [FunctionSignature; N],
}
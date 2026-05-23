mod types;

pub use crate::instruction::types::{Type, Value};

pub type Slot = u64;

#[derive(Clone)]
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

#[derive(Clone)]
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
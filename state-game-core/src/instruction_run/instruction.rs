use crate::instruction_run::types::Type;

pub type Slot = u64;

#[derive(Clone, Debug, PartialEq)]
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
        condition: Slot, // only Boolean
        true_target_position: usize,
        false_target_position: usize,
    },

    UnwrapSome {
        output: Slot,
        input: Slot, // only Option<Type>
    },

    UnwrapOk {
        output: Slot,
        input: Slot, // only Result<Type, Type>
    },

    UnwrapErr {
        output: Slot,
        input: Slot, // only Result<Type, Type>
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
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
    VectorInitInteger = 29,
    VectorInitFloat = 30,
    VectorInitString = 31,
    VectorInitChar = 32,
    VectorInitBoolean = 33,
    VectorPushInteger = 34,
    VectorPushFloat = 35,
    VectorPushString = 36,
    VectorPushChar = 37,
    VectorPushBoolean = 38,
    VectorPopInteger = 39,
    VectorPopFloat = 40,
    VectorPopString = 41,
    VectorPopChar = 42,
    VectorPopBoolean = 43,
}

impl Functions {
    pub const COUNT: usize = 44;
}

pub struct FunctionSignature {
    pub inputs:  &'static [Type],
    pub outputs: Type,
}

pub struct FunctionRegistry<const N: usize> {
    pub functions: [FunctionSignature; N],
}
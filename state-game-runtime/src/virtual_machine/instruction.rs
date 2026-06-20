use crate::virtual_machine::types::Type;

pub type Slot = u64;
pub type FunctionIdentifier = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    Bind {
        slot: Slot,
        type_name: Type,
        value: Literal
    },

    Call {
        function_name: Functions,
        inputs: Vec<Slot>,
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        output: Slot,
    },

    SpecialCall {
        function_name: SpecialFunctions,
        inputs: Vec<Slot>,
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        output: Slot,
    },

    CallDefined {
        function_identifier: FunctionIdentifier,
        inputs: Vec<Slot>, // input
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        destination_slots: Vec<Slot>,
        /// source
        ///
        /// WARNING: This slot does not belong to the currently running virtual machine.
        /// It refers to a slot within the domain of a different running virtual machine.
        source_slots: Vec<Slot>,
    },

    Jump {
        target_position: usize,
    },

    ConditionalJump {
        condition: Slot, // only Boolean
        true_target_position: usize,
        false_target_position: usize,
    },
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
    IsSome = 44,
    IsNone = 45,
    IsOk = 46,
    IsErr = 47,
    UnwrapSomeInteger = 48,
    UnwrapSomeFloat = 49,
    UnwrapSomeString = 50,
    UnwrapSomeChar = 51,
    UnwrapSomeBoolean = 52,
    UnwrapOkInteger = 53,
    UnwrapOkFloat = 54,
    UnwrapOkString = 55,
    UnwrapOkChar = 56,
    UnwrapOkBoolean = 57,
    UnwrapErrInteger = 58,
    UnwrapErrFloat = 59,
    UnwrapErrString = 60,
    UnwrapErrChar = 61,
    UnwrapErrBoolean = 62,
}

impl Functions {
    pub const COUNT: usize = 63;
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialFunctions {
    ReadGlobalMemoryInteger = 0,
    ReadGlobalMemoryFloat = 1,
    ReadGlobalMemoryString = 2,
    ReadGlobalMemoryChar = 3,
    ReadGlobalMemoryBoolean = 4,
    WriteGlobalMemoryInteger = 5,
    WriteGlobalMemoryFloat = 6,
    WriteGlobalMemoryString = 7,
    WriteGlobalMemoryChar = 8,
    WriteGlobalMemoryBoolean = 9,
    GetInstructionPosition = 10,
    GetModificationNamespaceList = 11,
}

impl SpecialFunctions {
    pub const COUNT: usize = 12;
}

pub struct FunctionSignature {
    pub inputs:  &'static [Type],
    pub outputs: Type,
}

pub struct DefinedFunctionSignature {
    pub inputs: Box<[Type]>,
    pub destinations: Box<[Type]>,
    pub source: Box<[Type]>,
}

pub struct FunctionRegistry<const N: usize> {
    pub functions: [FunctionSignature; N],
}
use crate::define_function_registry;
use crate::runtime_task::types::Type;

pub type Slot = u64;
pub type FunctionIdentifier = u64;
pub type RuntimeTaskIdentifier = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    Bind {
        slot: Slot,
        type_name: Type,
        value: Literal,
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

    DefinedCall {
        function_identifier: FunctionIdentifier,
        inputs: Vec<Slot>, // input
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        destination_slots: Vec<Slot>,
    },

    Jump {
        target_position: usize,
    },

    ConditionalJump {
        condition: Slot, // only Boolean
        true_target_position: usize,
        false_target_position: usize,
    },

    ReturnDefinedCall {
        function_identifier: FunctionIdentifier,
        outputs: Vec<Slot>,
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

define_function_registry!(
    pub enum Functions;
    pub const FUNCTION_REGISTRY;
    
    // AddInteger
    AddInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    SubInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    MulInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    DivInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    ModInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    PowInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Integer
    },
    AddFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Float
    },
    SubFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Float
    },
    MulFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Float
    },
    DivFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Float
    },
    PowFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Float
    },
    EqualInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Boolean
    },
    NotEqualInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Boolean
    },
    GreaterThanInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Boolean
    },
    LessThanInteger => {
        inputs: [Type::Integer, Type::Integer],
        output: Type::Boolean
    },
    GreaterThanFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Boolean
    },
    LessThanFloat => {
        inputs: [Type::Float, Type::Float],
        output: Type::Boolean
    },
    Not => {
        inputs: [Type::Boolean],
        output: Type::Boolean
    },
    And => {
        inputs: [Type::Boolean, Type::Boolean],
        output: Type::Boolean
    },
    Or => {
        inputs: [Type::Boolean, Type::Boolean],
        output: Type::Boolean
    },
    Xor => {
        inputs: [Type::Boolean, Type::Boolean],
        output: Type::Boolean
    },
    EqualString => {
        inputs: [Type::String, Type::String],
        output: Type::Boolean
    },
    StringLength => {
        inputs: [Type::String],
        output: Type::Integer
    },
    StringGetChar => {
        inputs: [Type::String, Type::Integer],
        output: Type::Char
    },
    VectorGetInteger => {
        inputs: [Type::Vector(&Type::Integer), Type::Integer],
        output: Type::Integer
    },
    VectorGetFloat => {
        inputs: [Type::Vector(&Type::Float), Type::Integer],
        output: Type::Float
    },
    VectorGetString => {
        inputs: [Type::Vector(&Type::String), Type::Integer],
        output: Type::String
    },
    VectorGetChar => {
        inputs: [Type::Vector(&Type::Char), Type::Integer],
        output: Type::Char
    },
    VectorGetBoolean => {
        inputs: [Type::Vector(&Type::Boolean), Type::Integer],
        output: Type::Boolean
    },
    VectorInitInteger => {
        inputs: [Type::Integer],
        output: Type::Vector(&Type::Integer)
    },
    VectorInitFloat => {
        inputs: [Type::Float],
        output: Type::Vector(&Type::Float)
    },
    VectorInitString => {
        inputs: [Type::String],
        output: Type::Vector(&Type::String)
    },
    VectorInitChar => {
        inputs: [Type::Char],
        output: Type::Vector(&Type::Char)
    },
    VectorInitBoolean => {
        inputs: [Type::Boolean],
        output: Type::Vector(&Type::Boolean)
    },
    VectorPushInteger => {
        inputs: [Type::Vector(&Type::Integer), Type::Integer],
        output: Type::Vector(&Type::Integer)
    },
    VectorPushFloat => {
        inputs: [Type::Vector(&Type::Float), Type::Float],
        output: Type::Vector(&Type::Float)
    },
    VectorPushString => {
        inputs: [Type::Vector(&Type::String), Type::String],
        output: Type::Vector(&Type::String)
    },
    VectorPushChar => {
        inputs: [Type::Vector(&Type::Char), Type::Char],
        output: Type::Vector(&Type::Char)
    },
    VectorPushBoolean => {
        inputs: [Type::Vector(&Type::Boolean), Type::Boolean],
        output: Type::Vector(&Type::Boolean)
    },
    VectorPopInteger => {
        inputs: [Type::Vector(&Type::Integer)],
        output: Type::Vector(&Type::Integer)
    },
    VectorPopFloat => {
        inputs: [Type::Vector(&Type::Float)],
        output: Type::Vector(&Type::Float)
    },
    VectorPopString => {
        inputs: [Type::Vector(&Type::String)],
        output: Type::Vector(&Type::String)
    },
    VectorPopChar => {
        inputs: [Type::Vector(&Type::Char)],
        output: Type::Vector(&Type::Char)
    },
    VectorPopBoolean => {
        inputs: [Type::Vector(&Type::Boolean)],
        output: Type::Vector(&Type::Boolean)
    },
    IsSome => {
        inputs: [Type::Option(&Type::Any)],
        output: Type::Boolean
    },
    IsNone => {
        inputs: [Type::Option(&Type::Any)],
        output: Type::Boolean
    },
    IsOk => {
        inputs: [Type::Result(&Type::Any, &Type::Any)],
        output: Type::Boolean
    },
    IsErr => {
        inputs: [Type::Result(&Type::Any, &Type::Any)],
        output: Type::Boolean
    },
    UnwrapSomeInteger => {
        inputs: [Type::Option(&Type::Integer)],
        output: Type::Integer
    },
    UnwrapSomeFloat => {
        inputs: [Type::Option(&Type::Float)],
        output: Type::Float
    },
    UnwrapSomeString => {
        inputs: [Type::Option(&Type::String)],
        output: Type::String
    },
    UnwrapSomeChar => {
        inputs: [Type::Option(&Type::Char)],
        output: Type::Char
    },
    UnwrapSomeBoolean => {
        inputs: [Type::Option(&Type::Boolean)],
        output: Type::Boolean
    },
    UnwrapOkInteger => {
        inputs: [Type::Result(&Type::Integer, &Type::Any)],
        output: Type::Integer
    },
    UnwrapOkFloat => {
        inputs: [Type::Result(&Type::Float, &Type::Any)],
        output: Type::Float
    },
    UnwrapOkString => {
        inputs: [Type::Result(&Type::String, &Type::Any)],
        output: Type::String
    },
    UnwrapOkChar => {
        inputs: [Type::Result(&Type::Char, &Type::Any)],
        output: Type::Char
    },
    UnwrapOkBoolean => {
        inputs: [Type::Result(&Type::Boolean, &Type::Any)],
        output: Type::Boolean
    },
    UnwrapErrInteger => {
        inputs: [Type::Result(&Type::Any, &Type::Integer)],
        output: Type::Integer
    },
    UnwrapErrFloat => {
        inputs: [Type::Result(&Type::Any, &Type::Float)],
        output: Type::Float
    },
    UnwrapErrString => {
        inputs: [Type::Result(&Type::Any, &Type::String)],
        output: Type::String
    },
    UnwrapErrChar => {
        inputs: [Type::Result(&Type::Any, &Type::Char)],
        output: Type::Char
        },
    UnwrapErrBoolean => {
        inputs: [Type::Result(&Type::Any, &Type::Boolean)],
        output: Type::Boolean
    }
);

define_function_registry!(
    pub enum SpecialFunctions;
    pub const SPECIAL_FUNCTIONS_REGISTRY;
    
    ReadGlobalMemoryInteger => {
        inputs: [Type::String, Type::String],
        output: Type::Result(&Type::Integer, &Type::String)
    },
    ReadGlobalMemoryFloat => {
        inputs: [Type::String, Type::String],
        output: Type::Result(&Type::Float, &Type::String)
    },
    ReadGlobalMemoryString => {
        inputs: [Type::String, Type::String],
        output: Type::Result(&Type::String, &Type::String)
    },
    ReadGlobalMemoryChar => {
        inputs: [Type::String, Type::String],
        output: Type::Result(&Type::Char, &Type::String)
    },
    ReadGlobalMemoryBoolean => {
        inputs: [Type::String, Type::String],
        output: Type::Result(&Type::Boolean, &Type::String)
    },
    WriteGlobalMemoryInteger => {
        inputs: [Type::String, Type::String, Type::Integer],
        output: Type::Void
    },
    WriteGlobalMemoryFloat => {
        inputs: [Type::String, Type::String, Type::Float],
        output: Type::Void
    },
    WriteGlobalMemoryString => {
        inputs: [Type::String, Type::String, Type::String],
        output: Type::Void
    },
    WriteGlobalMemoryChar => {
        inputs: [Type::String, Type::String, Type::Char],
        output: Type::Void
    },
    WriteGlobalMemoryBoolean => {
        inputs: [Type::String, Type::String, Type::Boolean],
        output: Type::Void
    },
    GetInstructionPosition => {
        inputs: [],
        output: Type::Integer
    },
    GetModificationNamespaceList => {
        inputs: [],
        output: Type::Vector(&Type::String)
    },
);

pub struct FunctionSignature {
    pub inputs: &'static [Type],
    pub output: Type,
}

pub struct DefinedFunctionSignature {
    pub inputs: Box<[Type]>,
    pub destinations: Box<[Type]>,
    pub source: Box<[Type]>,
}

pub struct FunctionRegistry<const N: usize> {
    pub functions: [FunctionSignature; N],
}

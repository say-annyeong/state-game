use crate::{
    runtime_task::types::{ConcreteType, GenericIdentifier, GenericKind, GenericParameter, PrimitiveType, TypeExpression},
    define_function_registry
};

pub type Slot = u64;
pub type FunctionIdentifier = u64;
pub type RuntimeTaskIdentifier = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction<'a> {
    Bind {
        slot: Slot,
        type_name: PrimitiveType,
        value: Literal,
    },

    Call {
        function_name: Functions,
        generic_arguments: Box<[ConcreteType<'a>]>,
        inputs: Vec<Slot>,
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        output: Slot,
    },

    SpecialCall {
        function_name: SpecialFunctions,
        generic_arguments: Box<[ConcreteType<'a>]>,
        inputs: Vec<Slot>,
        /// The output must undergo the same type checking as Bind.
        /// Execution will fail if there is a type mismatch.
        output: Slot,
    },

    DefinedCall {
        function_identifier: FunctionIdentifier,
        generic_arguments: Box<[ConcreteType<'a>]>,
        inputs: Vec<Slot>, // input
        outputs: Vec<Slot>,
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

define_function_registry!(
    pub enum Functions;
    pub const FUNCTION_REGISTRY;

    AddInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    SubInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    MulInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    DivInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    ModInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    PowInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    AddFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Float)
    },
    SubFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Float)
    },
    MulFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Float)
    },
    DivFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Float)
    },
    PowFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Float)
    },
    EqualInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    NotEqualInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    GreaterThanInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    LessThanInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Primitive(PrimitiveType::Integer)], // integer1, integer2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    GreaterThanFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    LessThanFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Float), TypeExpression::Primitive(PrimitiveType::Float)], // float1, float2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    Not => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Boolean)], // bool
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    And => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Boolean), TypeExpression::Primitive(PrimitiveType::Boolean)], // bool1, bool2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    Or => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Boolean), TypeExpression::Primitive(PrimitiveType::Boolean)], // bool1, bool2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    Xor => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Boolean), TypeExpression::Primitive(PrimitiveType::Boolean)], // bool1, bool2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    EqualString => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Primitive(PrimitiveType::String)], // string1, string2
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    StringLength => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String)], // string
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    StringGetChar => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Primitive(PrimitiveType::Integer)], // string, index
        output: TypeExpression::Primitive(PrimitiveType::Char)
    },
    StringCombine => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Primitive(PrimitiveType::String)], // string1, string2
        output: TypeExpression::Primitive(PrimitiveType::String)
    },
    StringToInteger => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String)], // string
        output: TypeExpression::Option(&TypeExpression::Primitive(PrimitiveType::Integer))
    },
    StringToFloat => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String)], // string
        output: TypeExpression::Option(&TypeExpression::Primitive(PrimitiveType::Float))
    },
    StringToBoolean => {
        generics: &[],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String)], // string
        output: TypeExpression::Option(&TypeExpression::Primitive(PrimitiveType::Boolean))
    },
    VectorGet => {
        generics: &[],
        inputs: &[TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0))), TypeExpression::Primitive(PrimitiveType::Integer)], // vector, index
        output: TypeExpression::Generic(GenericIdentifier(0))
    },
    VectorNew => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[],
        output: TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0)))
    },
    VectorPush => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0))), TypeExpression::Generic(GenericIdentifier(0))], // vector, value
        output: TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0)))
    },
    VectorPop => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0)))], // vector
        output: TypeExpression::Vector(&TypeExpression::Generic(GenericIdentifier(0)))
    },
    IsSome => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Option(&TypeExpression::Generic(GenericIdentifier(0)))], // option
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    IsNone => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Option(&TypeExpression::Generic(GenericIdentifier(0)))], // option
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    IsOk => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }, GenericParameter {
            id: GenericIdentifier(1),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Result { ok: &TypeExpression::Generic(GenericIdentifier(0)), err: &TypeExpression::Generic(GenericIdentifier(1))}], // result
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    IsErr => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }, GenericParameter {
            id: GenericIdentifier(1),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Result { ok: &TypeExpression::Generic(GenericIdentifier(0)), err: &TypeExpression::Generic(GenericIdentifier(1))}], // result
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    UnwrapSome => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Option(&TypeExpression::Generic(GenericIdentifier(0)))], // option
        output: TypeExpression::Generic(GenericIdentifier(0))
    },
    UnwrapOk => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }, GenericParameter {
            id: GenericIdentifier(1),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Result { ok: &TypeExpression::Generic(GenericIdentifier(0)), err: &TypeExpression::Generic(GenericIdentifier(1))}], // result
        output: TypeExpression::Generic(GenericIdentifier(0))
    },
    UnwrapErr => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[]
        }, GenericParameter {
            id: GenericIdentifier(1),
            kind: GenericKind::Type,
            constraints: &[]
        }],
        inputs: &[TypeExpression::Result { ok: &TypeExpression::Generic(GenericIdentifier(0)), err: &TypeExpression::Generic(GenericIdentifier(1))}], // result
        output: TypeExpression::Generic(GenericIdentifier(1))
    },
);

define_function_registry!(
    pub enum SpecialFunctions;
    pub const SPECIAL_FUNCTIONS_REGISTRY;

    ReadGlobalMemory => {
        generics: &[GenericParameter{
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Primitive(PrimitiveType::String)], // namespace, identifier
        output: TypeExpression::Result { ok: &TypeExpression::Generic(GenericIdentifier(0)), err: &TypeExpression::Primitive(PrimitiveType::String) }
    },
    WriteGlobalMemory => {
        generics: &[GenericParameter{
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Primitive(PrimitiveType::String), TypeExpression::Generic(GenericIdentifier(0))], // namespace, identifier, value
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    },
    GetInstructionPosition => {
        generics: &[],
        inputs: &[],
        output: TypeExpression::Primitive(PrimitiveType::Integer)
    },
    GetModificationNamespaceList => {
        generics: &[],
        inputs: &[],
        output: TypeExpression::Vector(&TypeExpression::Primitive(PrimitiveType::String))
    },
    GetInputSlot => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer)], // index
        output: TypeExpression::Generic(GenericIdentifier(0))
    },
    WriteOutputSlot => {
        generics: &[GenericParameter {
            id: GenericIdentifier(0),
            kind: GenericKind::Type,
            constraints: &[],
        }],
        inputs: &[TypeExpression::Primitive(PrimitiveType::Integer), TypeExpression::Generic(GenericIdentifier(0))], // index, value
        output: TypeExpression::Primitive(PrimitiveType::Boolean)
    }
);

pub struct FunctionSignature {
    pub generics: &'static [GenericParameter],
    pub inputs: &'static [TypeExpression<'static>],
    pub output: TypeExpression<'static>,
}

pub struct DefinedFunctionSignature {
    pub inputs: Box<[TypeExpression<'static>]>,
    pub outputs: Box<[TypeExpression<'static>]>,
}

pub struct FunctionRegistry<const N: usize> {
    pub functions: [FunctionSignature; N],
}

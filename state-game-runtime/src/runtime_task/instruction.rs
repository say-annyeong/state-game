use crate::{
    runtime_task::types::{
        ConcreteType, GenericIdentifier, GenericKind, GenericParameter,
        GenericTypeIdentifier, GenericTypeInterner, PrimitiveType, TypeExpression,
    },
    define_function_registry,
};

pub type Slot = u64;
pub type FunctionIdentifier = u64;
pub type RuntimeTaskIdentifier = u64;

pub type ConcreteInstruction = Instruction<ConcreteType>;
pub type ExpressionInstruction = Instruction<TypeExpression>;

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction<Type> {
    Bind {
        slot: Slot,
        type_name: PrimitiveType,
        value: Literal,
    },

    Call {
        function_name: Functions,
        generic_arguments: Box<[Type]>,
        inputs: Vec<Slot>,
        output: Slot,
    },

    SpecialCall {
        function_name: SpecialFunctions,
        generic_arguments: Box<[Type]>,
        inputs: Vec<Slot>,
        output: Slot,
    },

    DefinedCall {
        function_identifier: FunctionIdentifier,
        generic_arguments: Box<[Type]>,
        inputs: Vec<Slot>,
        outputs: Vec<Slot>,
    },

    Jump {
        target_position: usize,
    },

    ConditionalJump {
        condition: Slot,
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

// ---------------------------------------------------------------------------
// FunctionSignature: built once per registry lookup via an interner.
// ---------------------------------------------------------------------------

pub struct FunctionSignature {
    pub generics: Vec<GenericParameter>,
    pub inputs: Vec<TypeExpression>,
    pub output: TypeExpression,
}

pub struct FunctionRegistry<const N: usize> {
    /// Each entry is a builder that creates a `FunctionSignature` using the
    /// provided `GenericTypeInterner`.
    pub builders: [fn(&mut GenericTypeInterner) -> FunctionSignature; N],
}

impl<const N: usize> FunctionRegistry<N> {
    pub fn build(&self, index: usize, interner: &mut GenericTypeInterner) -> Option<FunctionSignature> {
        self.builders.get(index).map(|f| f(interner))
    }
}

// ---------------------------------------------------------------------------
// Helper used in builders below
// ---------------------------------------------------------------------------

fn primitive(ty: PrimitiveType) -> TypeExpression {
    TypeExpression::Primitive(ty)
}

fn vector_of(inner: TypeExpression, interner: &mut GenericTypeInterner) -> TypeExpression {
    let id = interner.intern(inner);
    TypeExpression::Vector(id)
}

fn option_of(inner: TypeExpression, interner: &mut GenericTypeInterner) -> TypeExpression {
    let id = interner.intern(inner);
    TypeExpression::Option(id)
}

fn result_of(ok: TypeExpression, err: TypeExpression, interner: &mut GenericTypeInterner) -> TypeExpression {
    let ok_id = interner.intern(ok);
    let err_id = interner.intern(err);
    TypeExpression::Result { ok: ok_id, err: err_id }
}

fn generic(id: u64) -> TypeExpression {
    TypeExpression::Generic(GenericIdentifier(id))
}

fn generic_parameter(id: u64) -> GenericParameter {
    GenericParameter {
        id: GenericIdentifier(id),
        kind: GenericKind::Type,
        constraints: &[],
    }
}

// ---------------------------------------------------------------------------
// Functions enum + FUNCTION_REGISTRY
// ---------------------------------------------------------------------------

define_function_registry!(
    pub enum Functions;
    pub const FUNCTION_REGISTRY;

    // ── Integer arithmetic ────────────────────────────────────────────────────
    AddInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    SubInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    MulInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    DivInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    RemInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    RemEuclidInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    PowInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Integer),
        }
    },

    // ── Float arithmetic ──────────────────────────────────────────────────────
    AddFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Float),
        }
    },
    SubFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Float),
        }
    },
    MulFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Float),
        }
    },
    DivFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Float),
        }
    },
    PowFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Float),
        }
    },

    // ── Integer comparisons ───────────────────────────────────────────────────
    EqualInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    NotEqualInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    GreaterThanInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    LessThanInteger => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    GreaterThanFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    LessThanFloat => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Float), primitive(PrimitiveType::Float)],
            output: primitive(PrimitiveType::Boolean),
        }
    },

    // ── Boolean logic ─────────────────────────────────────────────────────────
    Not => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Boolean)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    And => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Boolean), primitive(PrimitiveType::Boolean)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    Or => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Boolean), primitive(PrimitiveType::Boolean)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    Xor => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::Boolean), primitive(PrimitiveType::Boolean)],
            output: primitive(PrimitiveType::Boolean),
        }
    },

    // ── String operations ─────────────────────────────────────────────────────
    EqualString => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String), primitive(PrimitiveType::String)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    StringLength => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String)],
            output: primitive(PrimitiveType::Integer),
        }
    },
    StringGetChar => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String), primitive(PrimitiveType::Integer)],
            output: primitive(PrimitiveType::Char),
        }
    },
    StringCombine => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String), primitive(PrimitiveType::String)],
            output: primitive(PrimitiveType::String),
        }
    },
    StringToInteger => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String)],
            output: option_of(primitive(PrimitiveType::Integer), i),
        }
    },
    StringToFloat => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String)],
            output: option_of(primitive(PrimitiveType::Float), i),
        }
    },
    StringToBoolean => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![primitive(PrimitiveType::String)],
            output: option_of(primitive(PrimitiveType::Boolean), i),
        }
    },

    // ── Vector operations ─────────────────────────────────────────────────────
    VectorGet => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![vector_of(generic(0), i), primitive(PrimitiveType::Integer)],
            output: generic(0),
        }
    },
    VectorNew => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![],
            output: vector_of(generic(0), i),
        }
    },
    VectorPush => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![vector_of(generic(0), i), generic(0)],
            output: vector_of(generic(0), i),
        }
    },
    VectorPop => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![vector_of(generic(0), i)],
            output: vector_of(generic(0), i),
        }
    },

    // ── Option operations ─────────────────────────────────────────────────────
    IsSome => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![option_of(generic(0), i)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    IsNone => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![option_of(generic(0), i)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    IsOk => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0), generic_parameter(1)],
            inputs: vec![result_of(generic(0), generic(1), i)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    IsErr => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0), generic_parameter(1)],
            inputs: vec![result_of(generic(0), generic(1), i)],
            output: primitive(PrimitiveType::Boolean),
        }
    },
    UnwrapSome => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![option_of(generic(0), i)],
            output: generic(0),
        }
    },
    UnwrapOk => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0), generic_parameter(1)],
            inputs: vec![result_of(generic(0), generic(1), i)],
            output: generic(0),
        }
    },
    UnwrapErr => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0), generic_parameter(1)],
            inputs: vec![result_of(generic(0), generic(1), i)],
            output: generic(1),
        }
    },
);

// ---------------------------------------------------------------------------
// SpecialFunctions enum + SPECIAL_FUNCTIONS_REGISTRY
// ---------------------------------------------------------------------------

define_function_registry!(
    pub enum SpecialFunctions;
    pub const SPECIAL_FUNCTIONS_REGISTRY;

    ReadGlobalMemory => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![primitive(PrimitiveType::String), primitive(PrimitiveType::String)],
            output: result_of(generic(0), primitive(PrimitiveType::String), i),
        }
    },
    WriteGlobalMemory => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![primitive(PrimitiveType::String), primitive(PrimitiveType::String), generic(0)],
            output: result_of(primitive(PrimitiveType::Unit), primitive(PrimitiveType::String), i),
        }
    },
    GetInstructionPosition => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![],
            output: primitive(PrimitiveType::Integer),
        }
    },
    GetModificationNamespaceList => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![],
            inputs: vec![],
            output: vector_of(primitive(PrimitiveType::String), i),
        }
    },
    GetInputSlot => {
        |_i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![primitive(PrimitiveType::Integer)],
            output: generic(0),
        }
    },
    WriteOutputSlot => {
        |i: &mut GenericTypeInterner| FunctionSignature {
            generics: vec![generic_parameter(0)],
            inputs: vec![primitive(PrimitiveType::Integer), generic(0)],
            output: result_of(primitive(PrimitiveType::Unit), primitive(PrimitiveType::String), i),
        }
    },
);

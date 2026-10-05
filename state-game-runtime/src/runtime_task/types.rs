use std::collections::HashMap;
use std::sync::Arc;

use crate::persistent_vector::PersistentVector;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Unit,
    Integer,
    Float,
    String,
    Char,
    Boolean,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GenericIdentifier(pub u64);

/// ID for a type node that may contain generics (used inside TypeExpression).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GenericTypeIdentifier(pub u64);

/// ID for a fully-concrete type node (used inside ConcreteType).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NonGenericTypeIdentifier(pub u64);

// ---------------------------------------------------------------------------
// TypeExpression — may reference generics; inner nodes stored by ID
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpression {
    Primitive(PrimitiveType),
    Generic(GenericIdentifier),
    Vector(GenericTypeIdentifier),
    Option(GenericTypeIdentifier),
    Result { ok: GenericTypeIdentifier, err: GenericTypeIdentifier },
}

// ---------------------------------------------------------------------------
// ConcreteType — no generics; inner nodes stored by ID
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConcreteType {
    Primitive(PrimitiveType),
    Vector(NonGenericTypeIdentifier),
    Option(NonGenericTypeIdentifier),
    Result { ok: NonGenericTypeIdentifier, err: NonGenericTypeIdentifier },
}

// ---------------------------------------------------------------------------
// Generic-type interner: stores TypeExpression nodes, returns GenericTypeIdentifier
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct GenericTypeInterner {
    types: Vec<TypeExpression>,
    map: HashMap<TypeExpression, GenericTypeIdentifier>,
}

impl GenericTypeInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, ty: TypeExpression) -> GenericTypeIdentifier {
        if let Some(id) = self.map.get(&ty) {
            return *id;
        }
        let id = GenericTypeIdentifier(self.types.len() as u64);
        self.types.push(ty.clone());
        self.map.insert(ty, id);
        id
    }

    pub fn get(&self, id: GenericTypeIdentifier) -> Option<&TypeExpression> {
        self.types.get(id.0 as usize)
    }
}

// ---------------------------------------------------------------------------
// Concrete-type interner: stores ConcreteType nodes, returns NonGenericTypeIdentifier
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct ConcreteTypeInterner {
    types: Vec<ConcreteType>,
    map: HashMap<ConcreteType, NonGenericTypeIdentifier>,
}

impl ConcreteTypeInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, ty: ConcreteType) -> NonGenericTypeIdentifier {
        if let Some(id) = self.map.get(&ty) {
            return *id;
        }
        let id = NonGenericTypeIdentifier(self.types.len() as u64);
        self.types.push(ty.clone());
        self.map.insert(ty, id);
        id
    }

    pub fn get(&self, id: NonGenericTypeIdentifier) -> Option<&ConcreteType> {
        self.types.get(id.0 as usize)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a ConcreteType into a TypeExpression using an interner for
/// any nested node IDs that need to be re-registered.
pub fn concrete_to_expression(
    ct: &ConcreteType,
    concrete_interner: &ConcreteTypeInterner,
    generic_interner: &mut GenericTypeInterner,
) -> TypeExpression {
    match ct {
        ConcreteType::Primitive(p) => TypeExpression::Primitive(*p),
        ConcreteType::Vector(inner_id) => {
            let inner_ct = concrete_interner.get(*inner_id).cloned()
                .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit));
            let inner_te = concrete_to_expression(&inner_ct, concrete_interner, generic_interner);
            let gid = generic_interner.intern(inner_te);
            TypeExpression::Vector(gid)
        }
        ConcreteType::Option(inner_id) => {
            let inner_ct = concrete_interner.get(*inner_id).cloned()
                .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit));
            let inner_te = concrete_to_expression(&inner_ct, concrete_interner, generic_interner);
            let gid = generic_interner.intern(inner_te);
            TypeExpression::Option(gid)
        }
        ConcreteType::Result { ok: ok_id, err: err_id } => {
            let ok_ct = concrete_interner.get(*ok_id).cloned()
                .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit));
            let err_ct = concrete_interner.get(*err_id).cloned()
                .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit));
            let ok_te = concrete_to_expression(&ok_ct, concrete_interner, generic_interner);
            let err_te = concrete_to_expression(&err_ct, concrete_interner, generic_interner);
            let ok_gid = generic_interner.intern(ok_te);
            let err_gid = generic_interner.intern(err_te);
            TypeExpression::Result { ok: ok_gid, err: err_gid }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericKind {
    Type,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericParameter {
    pub id: GenericIdentifier,
    pub kind: GenericKind,
    pub constraints: &'static [Constraint],
}

// ---------------------------------------------------------------------------
// RuntimeValue
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum RuntimeValue {
    Uninitialized,

    Unit,
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Char(char),
    String(Arc<str>),

    Vector(PersistentVector<Self>),

    Option(Option<Arc<Self>>),

    Result(Result<Arc<Self>, Arc<Self>>),
}

impl RuntimeValue {
    pub fn integer(&self) -> &i64 {
        let Self::Integer(integer) = self else { unreachable!() };
        integer
    }

    pub fn float(&self) -> &f64 {
        let Self::Float(float) = self else { unreachable!() };
        float
    }

    pub fn boolean(&self) -> &bool {
        let Self::Boolean(boolean) = self else { unreachable!() };
        boolean
    }

    pub fn char(&self) -> &char {
        let Self::Char(char) = self else { unreachable!() };
        char
    }

    pub fn string(&self) -> &Arc<str> {
        let Self::String(string) = self else { unreachable!() };
        string
    }

    pub fn vector(&self) -> &PersistentVector<Self> {
        let Self::Vector(vector) = self else { unreachable!() };
        vector
    }

    pub fn option(&self) -> &Option<Arc<Self>> {
        let Self::Option(option) = self else { unreachable!() };
        option
    }

    pub fn result(&self) -> &Result<Arc<Self>, Arc<Self>> {
        let Self::Result(result) = self else { unreachable!() };
        result
    }
}

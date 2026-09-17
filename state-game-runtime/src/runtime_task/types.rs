use std::sync::Arc;

use bumpalo::Bump;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeIdentifier(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpression<'a> {
    Primitive(PrimitiveType),

    Generic(GenericIdentifier),

    Vector(&'a Self),
    Option(&'a Self),
    Result { ok: &'a Self, err: &'a Self },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericKind {
    Type,
    // Const, // todo
    // Lifetime, // todo
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    None, // todo
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericParameter {
    pub id: GenericIdentifier,
    pub kind: GenericKind,
    pub constraints: &'static [Constraint],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConcreteType<'a> {
    Primitive(PrimitiveType),

    Vector(&'a Self),
    Option(&'a Self),
    Result { ok: &'a Self, err: &'a Self },
}

pub fn convert<'a>(
    value: &ConcreteType<'a>,
    arena: &'a Bump,
) -> &'a TypeExpression<'a> {
    match value {
        ConcreteType::Primitive(p) => {
            arena.alloc(TypeExpression::Primitive(*p))
        }

        ConcreteType::Vector(inner) => {
            let child = convert(inner, arena);

            arena.alloc(TypeExpression::Vector(child))
        }

        ConcreteType::Option(inner) => {
            let child = convert(inner, arena);

            arena.alloc(TypeExpression::Option(child))
        }

        ConcreteType::Result { ok, err } => {
            let ok = convert(ok, arena);
            let err = convert(err, arena);

            arena.alloc(TypeExpression::Result { ok, err })
        }
    }
}

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
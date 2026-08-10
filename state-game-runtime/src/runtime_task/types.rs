use crate::persistent_vector::PersistentVector;

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Type {
    Integer,
    Float,
    String,
    Char,
    Boolean,
    Vector(&'static Type),

    Option(&'static Type),
    Result(&'static Type, &'static Type),

    /// Only valid in function input positions.
    ///
    /// Any accepts every concrete type as an argument.
    ///
    /// ConcreteType -> Any     : allowed
    /// Any -> ConcreteType     : denied
    ///
    /// Any must never appear in outputs,
    /// variable storage, or inferred value types.
    Any,
    /// Only valid in function output positions.
    ///
    /// Represents the absence of a return value.
    Void,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Vector(PersistentVector<Value>),

    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),

    Any, // can't use value
    Void,
}

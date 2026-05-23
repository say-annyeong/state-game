#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Type {
    Integer,
    Float,
    String,
    Char,
    Boolean,
    Vector(Box<Type>),

    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),

    Void
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Vector(Vec<Value>),

    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),

    Void
}
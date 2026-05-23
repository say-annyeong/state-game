#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Type {
    Integer,
    Float,
    String,
    Char,
    Boolean,
    Vector(&'static Type),

    Option(&'static Type),
    Result(&'static Type, &'static Type),

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
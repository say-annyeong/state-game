pub enum Instruction {
    Bind {
        identifier: String,
        type_name: String,
        value: Literal
    },

    Call {
        function_name: String,
        output: String,
        arguments: Vec<String>
    },

    Jump {
        label: String,
    },

    ConditionalJump {
        condition: String,
        true_label: String,
        false_label: String,
    },

    UnwrapSome {
        output: String,
        input: String,
    },

    UnwrapOk {
        output: String,
        input: String,
    },

    UnwrapErr {
        output: String,
        input: String,
    },

    Label(String)
}

pub enum Literal {
    Integer(String),
    Float(String),
    String(String),
    Char(String),
    Boolean(String),
}

pub enum Type {
    Integer,
    Float,
    String,
    Char,
    Boolean,
    Vec(Box<Type>),

    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),

    Void
}

pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Vec(Vec<Value>),

    Option(Box<Value>),
    Result(Box<Value>, Box<Value>),

    Void
}
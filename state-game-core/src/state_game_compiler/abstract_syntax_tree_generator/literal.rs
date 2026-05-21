#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(String),    // Store as string initially, parse later
    Float(String),  // Store as string initially, parse later
    String(String),
    Boolean(bool),
    Null,
    // Potentially add ListLiteral and DictLiteral here if preferred over Expression variants
}

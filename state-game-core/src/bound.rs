pub enum Constraints {

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicValue {
    True,
    False,
    Unknown,
}

impl LogicValue {
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) => Self::False,
            (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) => Self::True,
            (_, Self::True ) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    pub fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

pub trait CustomLogicOp: Send + Sync {
    fn combine(&self, inputs: &[LogicValue]) -> LogicValue;
}
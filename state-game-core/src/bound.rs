use crate::{Identifier, Namespace};
use std::any::Any;
use std::sync::Arc;

/// 노드 고유 ID — Binding 추적 / 캐시 키 / dependency graph 키
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIdentifier(pub u64);

/// 함수 식별자
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionIdentifier(pub String);

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
            (_, Self::True) => Self::True,
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

#[derive(Clone)]
pub enum LogicOperators {
    And,
    Or,
    Not,
    Custom(Arc<dyn CustomLogicOp>),
}

impl LogicOperators {
    pub fn combine(&self, inputs: &[LogicValue]) -> LogicValue {
        match self {
            Self::And => inputs
                .iter()
                .copied()
                .fold(LogicValue::True, LogicValue::and),
            Self::Or => inputs
                .iter()
                .copied()
                .fold(LogicValue::True, LogicValue::or),
            Self::Not => inputs
                .get(0)
                .copied()
                .map_or(LogicValue::Unknown, LogicValue::not),
            Self::Custom(op) => op.combine(inputs),
        }
    }
}

pub enum FunctionOutputs {
    AbstractSyntaxTree(Expression),
    Data(Data),
    Both(Expression, Data),
}

impl FunctionOutputs {
    pub fn into_parts(self) -> (Option<Expression>, Option<Data>) {
        match self {
            Self::AbstractSyntaxTree(a) => (Some(a), None),
            Self::Data(d) => (None, Some(d)),
            Self::Both(e, d) => (Some(e), Some(d)),
        }
    }

    pub fn expr_ref(&self) -> Option<&Expression> {
        match self {
            Self::AbstractSyntaxTree(a) | Self::Both(a, _) => Some(a),
            _ => None,
        }
    }

    pub fn data_ref(&self) -> Option<&Data> {
        match self {
            Self::Data(d) | Self::Both(_, d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Data {
    pub namespace: Namespace,
    pub identifier: Identifier,
    pub data: Box<dyn Any + Send + Sync>,
}

#[derive(Debug, Clone)]
pub enum ParamRole {
    /// 해당 자식의 Expr → ast 파라미터
    Ast,
    /// 해당 자식의 Data → data 파라미터
    DataOnly,
    /// 해당 자식의 Expr + Data → 각각 전달
    Both,
}

#[derive(Debug, Clone)]
pub struct ParamBinding {
    pub child_index: usize,
    pub role: ParamRole,
}

#[derive(Clone)]
pub enum Expression {
    /// 외부 주입 대기 리프. 충족 전 Unknown.
    Binding(NodeIdentifier),

    /// 확정 리프
    Literal(LogicValue),

    /// 논리 연산 노드 (Logic 체인 — DependencyGraph에 명시 등록)
    Logic {
        id: NodeIdentifier,
        op: LogicOperators,
        children: Vec<Self>,
    },

    /// 함수 노드 — AST ↔ Data 유일한 bridge
    /// 자식 노드가 입력 파라미터.
    /// 반환: FnOutput (Ast | Data | Both)
    Fn {
        id: NodeIdentifier,
        fn_id: FunctionIdentifier,
        children: Vec<Self>,
        /// 자식 인덱스 → 파라미터 역할 명시적 매핑
        params: Vec<ParamBinding>,
    },
}

impl Expression {
    pub fn node_id(&self) -> Option<NodeIdentifier> {
        match self {
            Self::Binding(id) => Some(*id),
            Self::Literal(_) => None,
            Self::Logic { id, .. } => Some(*id),
            Self::Fn { id, .. } => Some(*id),
        }
    }
}

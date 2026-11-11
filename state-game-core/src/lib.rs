#![allow(unused)] // todo: delete
#![deny(unused_mut)]
#![warn(unused_imports)]

pub mod helper;
mod mod_metadata;

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identifier(pub Arc<str>);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Namespace(pub Arc<str>);

/// ModificationSpecifications defines a registry specification.
///
/// A specification provides metadata that determines how it participates
/// in the registry execution pipeline:
///
/// 1. base_priority: Defines phase ordering.
/// 2. use_mix_inside_priority: Defines ordering behavior among
///    specifications sharing the same base_priority.
/// 3. namespace: Defines the registry namespace. Duplicate namespaces
///    cause registry conflicts and loading will fail.
///
/// Execution model:
/// - Specifications are executed in ascending base_priority order.
/// - Execution is a sequential State transformation pipeline.
/// - The State produced by one specification becomes the input State
///   for the next.
/// - Results are NOT independently produced and merged later.
///   Composition IS sequential application.
/// - The final State of one base_priority phase becomes the input State
///   for the next phase.
///
/// Notes:
/// - Ordering among specifications sharing the same base_priority is
///   further defined by use_mix_inside_priority() and inside_priority().
/// - These metadata values are logically constant and must not change
///   across calls.
/// - They are not derived from runtime State and are treated as
///   immutable configuration.
pub trait ModificationSpecifications {
    /// Defines the phase in which this specification executes.
    ///
    /// Execution order by numeric priority:
    /// 1. Lower values run earlier.
    /// 2. base_priority < 0: pre-phase (runs before default)
    /// 3. base_priority = 0: default phase
    /// 4. base_priority > 0: post-phase (runs after default)
    ///
    /// Result production:
    /// - Each specification receives the current State and returns
    ///   a new State.
    /// - A specification may read or write any part of the State.
    /// - Preserving fields it does not intend to change is
    ///   RECOMMENDED for compatibility, but not enforced.
    /// - A specification may overwrite or discard changes made by
    ///   earlier specifications; this is the specification author's
    ///   responsibility.
    fn base_priority(&self) -> i64;

    /// Defines how this specification participates in ordering among
    /// specifications that share the same base_priority.
    ///
    /// Execution strategy:
    ///
    /// 1. use_mix_inside_priority = true
    ///    - Participates in the mixed chain.
    ///    - Mixed-chain specifications execute before non-mixed ones.
    ///    - Ordered by inside_priority().
    ///    - Specifications from different namespaces may be interleaved.
    ///    - If inside_priority values are identical, relative order is
    ///      NOT guaranteed.
    ///
    /// 2. use_mix_inside_priority = false
    ///    - Participates in the non-mixed chain.
    ///    - Executes after the mixed chain.
    ///    - Receives the State produced by the previous specification
    ///      in that chain.
    ///    - Relative order between non-mixed specifications is NOT
    ///      guaranteed.
    ///
    /// Determinism contract:
    /// - Whenever relative order is not guaranteed, the involved
    ///   specifications are REQUIRED to be commutative.
    /// - They must produce the same final State regardless of
    ///   execution order.
    /// - This requirement is not verified by the engine.
    /// - Violating this contract may result in non-deterministic
    ///   behavior when registration or iteration order changes.
    fn use_mix_inside_priority(&self) -> bool;

    /// Naming rule: snake_case.
    ///
    /// If multiple specifications share the same namespace,
    /// loading fails.
    fn namespace(&self) -> Namespace;
}

enum InputVariation {
    Select(u64),
    Number(InputVariationNumber),
    Text(String),
    Custom(InputVariationCustom)
}

enum InputVariationNumber {
    IntegerNumber(i64),
    RationalNumber(i64, i64),
    Float(f64),
    ComplexNumber(i64, i64),
}

struct InputVariationCustom {
    tag_content: Vec<String>, // namespace:snake_case
    value: InputVariationCustomValue
}

enum InputVariationCustomValue {
    Boolean(bool),
    Number(InputVariationNumber),
    Text(String),
    Vector(Vec<Self>),
    Map(HashMap<String, Self>),
}



pub trait StateTransition {
    fn transition(state, input: InputVariation) -> state;
}

/*
최종 외부 노출 함수
(State, Input) -> Option<State>를 만드는 함수는 Input을 검증하고, 올바르면 Some, 올바르지 않으면 None
State -> Vec<InputSpace>을 만드는 함수는 State에서 입력 가능한 모든 공간을 추출, 그리고 그 대상이 다른경우 다른 인덱스를 가짐
*/
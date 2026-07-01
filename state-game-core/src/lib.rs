mod bound;
pub mod helper;
mod mod_loader;

use serde_json::Value;

pub trait State: Send + Sync {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Identifier(pub String);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Namespace(pub String);

// =========================
// Input Layer
// =========================

pub trait InputSchema {
    fn identifier(&self) -> Identifier;
}

pub trait Input: Send + Sync {
    fn schema(&self) -> Identifier;
}

// =========================
// Execution Context
// =========================

pub struct Context {
    pub rng_seed: u64,
}

// =========================
// Engine Output
// =========================

pub enum StepResult {
    Next(Box<dyn State>),
    RequiresInput(InputSpace),
}

pub struct InputSpace {
    pub schemas: Vec<Box<dyn InputSchema>>,
}

// =========================
// Core Engine
// =========================

pub trait GameEngine {
    fn step(
        &self,
        state: Box<dyn State>,
        context: Context,
    ) -> StepResult;
}

// =========================
// Selection Strategy
// =========================

pub trait SelectionStrategy {
    fn select(
        &self,
        inputs: Vec<(Box<dyn Input>, f64)>,
        context: &Context,
    ) -> Box<dyn Input>;
}

// =========================
// Rule Module Registry
// =========================

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

    /// optional implementations
    fn input_providers(&self) -> &[&dyn InputProvider];
    fn input_generators(&self) -> &[&dyn InputGenerator];
    fn input_filters(&self) -> &[&dyn InputFilter];
    fn input_weights(&self) -> &[&dyn InputWeight];
    fn transformers(&self) -> &[&dyn StateTransformer];
    fn terminal_conditions(&self) -> &[&dyn TerminalCondition];
}

// =========================
// Input Pipeline
// =========================

pub trait InputProvider {
    fn provide(
        &self,
        state: &Box<dyn State>,
    ) -> Vec<Box<dyn InputSchema>>;
}

pub trait InputGenerator {
    fn generate(
        &self,
        schema: &Box<dyn InputSchema>,
    ) -> Box<dyn Iterator<Item = Box<dyn Input>>>;
}

pub trait InputFilter {
    fn allow(
        &self,
        state: &Box<dyn State>,
        input: &Box<dyn Input>,
    ) -> bool;
}

pub trait InputWeight {
    fn weight(
        &self,
        state: &Box<dyn State>,
        input: &Box<dyn Input>,
    ) -> f64;
}

// =========================
// State Transition
// =========================

pub trait StateTransformer {
    fn apply(
        &self,
        state: &Box<dyn State>,
        input: &Box<dyn Input>,
    ) -> Option<Box<dyn State>>;
}

// =========================
// Terminal Condition
// =========================

pub trait TerminalCondition {
    fn is_terminal(
        &self,
        state: &Box<dyn State>,
    ) -> bool;
}

// =========================
// Engine Pipeline (conceptual)
// =========================

impl dyn GameEngine {
    fn conceptual_flow(&self) {
        /*
        State
          ↓
        InputProvider
          ↓
        InputSchema
          ↓
        InputGenerator
          ↓
        Input
          ↓
        InputFilter
          ↓
        InputWeight
          ↓
        SelectionStrategy
          ↓
        StateTransformer
          ↓
        Next State
        */
    }
}


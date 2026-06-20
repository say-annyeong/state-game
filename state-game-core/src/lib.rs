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

/// ModificationSpecifications defines the game's registry.
/// It consists of three types of metadata and optional implementation methods.
/// The metadata fields are as follows:
/// 1. base_priority: Defines the priority.
/// 2. use_mix_inside_priority: Specifies whether the priority is shared with other registries.
/// 3. namespace: Defines the namespace. If duplicated, registry conflicts occur and loading will fail.
///
/// Notes:
/// These values are logically constant and must not change across calls.
/// They are not derived from runtime State and are treated as immutable configuration.
pub trait ModificationSpecifications {
    /// Base priority of this specification.
    ///
    /// Execution order by numeric priority:
    /// 1. Lower values run earlier.
    /// 2. base_priority < 0: pre-phase (runs before default)
    /// 3. base_priority = 0: default phase
    /// 4. base_priority > 0: post-phase (runs after default)
    ///
    /// Each base_priority phase is NOT a set of independent results that get merged.
    /// It is a single sequential fold: the State produced by one specification becomes
    /// the input State for the next. There is no separate "merge" or "compose" step —
    /// composition IS sequential application.
    ///
    /// Notes (when base_priority is equal):
    ///
    /// 1. Execution strategy (within the same base_priority phase):
    ///    - Specifications with use_mix_inside_priority = true run first, as a single
    ///      interleaved chain ordered by 'inside_priority'. Specifications from different
    ///      namespaces may be interleaved within this chain.
    ///      Note: If 'inside_priority' is identical, relative order between those
    ///      specifications is NOT guaranteed.
    ///    - Specifications with use_mix_inside_priority = false run after that, as a
    ///      second chain. Each such specification receives the State produced by the
    ///      previous one in this chain (mix chain's final output is the first input).
    ///      Note: The order between these specifications is NOT guaranteed.
    ///
    /// 2. Result production:
    ///    - Each specification receives the current State (output of the previous
    ///      specification in the chain, or the phase's initial State if it is first)
    ///      and returns a new State. A specification is free to read and write any
    ///      part of the State.
    ///    - Preserving fields it does not intend to change (rather than reconstructing
    ///      State from scratch) is RECOMMENDED for compatibility with other
    ///      specifications, but is NOT enforced. A specification may overwrite or
    ///      discard changes made by earlier specifications in the chain; this is the
    ///      specification author's responsibility.
    ///
    /// 3. Final result:
    ///    - The State produced by the last specification in the non-mix chain (or the
    ///      mix chain, if no non-mix specifications exist at this priority) becomes the
    ///      input State for the next base_priority phase.
    ///
    /// ## Determinism contract
    /// When relative order is "not guaranteed" (identical inside_priority, or identical
    /// base_priority within the non-mix chain), specifications occupying that tie are
    /// REQUIRED to be commutative with each other — i.e. produce the same final State
    /// regardless of which order they run in. This is NOT verified by the engine.
    /// Violating this contract results in non-deterministic behavior that may only
    /// surface when iteration/registration order happens to change.
    fn base_priority(&self) -> i64;
    fn use_mix_inside_priority(&self) -> bool;

    /// naming rule: snake_case
    /// If multiple specifications share the same namespace, loading fails.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {

    }
}

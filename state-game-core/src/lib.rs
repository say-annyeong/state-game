mod bound;

mod instruction_run;
mod helper;
// =========================
// Core Types
// =========================

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
pub trait ModificationSpecifications {
    /// Base priority of this specification.
    ///
    /// Execution order by numeric priority:
    /// 1. Lower values run earlier.
    /// 2. base_priority < 0: pre-phase (runs before default)
    /// 3. base_priority = 0: default phase
    /// 4. base_priority > 0: post-phase (runs after default)
    ///
    /// Notes (when base_priority is equal):
    ///
    /// 1. Execution strategy:
    ///    - Specifications with use_mix_inside_priority = true are executed first.
    ///      - Transitions and choosers from these specifications are merged
    ///        into a single sequence and ordered by inside_priority,
    ///        allowing interleaved execution.
    ///    - Specifications with use_mix_inside_priority = false are executed after that.
    ///      - Each specification is executed independently without interleaving.
    ///      - The execution order between these specifications is not guaranteed.
    ///
    /// 2. Result production:
    ///    - Each specification produces its own result regardless of the execution strategy.
    ///
    /// 3. Result composition:
    ///    - All results are always combined after execution.
    ///    - The composed result is used as the input for the next base_priority phase.
    ///
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

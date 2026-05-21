mod policy_graph;
mod bound;

mod state_game_compiler;
mod state_game_instruction;
mod state_game_virtual_machine;

use std::any::Any;
use std::collections::HashMap;

pub type Namespace = &'static str;
pub type Identifier = &'static str;

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
    fn transition(&self) -> &[&dyn Transition] { &[] }
    /// optional implementations
    fn chooser(&self) -> &[&dyn Chooser] { &[] }
    fn bound(&self) -> Bound { Bound::empty() }
}

trait ActionGenerator {
    fn enumerate(
        &self,
        state: &State,
        out: &mut InputAccumulator<Input>,
    );
}

trait Constraint {
    fn validate(
        &self,
        state: &State,
        input: &Input,
    ) -> bool;
}

trait Transition {
    fn apply(
        &self,
        state: &State,
        input: &Input,
        ctx: &ExecutionContext,
    ) -> Option<State>;
}

pub trait Transition {
    /// Execution order by numeric priority
    /// 1. Lower values run earlier.
    ///
    /// Notes (when priorities are equal):
    /// 1. Execution order is not guaranteed.
    fn inside_priority(&self) -> i64;
    /// naming rule: snake_case
    fn transition_identifier(&self) -> Identifier;
    /// todo
    fn transition(&self, state: State, bound: Bound) -> Vec<State>;
}

pub trait Chooser {
    /// Execution order by numeric priority
    /// 1. Lower values run earlier.
    ///
    /// Notes (when priorities are equal):
    /// 1. Execution order is not guaranteed.
    fn inside_priority(&self) -> i64;
    /// naming rule: snake_case
    fn chooser_identifier(&self) -> Identifier;
    /// todo
    fn choose(&self, set_state: &[State], input: &[Input]) -> Option<State>;
}

pub trait ValueType {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> ValueType for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct State {
    state: HashMap<(Namespace, Identifier), Value>,
}

pub struct Value {
    last_writer: Namespace,
    write_count: i64,
    value: Box<dyn ValueType>,
}

pub struct Input {
    namespace: Namespace,
    identifier: Identifier,
    value: Box<dyn ValueType>,
}

pub struct Bound {
    bound: BoundAbstractSyntaxTree
}

impl Bound {
    fn empty() -> Bound {
        Self { bound: BoundAbstractSyntaxTree::False }
    }
}

pub enum BoundAbstractSyntaxTree {
    And(Box<BoundAbstractSyntaxTree>, Box<BoundAbstractSyntaxTree>),
    Or(Box<BoundAbstractSyntaxTree>, Box<BoundAbstractSyntaxTree>),

    False
}

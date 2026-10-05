use std::{collections::HashMap, sync::Arc};

use crate::runtime_task::{
    instruction::{FunctionIdentifier, RuntimeTaskIdentifier, Slot},
};
use crate::runtime_task::types::RuntimeValue;

pub struct RuntimeTaskEvent {
    pub virtual_machine_identifier: RuntimeTaskIdentifier,
    pub virtual_machine_event_kind: RuntimeTaskEventKind,
}

pub enum RuntimeTaskEventKind {
    Log(RuntimeTaskLog),
    Trap(RuntimeTaskTrap),
    StateChange(StateChange),
    ExecutionFinished,
}

pub struct RuntimeTaskLog {
    pub level: RuntimeTaskLogLevel,
    pub message: String,
}

#[derive(Debug)]
pub enum RuntimeTaskLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    State,
    Dev,
}

#[derive(Clone, Debug)]
pub struct RuntimeTaskTrap {
    pub trapped_position: usize,
    pub reason: TrapReason,
}

/// Runtime-only failures — conditions the verifier cannot rule out statically.
#[derive(Clone, Debug)]
pub enum TrapReason {
    /// `UnwrapSome` on a `None` value.
    UnwrapNone,
    /// `UnwrapOk` on an `Err` value.
    UnwrapErrOnOk,
    /// `UnwrapErr` on an `Ok` value.
    UnwrapOkOnErr,
    /// Integer division or modulo by zero.
    DivisionByZero,
    /// Vector index out of bounds.
    IndexOutOfBounds {
        index: i64,
        length: usize,
    },
    /// `StringGetChar` index out of bounds.
    StringIndexOutOfBounds {
        index: i64,
        length: usize,
    },
    VerifierBug(String),
}

pub struct StateChange {
    pub identifier: String,
    pub old: Option<Arc<RuntimeValue>>,
    pub new: Option<Arc<RuntimeValue>>,
}

pub struct RuntimeTaskCallEvent {
    pub self_identifier: FunctionIdentifier,
    pub function_identifier: FunctionIdentifier,
    pub input: HashMap<Slot, Arc<RuntimeValue>>,
    pub output: HashMap<Slot, Arc<RuntimeValue>>,
}

impl RuntimeTaskCallEvent {
    pub fn new(
        self_identifier: FunctionIdentifier,
        function_identifier: FunctionIdentifier,
        input: HashMap<Slot, Arc<RuntimeValue>>,
        output: HashMap<Slot, Arc<RuntimeValue>>,
    ) -> Self {
        Self {
            self_identifier,
            function_identifier,
            input,
            output,
        }
    }
}

#[derive(Debug)]
pub enum RuntimeTaskYield {
    Call {
        function_identifier: FunctionIdentifier,
        inputs: HashMap<Slot, Arc<RuntimeValue>>,
        destination_slots: Vec<Slot>,
    },

    Finished,

    Return {
        function_identifier: FunctionIdentifier,
        outputs: Vec<Arc<RuntimeValue>>,
    },
}

use std::sync::mpsc::Sender;
use crate::state_game_instruction::{Instruction, Type, Value};

struct VirtualMachine {
    channel: Sender<VirtualMachineEvent>,
    execute_position: usize,
    instructions: Vec<Instruction>
}

impl VirtualMachine {
    fn new(channel: Sender<VirtualMachineEvent>, instructions: Vec<Instruction>) -> Self {
        Self { channel, execute_position: 0, instructions }
    }
}

enum VirtualMachineEvent {
    Log(VirtualMachineLog),
    Trap(VirtualMachineTrap),
    StateChange(StateChange),
    ExecutionFinished
}

struct VirtualMachineLog {
    level: VirtualMachineLogLevel,
    message: String,
}

enum VirtualMachineLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

struct VirtualMachineTrap {
    trapped_position: usize,
    label: Option<String>,
    reason: TrapReason
}

enum TrapReason {
    UnwrapNone,
    InvalidIdentifier(String),
    InvalidJump(String),
    TypeMismatch {
        expected: Type,
        actual: Type,
    }
}

struct StateChange {
    identifier: String,
    old: Option<Value>,
    new: Option<Value>
}
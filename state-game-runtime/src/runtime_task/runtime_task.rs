use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc},
};

use crossbeam_channel::{Receiver, Sender};
use dashmap::{DashMap, Entry};
use dashmap::try_result::TryResult;
use state_game_core::{Identifier, Namespace, helper::try_until};

use crate::runtime_task::{
    event::{
        StateChange, TrapReason, RuntimeTaskCallEvent, RuntimeTaskEvent, RuntimeTaskLog,
        RuntimeTaskLogLevel, RuntimeTaskTrap, RuntimeTaskYield
    },
    instruction::{FunctionIdentifier, Functions, Instruction, Literal, Slot, SpecialFunctions},
    types::{Type, Value},
};
use crate::runtime_task::event::RuntimeTaskEventKind;
use crate::runtime_task::instruction::RuntimeTaskIdentifier;
// ── Instruction pointer step ─────────────────────────────────────────────────

enum ExecutionResult {
    /// Advance by 1.
    Advance,
    /// Jump to an absolute position (already verified to be in-bounds).
    Goto(usize),

    YieldCall {
        function_identifier: FunctionIdentifier,
        inputs: HashMap<Slot, Arc<Value>>,
        destination_slots: Vec<Slot>,
    },

    ReturnDefinedCall {
        function_identifier: FunctionIdentifier,
        outputs: Vec<Arc<Value>>,
    }
}

// ── Virtual Machine ──────────────────────────────────────────────────────────

/// A RuntimeTask represents a single execution context.
///
/// Every instance shares the same execution engine, but owns its own
/// instruction stream, register state, and execution position.
///
/// Multiple RuntimeTask instances may execute concurrently.
/// When a user-defined function is invoked, the instance yields execution
/// to the scheduler, which is responsible for creating, scheduling,
/// and resuming other RuntimeTask instances.
///
/// Dependency management is intentionally outside the RuntimeTask.
/// Any required ordering, synchronization, or conflict resolution must
/// be enforced by the scheduler or caller. Race conditions or incorrect
/// execution order caused by missing dependencies are considered caller
/// errors rather than RuntimeTask implementation errors.
pub struct RuntimeTask {
    pub logger_sender: Sender<RuntimeTaskEvent>,
    pub scheduler_sender: Sender<RuntimeTaskCallEvent>,
    pub scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
    pub virtual_machine_identifier: RuntimeTaskIdentifier,
    pub instruction_pointer: usize,
    pub instructions: Arc<[Instruction]>,
    pub input_slots: Vec<(Slot, Arc<Value>)>,
    pub output_slots: Vec<(Slot, Arc<Value>)>,
    pub slots: Vec<Arc<Value>>,
    pub global_memory: Arc<DashMap<(Namespace, Identifier), Value>>,
    pub modification_namespace_list: Arc<[Namespace]>,
}

impl RuntimeTask {
    pub fn new(
        logger_sender: Sender<RuntimeTaskEvent>,
        scheduler_sender: Sender<RuntimeTaskCallEvent>,
        scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
        self_identifier: RuntimeTaskIdentifier,
        instructions: Arc<[Instruction]>,
        global_memory: Arc<DashMap<(Namespace, Identifier), Value>>,
        modification_namespace_list: Arc<[Namespace]>,
    ) -> Self {
        Self::with_instruction_pointer(
            logger_sender,
            scheduler_sender,
            scheduler_receiver,
            self_identifier,
            instructions,
            0,
            global_memory,
            modification_namespace_list,
        )
    }

    pub fn with_instruction_pointer(
        logger_sender: Sender<RuntimeTaskEvent>,
        scheduler_sender: Sender<RuntimeTaskCallEvent>,
        scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
        virtual_machine_identifier: RuntimeTaskIdentifier,
        instructions: Arc<[Instruction]>,
        instruction_pointer: usize,
        global_memory: Arc<DashMap<(Namespace, Identifier), Value>>,
        modification_namespace_list: Arc<[Namespace]>,
    ) -> Self {
        Self {
            logger_sender,
            scheduler_sender,
            scheduler_receiver,
            virtual_machine_identifier,
            instruction_pointer,
            instructions,
            slots: Vec::new(),
            global_memory,
            modification_namespace_list,
            input_slots: Vec::new(),
            output_slots: Vec::new(),
        }
    }

    pub fn call_function(
        &self,
        scheduler_sender: Sender<RuntimeTaskCallEvent>,
        scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
        virtual_machine_identifier: RuntimeTaskIdentifier,
        instruction_pointer: usize,
        instructions: Arc<[Instruction]>,
        input_slots: Vec<(Slot, Arc<Value>)>,
    ) -> Self {
        Self {
            logger_sender: self.logger_sender.clone(),
            scheduler_sender,
            scheduler_receiver,
            virtual_machine_identifier,
            instruction_pointer,
            instructions,
            slots: Vec::new(),
            global_memory: self.global_memory.clone(),
            modification_namespace_list: self.modification_namespace_list.clone(),
            input_slots,
            output_slots: Vec::new(),
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn emit(&self, event: RuntimeTaskEvent) {
        _ = self.logger_sender.send(event);
    }

    fn event_emit(&self, virtual_machine_event_kind: RuntimeTaskEventKind) {
        self.emit(
            RuntimeTaskEvent {
                virtual_machine_identifier: self.virtual_machine_identifier,
                virtual_machine_event_kind
            }
        )
    }

    fn log(&self, level: RuntimeTaskLogLevel, message: impl Into<String>) {
        self.emit(
            RuntimeTaskEvent {
                virtual_machine_identifier: self.virtual_machine_identifier,
                virtual_machine_event_kind: RuntimeTaskEventKind::Log(
                    RuntimeTaskLog {
                        level,
                        message: message.into(),
                    }
                )
            }
        );
    }

    fn trap(&self, reason: TrapReason) -> RuntimeTaskTrap {
        RuntimeTaskTrap {
            trapped_position: self.instruction_pointer,
            reason,
        }
    }

    fn slot_name(slot: Slot) -> String {
        format!("slot_{slot}")
    }

    /// Read a slot. Returns `Err(VerifierBug)` if the slot was never written —
    /// this indicates the instruction stream was not verified before execution.
    fn read(&self, slot: Slot) -> Result<Arc<Value>, RuntimeTaskTrap> {
        self.slots
            .get(slot as usize)
            .cloned()
            .ok_or_else(|| self.trap(TrapReason::VerifierBug("Unbound Slot".to_string())))
    }

    fn write(&mut self, slot: Slot, value: Arc<Value>) {
        let old = self.slots.get(slot as usize).cloned();
        self.slots.insert(slot as usize, value.clone());
        self.event_emit(
            RuntimeTaskEventKind::StateChange(
                StateChange {
                    identifier: Self::slot_name(slot),
                    old,
                    new: Some(value),
                }
            )
        );
    }

    // ── Main loop ─────────────────────────────────────────────────────────────

    pub fn run_until_yield(&mut self) -> Result<RuntimeTaskYield, RuntimeTaskTrap> {
        self.log(RuntimeTaskLogLevel::Info, "Virtual Machine Resume");

        while self.instruction_pointer < self.instructions.len() {
            let instr = self.instructions[self.instruction_pointer].clone();

            match self.execute(&instr) {
                Ok(ExecutionResult::Advance) => {
                    self.instruction_pointer += 1;
                }

                Ok(ExecutionResult::Goto(target)) => {
                    self.instruction_pointer = target;
                }

                Ok(ExecutionResult::YieldCall {
                    function_identifier,
                    inputs,
                    destination_slots,
                }) => {
                    return Ok(RuntimeTaskYield::Call {
                        function_identifier,
                        inputs,
                        destination_slots,
                    });
                }

                Ok(ExecutionResult::ReturnDefinedCall { function_identifier, outputs }) => {
                    return Ok(RuntimeTaskYield::Return {
                        function_identifier,
                        outputs,
                    })
                }

                Err(trap) => {
                    self.event_emit(RuntimeTaskEventKind::Trap(trap.clone()));

                    self.event_emit(RuntimeTaskEventKind::ExecutionFinished);

                    return Err(trap);
                }
            }
        }

        self.log(RuntimeTaskLogLevel::Info, "Virtual Machine Halt");

        self.event_emit(RuntimeTaskEventKind::ExecutionFinished);

        Ok(RuntimeTaskYield::Finished)
    }

    pub fn resume_call(
        &mut self,
        values: HashMap<Slot, Arc<Value>>,
    ) -> Result<(), RuntimeTaskTrap> {
        for (slot, value) in values {
            self.slots.insert(slot as usize, value);
        }

        self.instruction_pointer += 1;

        Ok(())
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn execute(&mut self, instr: &Instruction) -> Result<ExecutionResult, RuntimeTaskTrap> {
        match instr {
            // ── Bind ──────────────────────────────────────────────────────────
            Instruction::Bind {
                slot,
                type_name,
                value,
            } => {
                let v = parse_literal(type_name, value).ok_or_else(|| {
                    self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))
                })?;
                self.write(*slot, Arc::new(v));
                Ok(ExecutionResult::Advance)
            }

            // ── Call ──────────────────────────────────────────────────────────
            Instruction::Call {
                function_name,
                inputs,
                output,
            } => {
                let args = inputs
                    .iter()
                    .map(|s| self.read(*s))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = self.dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(ExecutionResult::Advance)
            }

            // ── Jump ──────────────────────────────────────────────────────────
            Instruction::Jump { target_position } => {
                self.log(
                    RuntimeTaskLogLevel::Debug,
                    format!("Jump to {}", target_position),
                );
                Ok(ExecutionResult::Goto(*target_position))
            }

            // ── ConditionalJump ───────────────────────────────────────────────
            Instruction::ConditionalJump {
                condition,
                true_target_position,
                false_target_position,
            } => {
                let v = self.read(*condition)?;
                let b = match &*v {
                    Value::Boolean(b) => *b,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())));
                    }
                };
                self.log(
                    RuntimeTaskLogLevel::Debug,
                    format!(
                        "Jump positions. true: {}, false: {}",
                        true_target_position, false_target_position
                    ),
                );
                Ok(ExecutionResult::Goto(if b {
                    self.log(
                        RuntimeTaskLogLevel::Debug,
                        format!("Jump to {}", true_target_position),
                    );
                    *true_target_position
                } else {
                    self.log(
                        RuntimeTaskLogLevel::Debug,
                        format!("Jump to {}", false_target_position),
                    );
                    *false_target_position
                }))
            }

            // ── SpecialCall ───────────────────────────────────────────────────
            Instruction::SpecialCall {
                function_name,
                inputs,
                output,
            } => {
                let args = inputs
                    .iter()
                    .map(|s| self.read(*s))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = self.special_dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(ExecutionResult::Advance)
            }

            Instruction::DefinedCall {
                function_identifier,
                inputs,
                destination_slots,
            } => {
                let resolved_inputs = {
                    let mut result = HashMap::new();
                    for slot in inputs {
                        let read = self.read(*slot)?;
                        result.insert(*slot, read);
                    }
                    result
                };

                Ok(ExecutionResult::YieldCall {
                    function_identifier: *function_identifier,
                    inputs: resolved_inputs,
                    destination_slots: destination_slots.clone(),
                })
            }
            Instruction::ReturnDefinedCall { function_identifier, outputs } => {
                let outputs = match outputs.iter().map(|slot| self.read(*slot)).collect() {
                    Ok(outputs) => outputs,
                    Err(error) => return Err(error),
                };
                Ok(ExecutionResult::ReturnDefinedCall {
                    function_identifier: *function_identifier,
                    outputs
                })
            }
        }
    }

    // ── Function dispatch ─────────────────────────────────────────────────────

    fn dispatch(&self, func: Functions, args: &[Arc<Value>]) -> Result<Value, RuntimeTaskTrap> {
        // Convenience extractors — return VerifierBug trap on wrong variant.
        macro_rules! int {
            ($v:expr) => {
                match &*$v {
                    Value::Integer(n) => *n,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! float {
            ($v:expr) => {
                match &*$v {
                    Value::Float(f) => *f,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! bool_ {
            ($v:expr) => {
                match &*$v {
                    Value::Boolean(b) => *b,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! str_ {
            ($v:expr) => {
                match &*$v {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! vec_ {
            ($v:expr) => {
                match &*$v {
                    Value::Vector(v) => v.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }

        match func {
            // ── Integer arithmetic ────────────────────────────────────────────
            Functions::AddInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_add(int!(args[1]))))
            }
            Functions::SubInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_sub(int!(args[1]))))
            }
            Functions::MulInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_mul(int!(args[1]))))
            }
            Functions::DivInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = int!(args[1]);
                if rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Value::Integer(int!(args[0]) / rhs))
            }
            Functions::ModInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = int!(args[1]);
                if rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Value::Integer(int!(args[0]) % rhs))
            }
            Functions::PowInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let base = int!(args[0]);
                let exp = int!(args[1]);
                let exp_u = u32::try_from(exp).unwrap_or(0);
                Ok(Value::Integer(base.wrapping_pow(exp_u)))
            }

            // ── Float arithmetic ──────────────────────────────────────────────
            Functions::AddFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) + float!(args[1])))
            }
            Functions::SubFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) - float!(args[1])))
            }
            Functions::MulFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) * float!(args[1])))
            }
            Functions::DivFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) / float!(args[1])))
            }
            Functions::PowFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]).powf(float!(args[1]))))
            }

            // ── Integer comparisons ───────────────────────────────────────────
            Functions::EqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) == int!(args[1])))
            }
            Functions::NotEqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) != int!(args[1])))
            }
            Functions::GreaterThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) > int!(args[1])))
            }
            Functions::LessThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) < int!(args[1])))
            }

            // ── Float comparisons ─────────────────────────────────────────────
            Functions::GreaterThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(float!(args[0]) > float!(args[1])))
            }
            Functions::LessThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(float!(args[0]) < float!(args[1])))
            }

            // ── Boolean logic ─────────────────────────────────────────────────
            Functions::Not => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(!bool_!(args[0])))
            }
            Functions::And => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) && bool_!(args[1])))
            }
            Functions::Or => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) || bool_!(args[1])))
            }
            Functions::Xor => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) ^ bool_!(args[1])))
            }

            // ── String operations ─────────────────────────────────────────────
            Functions::EqualString => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(str_!(args[0]) == str_!(args[1])))
            }
            Functions::StringLength => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(str_!(args[0]).chars().count() as i64))
            }
            Functions::StringGetChar => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let s = str_!(args[0]);
                let idx = int!(args[1]);
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let i = usize::try_from(idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::StringIndexOutOfBounds {
                            index: idx,
                            length: len,
                        })
                    })?;
                Ok(Value::Char(chars[i]))
            }

            // ── Vector get ────────────────────────────────────────────────────
            Functions::VectorGetInteger
            | Functions::VectorGetFloat
            | Functions::VectorGetString
            | Functions::VectorGetChar
            | Functions::VectorGetBoolean => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let v = vec_!(args[0]);
                let idx = int!(args[1]);
                let len = v.len();
                let i = usize::try_from(idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::IndexOutOfBounds {
                            index: idx,
                            length: len,
                        })
                    })?;
                Ok(v[i].clone())
            }

            // ── Vector init ───────────────────────────────────────────────────
            Functions::VectorInitInteger
            | Functions::VectorInitFloat
            | Functions::VectorInitString
            | Functions::VectorInitChar
            | Functions::VectorInitBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Vector(vec![(*args[0]).clone()]))
            }

            // ── Vector push ───────────────────────────────────────────────────
            Functions::VectorPushInteger
            | Functions::VectorPushFloat
            | Functions::VectorPushString
            | Functions::VectorPushChar
            | Functions::VectorPushBoolean => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let mut v = vec_!(args[0]);
                v.push((*args[1]).clone());
                Ok(Value::Vector(v))
            }

            // ── Vector pop ────────────────────────────────────────────────────
            Functions::VectorPopInteger
            | Functions::VectorPopFloat
            | Functions::VectorPopString
            | Functions::VectorPopChar
            | Functions::VectorPopBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let mut v = vec_!(args[0]);
                if v.is_empty() {
                    return Err(self.trap(TrapReason::IndexOutOfBounds {
                        index: -1,
                        length: 0,
                    }));
                }
                v.pop();
                Ok(Value::Vector(v))
            }

            // ── Option / Result inspection ────────────────────────────────────
            Functions::IsSome => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Option(Some(_)))))
            }
            Functions::IsNone => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Option(None))))
            }
            Functions::IsOk => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Result(Ok(_)))))
            }
            Functions::IsErr => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Result(Err(_)))))
            }

            // ── UnwrapSome ────────────────────────────────────────────────────
            Functions::UnwrapSomeInteger
            | Functions::UnwrapSomeFloat
            | Functions::UnwrapSomeString
            | Functions::UnwrapSomeChar
            | Functions::UnwrapSomeBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Option(Some(inner)) => Ok(*inner.clone()),
                    Value::Option(None) => Err(self.trap(TrapReason::UnwrapNone)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapOk ──────────────────────────────────────────────────────
            Functions::UnwrapOkInteger
            | Functions::UnwrapOkFloat
            | Functions::UnwrapOkString
            | Functions::UnwrapOkChar
            | Functions::UnwrapOkBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Result(Ok(inner)) => Ok(*inner.clone()),
                    Value::Result(Err(_)) => Err(self.trap(TrapReason::UnwrapErrOnOk)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapErr ─────────────────────────────────────────────────────
            Functions::UnwrapErrInteger
            | Functions::UnwrapErrFloat
            | Functions::UnwrapErrString
            | Functions::UnwrapErrChar
            | Functions::UnwrapErrBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Result(Err(inner)) => Ok(*inner.clone()),
                    Value::Result(Ok(_)) => Err(self.trap(TrapReason::UnwrapOkOnErr)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }
        }
    }

    // ── Special function dispatch ─────────────────────────────────────────────

    fn special_dispatch(
        &mut self,
        special_functions: SpecialFunctions,
        arguments: &[Arc<Value>],
    ) -> Result<Value, RuntimeTaskTrap> {
        macro_rules! int {
            ($v:expr) => {
                match &*$v {
                    Value::Integer(n) => *n,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! str_ {
            ($v:expr) => {
                match &*$v {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }

        match special_functions {
            SpecialFunctions::ReadGlobalMemoryInteger
            | SpecialFunctions::ReadGlobalMemoryFloat
            | SpecialFunctions::ReadGlobalMemoryString
            | SpecialFunctions::ReadGlobalMemoryChar
            | SpecialFunctions::ReadGlobalMemoryBoolean => {
                if arguments.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                let key = (Namespace(namespace), Identifier(identifier));
                let value = match self.global_memory.try_get(&key) {
                    TryResult::Present(value) => {
                        Value::Result(Ok(Box::new(value.value().clone())))
                    }
                    TryResult::Absent => {
                        self.log(RuntimeTaskLogLevel::Debug, "read to global memory empty space".to_string());
                        Value::Result(Err(Box::new(Value::String("absent".to_string()))))
                    }
                    TryResult::Locked => {
                        self.log(RuntimeTaskLogLevel::Warn, "global memory is blocking is try read".to_string());
                        Value::Result(Err(Box::new(Value::String("global memory is blocking".to_string()))))
                    }
                };
                Ok(value)
            }

            SpecialFunctions::WriteGlobalMemoryInteger
            | SpecialFunctions::WriteGlobalMemoryFloat
            | SpecialFunctions::WriteGlobalMemoryString
            | SpecialFunctions::WriteGlobalMemoryChar
            | SpecialFunctions::WriteGlobalMemoryBoolean => {
                if arguments.len() != 3 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                let input = arguments[2].clone();
                let key = (Namespace(namespace), Identifier(identifier));
                let value = match self.global_memory.try_entry(key) {
                    Some(Entry::Occupied(mut entry)) => {
                        entry.insert(input.deref().clone());
                        Value::Result(Ok(Box::new(Value::Void)))
                    }
                    Some(Entry::Vacant(entry)) => {
                        entry.insert(input.deref().clone());
                        Value::Result(Ok(Box::new(Value::Void)))
                    }
                    None => {
                        self.log(RuntimeTaskLogLevel::Warn, "global memory is blocking is try write".to_string());
                        Value::Result(Err(Box::new(Value::String("global memory is blocking".to_string()))))
                    }
                };
                Ok(value)
            }

            SpecialFunctions::GetInstructionPosition => {
                Ok(Value::Integer(self.instruction_pointer as i64))
            }

            SpecialFunctions::GetModificationNamespaceList => Ok(Value::Vector(
                self.modification_namespace_list
                    .iter()
                    .map(|ns| Value::String(ns.0.clone()))
                    .collect(),
            )),
        }
    }
}

// ── Literal conversion ────────────────────────────────────────────────────────

/// Returns `None` if the literal variant does not match the declared type,
/// which indicates the instruction stream was not verified before execution.
fn parse_literal(ty: &Type, lit: &Literal) -> Option<Value> {
    match (ty, lit) {
        (Type::Integer, Literal::Integer(n)) => Some(Value::Integer(*n)),
        (Type::Float, Literal::Float(f)) => Some(Value::Float(*f)),
        (Type::String, Literal::String(s)) => Some(Value::String(s.clone())),
        (Type::Char, Literal::Char(c)) => Some(Value::Char(*c)),
        (Type::Boolean, Literal::Boolean(b)) => Some(Value::Boolean(*b)),
        _ => None,
    }
}

// ── Logger ────────────────────────────────────────────────────────────────────

pub struct Logger {
    channel_receiver: Receiver<RuntimeTaskEvent>,
    verbose: bool,
}

impl Logger {
    pub fn new(rx: Receiver<RuntimeTaskEvent>) -> Self {
        Self {
            channel_receiver: rx,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, v: bool) -> Self {
        self.verbose = v;
        self
    }

    pub fn run(&self) {
        while let Ok(event) = self.channel_receiver.recv() {
            match event.virtual_machine_event_kind {
                RuntimeTaskEventKind::Log(l) => {
                    let line = format!("[{:?}] {}", l.level, l.message);
                    match l.level {
                        RuntimeTaskLogLevel::Warn | RuntimeTaskLogLevel::Error => {
                            eprintln!("{line}")
                        }
                        _ => println!("{line}"),
                    }
                }
                RuntimeTaskEventKind::Trap(t) => {
                    eprintln!("[TRAP @ {}] {:?}", t.trapped_position, t.reason);
                }
                RuntimeTaskEventKind::StateChange(s) if self.verbose => {
                    println!("[STATE] {}: {:?} -> {:?}", s.identifier, s.old, s.new);
                }
                RuntimeTaskEventKind::StateChange(_) => {}
                RuntimeTaskEventKind::ExecutionFinished => {
                    println!("[VM] finished");
                    break;
                }
            }
        }
    }
}

use std::{collections::HashMap, ops::Deref, sync::Arc};
use std::num::{ParseFloatError, ParseIntError};
use std::str::ParseBoolError;
use crossbeam_channel::{Receiver, Sender};
use dashmap::{DashMap, Entry, try_result::TryResult};

use state_game_core::{Identifier, Namespace, helper::try_until};

use crate::{
    persistent_vector::InnerPersistentVector,
    runtime_task::{
        event::{
            RuntimeTaskCallEvent, RuntimeTaskEvent, RuntimeTaskEventKind, RuntimeTaskLog,
            RuntimeTaskLogLevel, RuntimeTaskTrap, RuntimeTaskYield, StateChange, TrapReason,
        },
        instruction::{
            FunctionIdentifier, Functions, Instruction, Literal, RuntimeTaskIdentifier, Slot,
            SpecialFunctions,
        },
    },
};
use crate::runtime_task::instruction::ConcreteInstruction;
use crate::runtime_task::types::{PrimitiveType, RuntimeValue};

// ── Instruction pointer step ─────────────────────────────────────────────────

enum ExecutionResult {
    /// Advance by 1.
    Advance,
    /// Jump to an absolute position (already verified to be in-bounds).
    Goto(usize),

    YieldCall {
        function_identifier: FunctionIdentifier,
        inputs: HashMap<Slot, Arc<RuntimeValue>>,
        destination_slots: Vec<Slot>,
    },

    ReturnDefinedCall {
        function_identifier: FunctionIdentifier,
        outputs: Vec<Arc<RuntimeValue>>,
    },
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
///
/// The RuntimeTask assumes that every instruction it receives has already
/// been fully type-checked and validated by the Verifier. It therefore
/// trusts that each instruction is well-formed and that all types involved
/// during execution are consistent with the verified program. The
/// RuntimeTask does not perform runtime type validation of instructions,
/// their type relationships, or Slot assignments.
///
/// The Verifier is responsible for determining the Slot layout and the
/// number of Slots required by the program. The RuntimeTask initializes
/// all Slots before execution and subsequently only reads from or replaces
/// their values. A Slot has a fixed type determined by the verified
/// program, while the value stored in a Slot may change during execution.
///
/// Any undefined behavior resulting from the Verifier failing to validate
/// an instruction correctly, accepting an invalid type, providing
/// inconsistent type information, or producing an invalid Slot layout is
/// outside the responsibility of the RuntimeTask.
///
/// All Slot indices must be contiguous. Using non-contiguous Slot indices
/// results in undefined behavior.
pub struct RuntimeTask<'a> {
    pub logger_sender: Sender<RuntimeTaskEvent>,
    pub scheduler_sender: Sender<RuntimeTaskCallEvent>,
    pub scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
    pub virtual_machine_identifier: RuntimeTaskIdentifier,
    pub instruction_pointer: usize,
    pub instructions: Arc<[ConcreteInstruction<'a>]>,
    pub input_slots: Vec<Arc<RuntimeValue>>,
    pub output_slots: Vec<Arc<RuntimeValue>>,
    pub slots: Vec<Arc<RuntimeValue>>,
    pub global_memory: Arc<DashMap<(Namespace, Identifier), RuntimeValue>>,
    pub modification_namespace_list: Arc<[Namespace]>,
}

/*
use std::collections::HashMap;

pub struct TypeInterner {
    types: Vec<ConcreteType>,
    map: HashMap<ConcreteType, TypeId>,
}

impl TypeInterner {
    pub fn intern(&mut self, ty: ConcreteType) -> TypeId {
        if let Some(id) = self.map.get(&ty) {
            return *id;
        }

        let id = TypeId(self.types.len() as u32);

        self.types.push(ty.clone());
        self.map.insert(ty, id);

        id
    }

    pub fn get(&self, id: TypeId) -> &ConcreteType {
        &self.types[id.0 as usize]
    }
}
 */

impl<'a> RuntimeTask<'a> {
    pub fn new(
        logger_sender: Sender<RuntimeTaskEvent>,
        scheduler_sender: Sender<RuntimeTaskCallEvent>,
        scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
        self_identifier: RuntimeTaskIdentifier,
        instructions: Arc<[ConcreteInstruction<'a>]>,
        global_memory: Arc<DashMap<(Namespace, Identifier), RuntimeValue>>,
        modification_namespace_list: Arc<[Namespace]>,
        slots_size: usize,
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
            slots_size,
        )
    }

    pub fn with_instruction_pointer(
        logger_sender: Sender<RuntimeTaskEvent>,
        scheduler_sender: Sender<RuntimeTaskCallEvent>,
        scheduler_receiver: Receiver<RuntimeTaskCallEvent>,
        virtual_machine_identifier: RuntimeTaskIdentifier,
        instructions: Arc<[ConcreteInstruction<'a>]>,
        instruction_pointer: usize,
        global_memory: Arc<DashMap<(Namespace, Identifier), RuntimeValue>>,
        modification_namespace_list: Arc<[Namespace]>,
        slots_size: usize,
    ) -> Self {
        Self {
            logger_sender,
            scheduler_sender,
            scheduler_receiver,
            virtual_machine_identifier,
            instruction_pointer,
            instructions,
            slots: vec![Arc::new(RuntimeValue::Uninitialized); slots_size],
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
        instructions: Arc<[ConcreteInstruction<'a>]>,
        input_slots: Vec<Arc<RuntimeValue>>,
        slots_size: usize,
    ) -> Self {
        Self {
            logger_sender: self.logger_sender.clone(),
            scheduler_sender,
            scheduler_receiver,
            virtual_machine_identifier,
            instruction_pointer,
            instructions,
            slots: vec![Arc::new(RuntimeValue::Uninitialized); slots_size],
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
        self.emit(RuntimeTaskEvent {
            virtual_machine_identifier: self.virtual_machine_identifier,
            virtual_machine_event_kind,
        })
    }

    fn log(&self, level: RuntimeTaskLogLevel, message: impl Into<String>) {
        self.emit(RuntimeTaskEvent {
            virtual_machine_identifier: self.virtual_machine_identifier,
            virtual_machine_event_kind: RuntimeTaskEventKind::Log(RuntimeTaskLog {
                level,
                message: message.into(),
            }),
        });
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
    fn read(&self, slot: Slot) -> Result<Arc<RuntimeValue>, RuntimeTaskTrap> {
        self.slots
            .get(slot as usize)
            .cloned()
            .ok_or_else(|| self.trap(TrapReason::VerifierBug("Unbound Slot".to_string())))
    }

    fn write(&mut self, slot: Slot, value: Arc<RuntimeValue>) {
        let index = slot as usize;

        if let Some(old) = self.slots.get_mut(index) {
            let old_value = Some(old.clone());
            *old = value.clone();

            self.event_emit(RuntimeTaskEventKind::StateChange(StateChange {
                identifier: Self::slot_name(slot),
                old: old_value,
                new: Some(value),
            }));
        } else if index == self.slots.len() {
            self.slots.push(value.clone());

            self.event_emit(RuntimeTaskEventKind::StateChange(StateChange {
                identifier: Self::slot_name(slot),
                old: None,
                new: Some(value),
            }));
        }
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

                Ok(ExecutionResult::ReturnDefinedCall {
                    function_identifier,
                    outputs,
                }) => {
                    return Ok(RuntimeTaskYield::Return {
                        function_identifier,
                        outputs,
                    });
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
        values: HashMap<Slot, Arc<RuntimeValue>>,
    ) -> Result<(), RuntimeTaskTrap> {
        for (slot, value) in values {
            self.slots.insert(slot as usize, value);
        }

        self.instruction_pointer += 1;

        Ok(())
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn execute(&mut self, instr: &ConcreteInstruction) -> Result<ExecutionResult, RuntimeTaskTrap> {
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
                ..
            } => {
                let args = inputs
                    .iter()
                    .map(|s| self.read(*s))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = self.dispatch(*function_name, &args)?;
                self.write(*output, result);
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
                    RuntimeValue::Boolean(b) => *b,
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
                ..
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
                outputs,
                ..
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
                    destination_slots: outputs.clone(),
                })
            }
            Instruction::ReturnDefinedCall {
                function_identifier,
                outputs,
            } => {
                let outputs = match outputs.iter().map(|slot| self.read(*slot)).collect() {
                    Ok(outputs) => outputs,
                    Err(error) => return Err(error),
                };
                Ok(ExecutionResult::ReturnDefinedCall {
                    function_identifier: *function_identifier,
                    outputs,
                })
            }
        }
    }

    // ── Function dispatch ─────────────────────────────────────────────────────

    fn dispatch(&self, func: Functions, args: &[Arc<RuntimeValue>]) -> Result<Arc<RuntimeValue>, RuntimeTaskTrap> {
        match func {
            // ── Integer arithmetic ────────────────────────────────────────────
            Functions::AddInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].integer().wrapping_add(*args[1].integer()))))
            }
            Functions::SubInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].integer().wrapping_sub(*args[1].integer()))))
            }
            Functions::MulInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].integer().wrapping_mul(*args[1].integer()))))
            }
            Functions::DivInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = args[1].integer();
                if *rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].integer() / rhs)))
            }
            Functions::ModInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = args[1].integer();
                if *rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].integer() % rhs)))
            }
            Functions::PowInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let base = args[0].integer();
                let exp = args[1].integer();
                let exp_u = u32::try_from(*exp).unwrap_or(0);
                Ok(Arc::new(RuntimeValue::Integer(base.wrapping_pow(exp_u))))
            }

            // ── Float arithmetic ──────────────────────────────────────────────
            Functions::AddFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Float(args[0].float() + args[1].float())))
            }
            Functions::SubFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Float(args[0].float() - args[1].float())))
            }
            Functions::MulFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Float(args[0].float() * args[1].float())))
            }
            Functions::DivFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Float(args[0].float() / args[1].float())))
            }
            Functions::PowFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Float(args[0].float().powf(*args[1].float()))))
            }

            // ── Integer comparisons ───────────────────────────────────────────
            Functions::EqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].integer() == args[1].integer())))
            }
            Functions::NotEqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].integer() != args[1].integer())))
            }
            Functions::GreaterThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].integer() > args[1].integer())))
            }
            Functions::LessThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].integer() < args[1].integer())))
            }

            // ── Float comparisons ─────────────────────────────────────────────
            Functions::GreaterThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].float() > args[1].float())))
            }
            Functions::LessThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].float() < args[1].float())))
            }

            // ── Boolean logic ─────────────────────────────────────────────────
            Functions::Not => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(!args[0].boolean())))
            }
            Functions::And => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(*args[0].boolean() && *args[1].boolean())))
            }
            Functions::Or => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(*args[0].boolean() || *args[1].boolean())))
            }
            Functions::Xor => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(*args[0].boolean() ^ *args[1].boolean())))
            }

            // ── String operations ─────────────────────────────────────────────
            Functions::EqualString => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(args[0].string() == args[1].string())))
            }
            Functions::StringLength => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Integer(args[0].string().chars().count() as i64)))
            }
            Functions::StringGetChar => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let s = args[0].string();
                let idx = args[1].integer();
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let i = usize::try_from(*idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::StringIndexOutOfBounds {
                            index: *idx,
                            length: len,
                        })
                    })?;
                Ok(Arc::new(RuntimeValue::Char(chars[i])))
            }

            // ── Vector get ────────────────────────────────────────────────────
            Functions::VectorGet => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let v = args[0].vector();
                let idx = args[1].integer();
                let len = v.len();
                let i = usize::try_from(*idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::IndexOutOfBounds {
                            index: *idx,
                            length: len,
                        })
                    })?;
                let boxed = v.get(i).map(|arc| Arc::new((*arc).clone()));
                Ok(Arc::new(RuntimeValue::Option(boxed)))
            }

            // ── Vector init ───────────────────────────────────────────────────
            Functions::VectorNew => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let vector = InnerPersistentVector::new();
                let result = vector.push(args[0].clone());
                Ok(Arc::new(RuntimeValue::Vector(result)))
            }

            // ── Vector push ───────────────────────────────────────────────────
            Functions::VectorPush => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let v = args[0].vector();
                let v = v.push((args[1]).clone());
                Ok(Arc::new(RuntimeValue::Vector(v)))
            }

            // ── Vector pop ────────────────────────────────────────────────────
            Functions::VectorPop => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let v = args[0].vector();
                if v.is_empty() {
                    return Err(self.trap(TrapReason::IndexOutOfBounds {
                        index: -1,
                        length: 0,
                    }));
                }
                let v = v.pop().unwrap();
                Ok(Arc::new(RuntimeValue::Vector(v)))
            }

            // ── Option / Result inspection ────────────────────────────────────
            Functions::IsSome => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(matches!(&*args[0], RuntimeValue::Option(Some(_))))))
            }
            Functions::IsNone => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(matches!(&*args[0], RuntimeValue::Option(None)))))
            }
            Functions::IsOk => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(matches!(&*args[0], RuntimeValue::Result(Ok(_))))))
            }
            Functions::IsErr => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Arc::new(RuntimeValue::Boolean(matches!(&*args[0], RuntimeValue::Result(Err(_))))))
            }

            // ── UnwrapSome ────────────────────────────────────────────────────
            Functions::UnwrapSome => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    RuntimeValue::Option(Some(inner)) => Ok(inner.clone()),
                    RuntimeValue::Option(None) => Err(self.trap(TrapReason::UnwrapNone)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapOk ──────────────────────────────────────────────────────
            Functions::UnwrapOk => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    RuntimeValue::Result(Ok(inner)) => Ok(inner.clone()),
                    RuntimeValue::Result(Err(_)) => Err(self.trap(TrapReason::UnwrapErrOnOk)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapErr ─────────────────────────────────────────────────────
            Functions::UnwrapErr => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    RuntimeValue::Result(Err(inner)) => Ok(inner.clone()),
                    RuntimeValue::Result(Ok(_)) => Err(self.trap(TrapReason::UnwrapOkOnErr)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }
            Functions::StringCombine => {
                let string1 = args[0].string();
                let string2 = args[1].string();
                let result: String = string1.chars().chain(string2.chars()).collect();
                Ok(Arc::new(RuntimeValue::String(Arc::from(result))))
            }
            Functions::StringToInteger => {
                let string = args[0].string();
                let parse: Result<i64, String> = string.parse().map_err(|error: ParseIntError| error.to_string());
                let result = parse.map(|value| Arc::new(RuntimeValue::Integer(value))).map_err(|error| Arc::new(RuntimeValue::String(Arc::from(error))));
                Ok(Arc::new(RuntimeValue::Result(result)))
            }
            Functions::StringToFloat => {
                let string = args[0].string();
                let parse: Result<f64, String> = string.parse().map_err(|error: ParseFloatError| error.to_string());
                let result = parse.map(|value| Arc::new(RuntimeValue::Float(value))).map_err(|error| Arc::new(RuntimeValue::String(Arc::from(error))));
                Ok(Arc::new(RuntimeValue::Result(result)))
            }
            Functions::StringToBoolean => {
                let string = args[0].string();
                let parse: Result<bool, String> = string.parse().map_err(|error: ParseBoolError| error.to_string());
                let result = parse.map(|value| Arc::new(RuntimeValue::Boolean(value))).map_err(|error| Arc::new(RuntimeValue::String(Arc::from(error))));
                Ok(Arc::new(RuntimeValue::Result(result)))
            }
        }
    }

    // ── Special function dispatch ─────────────────────────────────────────────

    fn special_dispatch(
        &mut self,
        special_functions: SpecialFunctions,
        arguments: &[Arc<RuntimeValue>],
    ) -> Result<RuntimeValue, RuntimeTaskTrap> {
        match special_functions {
            SpecialFunctions::ReadGlobalMemory => {
                if arguments.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = arguments[0].string();
                let identifier = arguments[1].string();
                let key = (Namespace(namespace.clone()), Identifier(identifier.clone()));
                let value = match self.global_memory.try_get(&key) {
                    TryResult::Present(value) => RuntimeValue::Result(Ok(Arc::new(value.value().clone()))),
                    TryResult::Absent => {
                        self.log(
                            RuntimeTaskLogLevel::Debug,
                            "read to global memory empty space".to_string(),
                        );
                        RuntimeValue::Result(Err(Arc::new(RuntimeValue::String(Arc::from("absent")))))
                    }
                    TryResult::Locked => {
                        self.log(
                            RuntimeTaskLogLevel::Warn,
                            "global memory is blocking is try read".to_string(),
                        );
                        RuntimeValue::Result(Err(Arc::new(RuntimeValue::String(
                            Arc::from("global memory is blocking"),
                        ))))
                    }
                };
                Ok(value)
            }

            SpecialFunctions::WriteGlobalMemory => {
                if arguments.len() != 3 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = arguments[0].string();
                let identifier = arguments[1].string();
                let input = arguments[2].clone();
                let key = (Namespace(namespace.clone()), Identifier(identifier.clone()));
                let value = match self.global_memory.try_entry(key) {
                    Some(Entry::Occupied(mut entry)) => {
                        entry.insert(input.deref().clone());
                        RuntimeValue::Result(Ok(Arc::new(RuntimeValue::Unit)))
                    }
                    Some(Entry::Vacant(entry)) => {
                        entry.insert(input.deref().clone());
                        RuntimeValue::Result(Ok(Arc::new(RuntimeValue::Unit)))
                    }
                    None => {
                        self.log(
                            RuntimeTaskLogLevel::Warn,
                            "global memory is blocking is try write".to_string(),
                        );
                        RuntimeValue::Result(Err(Arc::new(RuntimeValue::String(
                            Arc::from("global memory is blocking"),
                        ))))
                    }
                };
                Ok(value)
            }

            SpecialFunctions::GetInstructionPosition => {
                Ok(RuntimeValue::Integer(self.instruction_pointer as i64))
            }

            SpecialFunctions::GetModificationNamespaceList => Ok(RuntimeValue::Vector(
                self.modification_namespace_list
                    .iter()
                    .map(|ns| RuntimeValue::String(ns.0.clone()))
                    .collect(),
            )),
            SpecialFunctions::GetInputSlot => {
                let slot_index = arguments[0].integer();
                let slot = self.input_slots.get(*slot_index as usize).map(|value| value.clone());
                Ok(RuntimeValue::Option(slot))
            }
            SpecialFunctions::WriteOutputSlot => {
                let output_slot_index = *arguments[0].integer();
                let input_slot_index = *arguments[1].integer();

                if output_slot_index < 0 {
                    return Ok(RuntimeValue::Result(
                        Err(Arc::new(RuntimeValue::String(
                            Arc::from("out of index"),
                        )))
                    ));
                }

                let output_slot_index = output_slot_index as usize;

                let value = match self.slots.get(input_slot_index as usize).cloned() {
                    Some(value) => value,
                    None => {
                        return Ok(RuntimeValue::Result(
                            Err(Arc::new(RuntimeValue::String(
                                Arc::from("nothing in value"),
                            )))
                        ));
                    }
                };

                if output_slot_index > self.output_slots.len() {
                    return Ok(RuntimeValue::Result(
                        Err(Arc::new(RuntimeValue::String(
                            Arc::from("out of index"),
                        )))
                    ));
                }

                if output_slot_index == self.output_slots.len() {
                    self.output_slots.push(value);
                } else {
                    self.output_slots[output_slot_index] = value;
                }

                Ok(RuntimeValue::Result(
                    Ok(Arc::new(RuntimeValue::Unit))
                ))
            }
        }
    }
}

// ── Literal conversion ────────────────────────────────────────────────────────

/// Returns `None` if the literal variant does not match the declared type,
/// which indicates the instruction stream was not verified before execution.
fn parse_literal(ty: &PrimitiveType, lit: &Literal) -> Option<RuntimeValue> {
    match (ty, lit) {
        (PrimitiveType::Integer, Literal::Integer(n)) => Some(RuntimeValue::Integer(*n)),
        (PrimitiveType::Float, Literal::Float(f)) => Some(RuntimeValue::Float(*f)),
        (PrimitiveType::String, Literal::String(s)) => Some(RuntimeValue::String(Arc::from(s.clone()))),
        (PrimitiveType::Char, Literal::Char(c)) => Some(RuntimeValue::Char(*c)),
        (PrimitiveType::Boolean, Literal::Boolean(b)) => Some(RuntimeValue::Boolean(*b)),
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

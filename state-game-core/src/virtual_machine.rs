mod event;

use std::collections::HashMap;
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::instruction::{Instruction, Literal, Slot, Type, Value};
use crate::virtual_machine::event::{StateChange, TrapReason, VirtualMachineEvent, VirtualMachineLog, VirtualMachineLogLevel, VirtualMachineTrap};

struct VirtualMachine {
    channel_transmit: Sender<VirtualMachineEvent>,
    instruction_pointer: usize,
    instructions: Arc<[Instruction]>,
    registers: HashMap<Slot, Arc<Value>>,
}

impl VirtualMachine {
    pub fn new(channel_transmit: Sender<VirtualMachineEvent>, instructions: Arc<[Instruction]>) -> Self {
        Self { channel_transmit, instruction_pointer: 0, instructions, registers: HashMap::new() }
    }

    fn emit(&self, event: VirtualMachineEvent) {
        let _ = self.channel_transmit.send(event);
    }

    fn log(&self, level: VirtualMachineLogLevel, message: impl Into<String>) {
        self.emit(VirtualMachineEvent::Log(VirtualMachineLog { level, message: message.into() }));
    }

    fn trap(&self, reason: TrapReason) -> VirtualMachineTrap {
        VirtualMachineTrap { trapped_position: self.instruction_pointer, reason }
    }

    fn ident(slot: Slot) -> String { format!("slot_{slot}") }

    fn read(&self, slot: Slot) -> Result<Arc<Value>, VirtualMachineTrap> {
        self.registers.get(&slot).cloned()
            .ok_or_else(|| self.trap(TrapReason::InvalidIdentifier(Self::ident(slot))))
    }

    fn write(&mut self, slot: Slot, value: Arc<Value>) {
        let old = self.registers.insert(slot, value.clone());
        self.emit(VirtualMachineEvent::StateChange(StateChange {
            identifier: Self::ident(slot),
            old,
            new: Some(value),
        }));
    }

    pub fn run(&mut self) -> Result<(), VirtualMachineTrap> {
        self.log(VirtualMachineLogLevel::Info, "Virtual Machine Start");

        while self.instruction_pointer < self.instructions.len() {
            let instr = self.instructions[self.instruction_pointer].clone();
            let step = self.execute(&instr);
            match step {
                Ok(NextInstructionPointer::Next(step)) => self.instruction_pointer += step,
                Ok(NextInstructionPointer::Goto(point)) => {
                    if point > self.instructions.len() {
                        let t = self.trap(TrapReason::InvalidJump(format!("{point} out of bounds")));
                        self.emit(VirtualMachineEvent::Trap(t.clone()));
                        self.emit(VirtualMachineEvent::ExecutionFinished);
                        return Err(t);
                    }
                    self.instruction_pointer = point;
                }
                Err(t) => {
                    self.emit(VirtualMachineEvent::Trap(t.clone()));
                    self.emit(VirtualMachineEvent::ExecutionFinished);
                    return Err(t);
                }
            }
        }

        self.log(VirtualMachineLogLevel::Info, "Virtual Machine Halt");
        self.emit(VirtualMachineEvent::ExecutionFinished);
        Ok(())
    }

    fn execute(&mut self, instr: &Instruction) -> Result<NextInstructionPointer, VirtualMachineTrap> {
        match instr {
            Instruction::Bind { slot, type_name, value } => {
                let v = self.parse_literal(type_name, value)?;
                self.write(*slot, Arc::new(v));
                Ok(NextInstructionPointer::Next(1))
            }
            Instruction::Call { function_name, output, arguments } => {
                let sig = &FUNCTION_REGISTRY.functions[*function_name as usize];
                if arguments.len() != sig.inputs.len() {
                    return Err(self.trap(TrapReason::InvalidIdentifier(
                        format!("{:?} expects {} args, got {}", function_name, sig.inputs.len(), arguments.len())
                    )));
                }
                let mut args: Vec<Arc<Value>> = Vec::with_capacity(arguments.len());
                for (slot, expected) in arguments.iter().zip(sig.inputs.iter()) {
                    let v = self.read(*slot)?;
                    let actual = type_of(&v);
                    if !type_eq(&actual, expected) {
                        return Err(self.trap(TrapReason::TypeMismatch {
                            expected: expected.clone(), actual,
                        }));
                    }
                    args.push(v);
                }
                let result = self.dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(NextIp::Advance)
            }
            Instruction::Jump { target_position } => Ok(NextIp::Goto(*target_position)),
            Instruction::ConditionalJump { condition, true_target_position, false_target_position } => {
                let v = self.read(*condition)?;
                match &*v {
                    Value::Boolean(b) => Ok(NextIp::Goto(
                        if *b { *true_target_position } else { *false_target_position }
                    )),
                    other => Err(self.trap(TrapReason::TypeMismatch {
                        expected: Type::Boolean, actual: type_of_ref(other),
                    })),
                }
            }
            Instruction::UnwrapSome { output, input } => {
                let v = self.read(*input)?;
                match &*v {
                    Value::Option(Some(inner)) => { self.write(*output, inner.clone()); Ok(NextIp::Advance) }
                    Value::Option(None) => Err(self.trap(TrapReason::UnwrapNone)),
                    _ => Err(self.trap(TrapReason::UnwrapWrongVariant("expected Option"))),
                }
            }
            Instruction::UnwrapOk { output, input } => {
                let v = self.read(*input)?;
                match &*v {
                    Value::Result(Ok(inner)) => { self.write(*output, inner.clone()); Ok(NextIp::Advance) }
                    Value::Result(Err(_)) => Err(self.trap(TrapReason::UnwrapWrongVariant("expected Ok, got Err"))),
                    _ => Err(self.trap(TrapReason::UnwrapWrongVariant("expected Result"))),
                }
            }
            Instruction::UnwrapErr { output, input } => {
                let v = self.read(*input)?;
                match &*v {
                    Value::Result(Err(inner)) => { self.write(*output, inner.clone()); Ok(NextIp::Advance) }
                    Value::Result(Ok(_)) => Err(self.trap(TrapReason::UnwrapWrongVariant("expected Err, got Ok"))),
                    _ => Err(self.trap(TrapReason::UnwrapWrongVariant("expected Result"))),
                }
            }
        }
    }

    fn parse_literal(&self, ty: &Type, lit: &Literal) -> Result<Value, VirtualMachineTrap> {
        let mismatch = |actual: Type| self.trap(TrapReason::TypeMismatch { expected: ty.clone(), actual });
        match (ty, lit) {
            (Type::Integer, Literal::Integer(s)) => i64::from_str(s)
                .map(Value::Integer)
                .map_err(|_| self.trap(TrapReason::ArithmeticError(format!("bad integer {s:?}")))),
            (Type::Float, Literal::Float(s)) => f64::from_str(s)
                .map(Value::Float)
                .map_err(|_| self.trap(TrapReason::ArithmeticError(format!("bad float {s:?}")))),
            (Type::String, Literal::String(s)) => Ok(Value::String(s.clone())),
            (Type::Char, Literal::Char(s)) => {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(Value::Char(c)),
                    _ => Err(self.trap(TrapReason::ArithmeticError(format!("bad char {s:?}")))),
                }
            }
            (Type::Boolean, Literal::Boolean(s)) => match s.as_str() {
                "true" => Ok(Value::Boolean(true)),
                "false" => Ok(Value::Boolean(false)),
                _ => Err(self.trap(TrapReason::ArithmeticError(format!("bad bool {s:?}")))),
            },
            (_, other) => Err(mismatch(literal_type(other))),
        }
    }

}

pub struct Logger {
    channel_receiver: Receiver<VirtualMachineEvent>,
    verbose: bool,
}

impl Logger {
    pub fn new(rx: Receiver<VirtualMachineEvent>) -> Self { Self { channel_receiver: rx, verbose: false } }
    pub fn with_verbose(mut self, v: bool) -> Self { self.verbose = v; self }

    pub fn run(&self) {
        while let Ok(event) = self.channel_receiver.recv() {
            match event {
                VirtualMachineEvent::Log(l) => {
                    let line = format!("[{:?}] {}", l.level, l.message);
                    match l.level {
                        VirtualMachineLogLevel::Warn | VirtualMachineLogLevel::Error => eprintln!("{line}"),
                        _ => println!("{line}"),
                    }
                }
                VirtualMachineEvent::Trap(t) => {
                    eprintln!("[TRAP @ {}] {:?}", t.trapped_position, t.reason);
                }
                VirtualMachineEvent::StateChange(s) if self.verbose => {
                    println!("[STATE] {}: {:?} -> {:?}", s.identifier, s.old, s.new);
                }
                VirtualMachineEvent::StateChange(_) => {}
                VirtualMachineEvent::ExecutionFinished => {
                    println!("[VM] finished");
                    break;
                }
            }
        }
    }
}

enum NextInstructionPointer {
    Next(usize),
    Goto(usize),
}

fn type_of(v: &Arc<Value>) -> Type { type_of_ref(v) }

fn type_of_ref(v: &Value) -> Type {
    match v {
        Value::Integer(_) => Type::Integer,
        Value::Float(_) => Type::Float,
        Value::String(_) => Type::String,
        Value::Char(_) => Type::Char,
        Value::Boolean(_) => Type::Boolean,
        Value::Vector(items) => {
            let inner = items.first().map(|v| type_of_ref(v)).unwrap_or(Type::Integer);
            Type::Vector(Box::new(inner))
        }
        _ => Type::Integer,
    }
}

fn type_eq(a: &Type, b: &Type) -> bool { format!("{a:?}") == format!("{b:?}") }

fn literal_type(l: &Literal) -> Type {
    match l {
        Literal::Integer(_) => Type::Integer,
        Literal::Float(_) => Type::Float,
        Literal::String(_) => Type::String,
        Literal::Char(_) => Type::Char,
        Literal::Boolean(_) => Type::Boolean,
    }
}
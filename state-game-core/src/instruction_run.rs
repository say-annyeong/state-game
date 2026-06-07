mod event;
mod types;
mod instruction;
mod instruction_verifier;
mod virtual_machine;

#[cfg(test)]
mod test {
    use std::{ops::Deref, sync::Arc, thread};
    use std::collections::HashMap;
    use crate::instruction_run::{
        instruction::{Functions, Instruction, Literal},
        instruction_verifier::InstructionVerifier,
        types::{Type, Value},
        virtual_machine::{Logger, VirtualMachine}
    };


    fn make_virtual_machine_and_logger(instructions: Arc<[Instruction]>) -> (VirtualMachine, Logger) {
        let instruction_verifier = InstructionVerifier::new(instructions.clone(), Arc::new(HashMap::new())).verify().is_empty();
        let (logger_sender, logger_receiver) = crossbeam_channel::unbounded();
        let (sender, receiver) = crossbeam_channel::unbounded();
        assert!(instruction_verifier);
        // Warning: Do not use it like this.
        (VirtualMachine::new(logger_sender, sender, receiver, 0, instructions.clone()), Logger::new(logger_receiver))
    }

    #[test]
    fn add_integer() {
        let instructions = Arc::new([
            Instruction::Bind { slot: 0, value: Literal::Integer(2), type_name: Type::Integer },
            Instruction::Bind { slot: 1, value: Literal::Integer(3), type_name: Type::Integer },
            Instruction::Call { function_name: Functions::AddInteger, inputs: vec![0, 1], output: 2 },
        ]);
        let (mut virtual_machine, logger) = make_virtual_machine_and_logger(instructions);
        let (thread1, thread2) = (thread::spawn(move || { let _ = virtual_machine.run(); } ), thread::spawn(move || { let _ = logger.run(); } ));
        let _ = thread1.join();
        let _ = thread2.join();
    }

    #[test]
    fn hello_world() {
        let instructions = Arc::new([
            Instruction::Bind { slot: 0, value: Literal::Integer(72), type_name: Type::Integer },
            Instruction::Bind { slot: 1, value: Literal::Integer(101), type_name: Type::Integer },
            Instruction::Bind { slot: 2, value: Literal::Integer(108), type_name: Type::Integer },
            Instruction::Bind { slot: 3, value: Literal::Integer(108), type_name: Type::Integer },
            Instruction::Bind { slot: 4, value: Literal::Integer(111), type_name: Type::Integer },
            Instruction::Bind { slot: 5, value: Literal::Integer(44), type_name: Type::Integer },
            Instruction::Bind { slot: 6, value: Literal::Integer(32), type_name: Type::Integer },
            Instruction::Bind { slot: 7, value: Literal::Integer(87), type_name: Type::Integer },
            Instruction::Bind { slot: 8, value: Literal::Integer(111), type_name: Type::Integer },
            Instruction::Bind { slot: 9, value: Literal::Integer(114), type_name: Type::Integer },
            Instruction::Bind { slot: 10, value: Literal::Integer(108), type_name: Type::Integer },
            Instruction::Bind { slot: 11, value: Literal::Integer(100), type_name: Type::Integer },
            Instruction::Bind { slot: 12, value: Literal::Integer(33), type_name: Type::Integer },
            Instruction::Bind { slot: 13, value: Literal::String("Hello, World!".to_string()), type_name: Type::Integer },
        ]);
        let (mut virtual_machine, logger) = make_virtual_machine_and_logger(instructions);
        let (thread1, thread2) = (thread::spawn(move || {
            let _ = virtual_machine.run();
            let mut slots = Vec::new();
            for (k, v) in virtual_machine.slots {
                slots.push((k, v));
            }
            slots.sort_by(|a, b| a.0.cmp(&b.0));
            let mut result = String::new();
            for (_, value) in slots {
                if let Value::Integer(i) = value.deref().clone() {
                    result.push(i as u8 as char);
                }
            }
            println!("{}", result);
        } ), thread::spawn(move || {
            let _ = logger.run();
        } ));
        let _ = thread1.join();
        let _ = thread2.join();
    }

    #[test]
    fn fibo() {
        let instructions = Arc::new([
            Instruction::Bind { slot: 0, value: Literal::Integer(5), type_name: Type::Integer },
            Instruction::Bind { slot: 1, value: Literal::Integer(0), type_name: Type::Integer },
            Instruction::Bind { slot: 2, value: Literal::Integer(1), type_name: Type::Integer },
            Instruction::Bind { slot: 3, value: Literal::Integer(13), type_name: Type::Integer },
            Instruction::Bind { slot: 4, value: Literal::Integer(20), type_name: Type::Integer },
            Instruction::Bind { slot: 5, value: Literal::Integer(40), type_name: Type::Integer },
            Instruction::Call { function_name: Functions::VectorInitInteger, inputs: Vec::new(), output: 6 },
            Instruction::Bind { slot: 7, value: Literal::Integer(0), type_name: Type::Integer },
            Instruction::Bind { slot: 8, value: Literal::Integer(0), type_name: Type::Integer },
            Instruction::Call { function_name: Functions::VectorPushInteger, inputs: vec![6, 7], output: 9 },
            Instruction::Call { function_name: Functions::VectorPushInteger, inputs: vec![9, 8], output: 10 },
        ]);
    }
}
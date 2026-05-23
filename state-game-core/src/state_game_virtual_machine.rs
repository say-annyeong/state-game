mod event;

use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::state_game_instruction::{Instruction};
use crate::state_game_virtual_machine::event::{VirtualMachineEvent, VirtualMachineLog, VirtualMachineLogLevel, VirtualMachineTrap};

struct VirtualMachine {
    channel_transmit: Sender<VirtualMachineEvent>,
    instruction_pointer: usize,
    instructions: Arc<[Instruction]>
}

impl VirtualMachine {
    pub fn new(channel_transmit: Sender<VirtualMachineEvent>, instructions: Arc<[Instruction]>) -> Self {
        Self { channel_transmit, instruction_pointer: 0, instructions }
    }

    fn emit(&self, event: VirtualMachineEvent) {
        let _ = self.channel_transmit.send(event);
    }

    fn run(&mut self) -> Result<(), VirtualMachineTrap> {
        self.emit(
            VirtualMachineEvent::Log(
                VirtualMachineLog { level: VirtualMachineLogLevel::Info, message: "Virtual Machine Start".to_string() }
            )
        );
        Ok(())
    }
}

struct Logger {
    channel_receiver: Receiver<VirtualMachineEvent>,
}
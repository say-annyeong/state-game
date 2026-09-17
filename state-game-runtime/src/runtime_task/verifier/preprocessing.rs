use std::sync::Arc;
use crate::runtime_task::instruction::{ExpressionInstruction, Instruction};
use crate::runtime_task::verifier::virtual_machine_instruction_metadata::{ExpressionFunctionMetadata, ExpressionInstructionMetadata};

struct Preprocessing<'a> {
    expression_function_metadata: Arc<[(ExpressionFunctionMetadata<'a>, Arc<[ExpressionInstruction<'a>]>, Arc<[ExpressionInstructionMetadata<'a>]>)]>
}
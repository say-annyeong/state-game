use std::collections::HashMap;
use std::sync::Arc;

use crate::runtime_task::{
    instruction::{
        ConcreteInstruction, ExpressionInstruction, FunctionIdentifier,
        Instruction, FUNCTION_REGISTRY, SPECIAL_FUNCTIONS_REGISTRY,
    },
    types::{
        ConcreteType, ConcreteTypeInterner, GenericIdentifier, GenericTypeIdentifier,
        GenericTypeInterner, NonGenericTypeIdentifier, PrimitiveType, TypeExpression,
    },
    verifier::virtual_machine_instruction_metadata::{
        ConcreteFunctionMetadata, ExpressionFunctionMetadata,
    },
};

// ---------------------------------------------------------------------------
// SlotMap: slot index -> ConcreteType
// ---------------------------------------------------------------------------

type SlotMap = HashMap<u64, ConcreteType>;

// ---------------------------------------------------------------------------
// Helpers: TypeExpression <-> ConcreteType conversion via interners
// ---------------------------------------------------------------------------

/// Try to convert a TypeExpression to a ConcreteType.
/// Returns None if the expression contains unresolved generics.
fn try_concrete(
    expression: &TypeExpression,
    generic_interner: &GenericTypeInterner,
    concrete_interner: &mut ConcreteTypeInterner,
) -> Option<ConcreteType> {
    match expression {
        TypeExpression::Primitive(p) => Some(ConcreteType::Primitive(*p)),
        TypeExpression::Generic(_) => None,
        TypeExpression::Vector(generic_identifier) => {
            let inner_te = generic_interner.get(*generic_identifier)?;
            let inner_ct = try_concrete(inner_te, generic_interner, concrete_interner)?;
            let inner_id = concrete_interner.intern(inner_ct);
            Some(ConcreteType::Vector(inner_id))
        }
        TypeExpression::Option(generic_identifier) => {
            let inner_te = generic_interner.get(*generic_identifier)?;
            let inner_ct = try_concrete(inner_te, generic_interner, concrete_interner)?;
            let inner_id = concrete_interner.intern(inner_ct);
            Some(ConcreteType::Option(inner_id))
        }
        TypeExpression::Result { ok: ok_gid, err: err_gid } => {
            let ok_te = generic_interner.get(*ok_gid)?;
            let err_te = generic_interner.get(*err_gid)?;
            let ok_ct = try_concrete(ok_te, generic_interner, concrete_interner)?;
            let err_ct = try_concrete(err_te, generic_interner, concrete_interner)?;
            let ok_id = concrete_interner.intern(ok_ct);
            let err_id = concrete_interner.intern(err_ct);
            Some(ConcreteType::Result { ok: ok_id, err: err_id })
        }
    }
}

/// Substitute generic parameters in a TypeExpression and convert to ConcreteType.
/// `parameter_identifiers` — ordered list of formal generic parameter IDs.
/// `assignments` — concrete type for each parameter (None = unresolved).
fn substitute(
    expression: &TypeExpression,
    parameter_identifiers: &[GenericIdentifier],
    assignments: &[Option<ConcreteType>],
    generic_interner: &GenericTypeInterner,
    concrete_interner: &mut ConcreteTypeInterner,
) -> Option<ConcreteType> {
    match expression {
        TypeExpression::Primitive(p) => Some(ConcreteType::Primitive(*p)),
        TypeExpression::Generic(id) => {
            let position = parameter_identifiers.iter().position(|generic_parameter| generic_parameter == id)?;
            assignments.get(position).and_then(|option| option.clone())
        }
        TypeExpression::Vector(generic_identifier) => {
            let inner_te = generic_interner.get(*generic_identifier)?;
            let inner_ct = substitute(inner_te, parameter_identifiers, assignments, generic_interner, concrete_interner)?;
            let inner_id = concrete_interner.intern(inner_ct);
            Some(ConcreteType::Vector(inner_id))
        }
        TypeExpression::Option(generic_identifier) => {
            let inner_te = generic_interner.get(*generic_identifier)?;
            let inner_ct = substitute(inner_te, parameter_identifiers, assignments, generic_interner, concrete_interner)?;
            let inner_id = concrete_interner.intern(inner_ct);
            Some(ConcreteType::Option(inner_id))
        }
        TypeExpression::Result { ok: ok_gid, err: err_gid } => {
            let ok_te = generic_interner.get(*ok_gid)?;
            let err_te = generic_interner.get(*err_gid)?;
            let ok_ct = substitute(ok_te, parameter_identifiers, assignments, generic_interner, concrete_interner)?;
            let err_ct = substitute(err_te, parameter_identifiers, assignments, generic_interner, concrete_interner)?;
            let ok_id = concrete_interner.intern(ok_ct);
            let err_id = concrete_interner.intern(err_ct);
            Some(ConcreteType::Result { ok: ok_id, err: err_id })
        }
    }
}

/// Infer generic assignments by matching a concrete type against a TypeExpression template.
/// Writes into `assignments`; returns false on contradiction.
fn infer_generics(
    template: &TypeExpression,
    concrete: &ConcreteType,
    parameter_identifiers: &[GenericIdentifier],
    assignments: &mut Vec<Option<ConcreteType>>,
    generic_interner: &GenericTypeInterner,
    concrete_interner: &ConcreteTypeInterner,
) -> bool {
    match (template, concrete) {
        (TypeExpression::Generic(id), _) => {
            if let Some(position) = parameter_identifiers.iter().position(|generic_parameter| generic_parameter == id) {
                if assignments.len() <= position {
                    assignments.resize(position + 1, None);
                }
                match &assignments[position] {
                    None => {
                        assignments[position] = Some(concrete.clone());
                        true
                    }
                    Some(existing) => existing == concrete,
                }
            } else {
                false
            }
        }
        (TypeExpression::Primitive(p), ConcreteType::Primitive(c)) => p == c,
        (TypeExpression::Vector(generic_identifier), ConcreteType::Vector(non_generic_identifier)) => {
            let Some(inner_te) = generic_interner.get(*generic_identifier) else { return false; };
            let Some(inner_ct) = concrete_interner.get(*non_generic_identifier) else { return false; };
            infer_generics(inner_te, inner_ct, parameter_identifiers, assignments, generic_interner, concrete_interner)
        }
        (TypeExpression::Option(generic_identifier), ConcreteType::Option(non_generic_identifier)) => {
            let Some(inner_te) = generic_interner.get(*generic_identifier) else { return false; };
            let Some(inner_ct) = concrete_interner.get(*non_generic_identifier) else { return false; };
            infer_generics(inner_te, inner_ct, parameter_identifiers, assignments, generic_interner, concrete_interner)
        }
        (TypeExpression::Result { ok: ok_gid, err: err_gid },
         ConcreteType::Result { ok: ok_ngid, err: err_ngid }) => {
            let Some(ok_te) = generic_interner.get(*ok_gid) else { return false; };
            let Some(err_te) = generic_interner.get(*err_gid) else { return false; };
            let Some(ok_ct) = concrete_interner.get(*ok_ngid) else { return false; };
            let Some(err_ct) = concrete_interner.get(*err_ngid) else { return false; };
            infer_generics(ok_te, ok_ct, parameter_identifiers, assignments, generic_interner, concrete_interner)
                && infer_generics(err_te, err_ct, parameter_identifiers, assignments, generic_interner, concrete_interner)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Variant: one monomorphised copy of an expression function
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Variant {
    sub_id: FunctionIdentifier,
    slot_map: SlotMap,
    input_types: Vec<ConcreteType>,
    output_types: Vec<ConcreteType>,
}

// ---------------------------------------------------------------------------
// Preprocessing
// ---------------------------------------------------------------------------

pub struct Preprocessing<'a> {
    expression_function_metadata:
        &'a [(ExpressionFunctionMetadata, Arc<[ExpressionInstruction]>)],
    generic_interner: GenericTypeInterner,
    concrete_interner: ConcreteTypeInterner,
}

impl<'a> Preprocessing<'a> {
    pub fn new(
        expression_function_metadata: &'a [(ExpressionFunctionMetadata, Arc<[ExpressionInstruction]>)],
    ) -> Self {
        Self {
            expression_function_metadata,
            generic_interner: GenericTypeInterner::new(),
            concrete_interner: ConcreteTypeInterner::new(),
        }
    }

    /// Run type inference and monomorphisation, returning concrete function metadata
    /// and instructions.
    pub fn preprocess(
        mut self,
    ) -> (
        Vec<(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)>,
        GenericTypeInterner,
        ConcreteTypeInterner,
    ) {
        let mut all_variants: Vec<(FunctionIdentifier, Vec<Variant>)> = Vec::new();
        let mut next_sub_id: FunctionIdentifier = 0;

        for (metadata, instructions) in self.expression_function_metadata.iter() {
            let variant = self.infer_variant(metadata, instructions, &mut next_sub_id);
            all_variants.push((metadata.function_identifier, variant));
        }

        // Build output
        let mut result: Vec<(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)> = Vec::new();

        for ((func_id, variants), (metadata, instructions)) in
            all_variants.iter().zip(self.expression_function_metadata.iter())
        {
            for variant in variants {
                let max_slot = metadata.slot_types.len()
                    .max(variant.slot_map.keys().copied().map(|k| k as usize + 1).max().unwrap_or(0));

                let slot_types: Vec<ConcreteType> = (0..max_slot as u64)
                    .map(|i| {
                        variant.slot_map.get(&i)
                            .cloned()
                            .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit))
                    })
                    .collect();

                let concrete_instructions: Arc<[ConcreteInstruction]> = instructions
                    .iter()
                    .map(|instr| self.concretize_instruction(instr, variant))
                    .collect();

                result.push((
                    ConcreteFunctionMetadata {
                        function_identifier: *func_id,
                        sub_function_identifier: variant.sub_id,
                        slot_types,
                        input_types: variant.input_types.clone(),
                        output_types: variant.output_types.clone(),
                    },
                    concrete_instructions,
                ));
            }
        }

        (result, self.generic_interner, self.concrete_interner)
    }

    // -------------------------------------------------------------------------
    // Infer a single Variant for one expression function
    // -------------------------------------------------------------------------

    fn infer_variant(
        &mut self,
        metadata: &ExpressionFunctionMetadata,
        instructions: &[ExpressionInstruction],
        next_sub_id: &mut FunctionIdentifier,
    ) -> Vec<Variant> {
        let sub_id = *next_sub_id;
        *next_sub_id += 1;

        let mut slot_map: SlotMap = HashMap::new();

        // Seed from declared input types
        for (i, input_type) in metadata.input_types.iter().enumerate() {
            if let Some(concrete_type) = try_concrete(input_type, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.insert(i as u64, concrete_type);
            }
        }

        // Pass 1: definite (non-generic) evidence
        self.pass_definite(instructions, &mut slot_map);

        // Pass 2: iterative generic inference until fixed point
        self.pass_generic(instructions, &mut slot_map);

        // Resolve input/output types
        let input_types: Vec<ConcreteType> = metadata.input_types.iter().enumerate()
            .map(|(i, _)| {
                slot_map.get(&(i as u64))
                    .cloned()
                    .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit))
            })
            .collect();

        let output_types: Vec<ConcreteType> = metadata.output_types.iter()
            .map(|expression_type| {
                try_concrete(expression_type, &self.generic_interner, &mut self.concrete_interner)
                    .unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit))
            })
            .collect();

        vec![Variant { sub_id, slot_map, input_types, output_types }]
    }

    // -------------------------------------------------------------------------
    // Pass 1: definite evidence
    // Priority: Bind > non-generic Call/SpecialCall > explicit generic arguments
    // -------------------------------------------------------------------------

    fn pass_definite(&mut self, instructions: &[ExpressionInstruction], slot_map: &mut SlotMap) {
        // 1. Bind — most definite source
        for instr in instructions.iter() {
            if let ExpressionInstruction::Bind { slot, type_name, .. } = instr {
                slot_map.insert(*slot, ConcreteType::Primitive(*type_name));
            }
        }

        // 2. Call/SpecialCall with fully concrete signatures or explicit generics
        for instr in instructions.iter() {
            match instr {
                ExpressionInstruction::Call { function_name, generic_arguments, inputs, output } => {
                    let mut temporary_interner = self.generic_interner.clone();
                    if let Some(signature) = FUNCTION_REGISTRY.build(*function_name as usize, &mut temporary_interner) {
                        self.generic_interner = temporary_interner;
                        let parameter_identifiers: Vec<GenericIdentifier> =
                            signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                        if parameter_identifiers.is_empty() {
                            self.apply_concrete_sig(&signature.inputs, &signature.output, inputs, *output, slot_map);
                        } else {
                            self.try_apply_explicit(
                                &parameter_identifiers, &signature.inputs, &signature.output,
                                generic_arguments, inputs, *output, slot_map,
                            );
                        }
                    }
                }
                ExpressionInstruction::SpecialCall { function_name, generic_arguments, inputs, output } => {
                    let mut temporary_interner = self.generic_interner.clone();
                    if let Some(signature) = SPECIAL_FUNCTIONS_REGISTRY.build(*function_name as usize, &mut temporary_interner) {
                        self.generic_interner = temporary_interner;
                        let parameter_identifiers: Vec<GenericIdentifier> =
                            signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                        if parameter_identifiers.is_empty() {
                            self.apply_concrete_sig(&signature.inputs, &signature.output, inputs, *output, slot_map);
                        } else {
                            self.try_apply_explicit(
                                &parameter_identifiers, &signature.inputs, &signature.output,
                                generic_arguments, inputs, *output, slot_map,
                            );
                        }
                    }
                }
                ExpressionInstruction::ConditionalJump { condition, .. } => {
                    slot_map.entry(*condition).or_insert(ConcreteType::Primitive(PrimitiveType::Boolean));
                }
                _ => {}
            }
        }
    }

    /// Apply a fully-concrete (generic-free) signature to the slot map.
    fn apply_concrete_sig(
        &mut self,
        signature_inputs: &[TypeExpression],
        signature_output: &TypeExpression,
        inputs: &[u64],
        output: u64,
        slot_map: &mut SlotMap,
    ) {
        for (slot, expression_type) in inputs.iter().zip(signature_inputs.iter()) {
            if let Some(concrete_type) = try_concrete(expression_type, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.entry(*slot).or_insert(concrete_type);
            }
        }
        if let Some(concrete_type) = try_concrete(signature_output, &self.generic_interner, &mut self.concrete_interner) {
            slot_map.entry(output).or_insert(concrete_type);
        }
    }

    /// Apply explicit generic_arguments (constraint 3: user wrote fn f<Int>(42)).
    fn try_apply_explicit(
        &mut self,
        parameter_identifiers: &[GenericIdentifier],
        signature_inputs: &[TypeExpression],
        signature_output: &TypeExpression,
        explicit: &[TypeExpression],
        inputs: &[u64],
        output: u64,
        slot_map: &mut SlotMap,
    ) {
        if explicit.is_empty() {
            return;
        }
        let assignments: Vec<Option<ConcreteType>> = explicit.iter()
            .map(|type_expression| try_concrete(type_expression, &self.generic_interner, &mut self.concrete_interner))
            .collect();

        for (slot, expression_type) in inputs.iter().zip(signature_inputs.iter()) {
            if let Some(concrete_type) = substitute(expression_type, parameter_identifiers, &assignments, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.entry(*slot).or_insert(concrete_type);
            }
        }
        if let Some(concrete_type) = substitute(signature_output, parameter_identifiers, &assignments, &self.generic_interner, &mut self.concrete_interner) {
            slot_map.entry(output).or_insert(concrete_type);
        }
    }

    // -------------------------------------------------------------------------
    // Pass 2: iterative generic inference until fixed point
    // -------------------------------------------------------------------------

    fn pass_generic(&mut self, instructions: &[ExpressionInstruction], slot_map: &mut SlotMap) {
        loop {
            let snapshot = slot_map.clone();

            for instr in instructions.iter() {
                match instr {
                    ExpressionInstruction::Call { function_name, generic_arguments, inputs, output } => {
                        let mut temporary_interner = self.generic_interner.clone();
                        if let Some(signature) = FUNCTION_REGISTRY.build(*function_name as usize, &mut temporary_interner) {
                            self.generic_interner = temporary_interner;
                            let parameter_identifiers: Vec<GenericIdentifier> =
                                signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                            if !parameter_identifiers.is_empty() {
                                self.infer_from_generic_signature(
                                    &parameter_identifiers, &signature.inputs, &signature.output,
                                    generic_arguments, inputs, *output, slot_map,
                                );
                            }
                        }
                    }
                    ExpressionInstruction::SpecialCall { function_name, generic_arguments, inputs, output } => {
                        let mut temporary_interner = self.generic_interner.clone();
                        if let Some(signature) = SPECIAL_FUNCTIONS_REGISTRY.build(*function_name as usize, &mut temporary_interner) {
                            self.generic_interner = temporary_interner;
                            let parameter_identifiers: Vec<GenericIdentifier> =
                                signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                            if !parameter_identifiers.is_empty() {
                                self.infer_from_generic_signature(
                                    &parameter_identifiers, &signature.inputs, &signature.output,
                                    generic_arguments, inputs, *output, slot_map,
                                );
                            }
                        }
                    }
                    ExpressionInstruction::DefinedCall { function_identifier, generic_arguments, inputs, outputs } => {
                        self.infer_from_defined_call(*function_identifier, inputs, outputs, slot_map);
                    }
                    _ => {}
                }
            }

            if *slot_map == snapshot {
                break;
            }
        }
    }

    /// Infer generic assignments from known slots, then fill remaining slots.
    fn infer_from_generic_signature(
        &mut self,
        parameter_identifiers: &[GenericIdentifier],
        signature_inputs: &[TypeExpression],
        signature_output: &TypeExpression,
        explicit: &[TypeExpression],
        inputs: &[u64],
        output: u64,
        slot_map: &mut SlotMap,
    ) {
        // Explicit generic arguments take priority (constraint 3)
        if !explicit.is_empty() {
            self.try_apply_explicit(parameter_identifiers, signature_inputs, signature_output, explicit, inputs, output, slot_map);
            return;
        }

        // Infer from already-known slots
        let mut assignments: Vec<Option<ConcreteType>> = vec![None; parameter_identifiers.len()];

        for (slot, expression_type) in inputs.iter().zip(signature_inputs.iter()) {
            if let Some(concrete_type) = slot_map.get(slot).cloned() {
                infer_generics(expression_type, &concrete_type, parameter_identifiers, &mut assignments, &self.generic_interner, &self.concrete_interner);
            }
        }
        if let Some(concrete_type) = slot_map.get(&output).cloned() {
            infer_generics(signature_output, &concrete_type, parameter_identifiers, &mut assignments, &self.generic_interner, &self.concrete_interner);
        }

        // Fill remaining unknown slots using inferred assignments
        for (slot, expression_type) in inputs.iter().zip(signature_inputs.iter()) {
            if !slot_map.contains_key(slot) {
                if let Some(concrete_type) = substitute(expression_type, parameter_identifiers, &assignments, &self.generic_interner, &mut self.concrete_interner) {
                    slot_map.insert(*slot, concrete_type);
                }
            }
        }
        if !slot_map.contains_key(&output) {
            if let Some(concrete_type) = substitute(signature_output, parameter_identifiers, &assignments, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.insert(output, concrete_type);
            }
        }
    }

    /// Infer types for a DefinedCall from the callee's declared signature.
    fn infer_from_defined_call(
        &mut self,
        function_identifier: FunctionIdentifier,
        inputs: &[u64],
        outputs: &[u64],
        slot_map: &mut SlotMap,
    ) {
        let callee = self.expression_function_metadata
            .iter()
            .find(|(m, _)| m.function_identifier == function_identifier);

        let Some((callee_meta, _)) = callee else { return; };

        let callee_inputs: Vec<TypeExpression> = callee_meta.input_types.clone();
        let callee_outputs: Vec<TypeExpression> = callee_meta.output_types.clone();

        for (slot, expression_type) in inputs.iter().zip(callee_inputs.iter()) {
            if let Some(concrete_type) = try_concrete(expression_type, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.entry(*slot).or_insert(concrete_type);
            }
        }
        for (slot, expression_type) in outputs.iter().zip(callee_outputs.iter()) {
            if let Some(concrete_type) = try_concrete(expression_type, &self.generic_interner, &mut self.concrete_interner) {
                slot_map.entry(*slot).or_insert(concrete_type);
            }
        }
    }

    // -------------------------------------------------------------------------
    // Convert ExpressionInstruction -> ConcreteInstruction
    // -------------------------------------------------------------------------

    fn concretize_instruction(
        &mut self,
        instr: &ExpressionInstruction,
        variant: &Variant,
    ) -> ConcreteInstruction {
        match instr {
            Instruction::Bind { slot, type_name, value } => {
                Instruction::Bind { slot: *slot, type_name: *type_name, value: value.clone() }
            }
            Instruction::Call { function_name, generic_arguments, inputs, output } => {
                let mut temporary_interner = self.generic_interner.clone();
                let signature = FUNCTION_REGISTRY.build(*function_name as usize, &mut temporary_interner);
                self.generic_interner = temporary_interner;
                let concrete_generics = if let Some(signature) = signature {
                    let parameter_identifiers: Vec<GenericIdentifier> = signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                    self.resolve_generic_argument(
                        &parameter_identifiers, generic_arguments, inputs, *output,
                        &signature.inputs, &signature.output, &variant.slot_map,
                    )
                } else {
                    vec![]
                };
                Instruction::Call {
                    function_name: *function_name,
                    generic_arguments: concrete_generics.into_boxed_slice(),
                    inputs: inputs.clone(),
                    output: *output,
                }
            }
            Instruction::SpecialCall { function_name, generic_arguments, inputs, output } => {
                let mut temporary_interner = self.generic_interner.clone();
                let signature = SPECIAL_FUNCTIONS_REGISTRY.build(*function_name as usize, &mut temporary_interner);
                self.generic_interner = temporary_interner;
                let concrete_generics = if let Some(signature) = signature {
                    let parameter_identifiers: Vec<GenericIdentifier> = signature.generics.iter().map(|generic_parameter| generic_parameter.id).collect();
                    self.resolve_generic_argument(
                        &parameter_identifiers, generic_arguments, inputs, *output,
                        &signature.inputs, &signature.output, &variant.slot_map,
                    )
                } else {
                    vec![]
                };
                Instruction::SpecialCall {
                    function_name: *function_name,
                    generic_arguments: concrete_generics.into_boxed_slice(),
                    inputs: inputs.clone(),
                    output: *output,
                }
            }
            Instruction::DefinedCall { function_identifier, inputs, outputs, .. } => {
                Instruction::DefinedCall {
                    function_identifier: *function_identifier,
                    generic_arguments: Box::new([]),
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                }
            }
            Instruction::Jump { target_position } => {
                Instruction::Jump { target_position: *target_position }
            }
            Instruction::ConditionalJump { condition, true_target_position, false_target_position } => {
                Instruction::ConditionalJump {
                    condition: *condition,
                    true_target_position: *true_target_position,
                    false_target_position: *false_target_position,
                }
            }
            Instruction::ReturnDefinedCall { function_identifier, outputs } => {
                Instruction::ReturnDefinedCall {
                    function_identifier: *function_identifier,
                    outputs: outputs.clone(),
                }
            }
        }
    }

    fn resolve_generic_argument(
        &mut self,
        parameter_identifiers: &[GenericIdentifier],
        explicit: &[TypeExpression],
        inputs: &[u64],
        output: u64,
        signature_inputs: &[TypeExpression],
        signature_output: &TypeExpression,
        slot_map: &SlotMap,
    ) -> Vec<ConcreteType> {
        if parameter_identifiers.is_empty() {
            return vec![];
        }
        // Explicit generic arguments take priority (constraint 3)
        if !explicit.is_empty() {
            return explicit.iter()
                .filter_map(|type_expression| try_concrete(type_expression, &self.generic_interner, &mut self.concrete_interner))
                .collect();
        }

        let mut assignments: Vec<Option<ConcreteType>> = vec![None; parameter_identifiers.len()];

        for (slot, expression_type) in inputs.iter().zip(signature_inputs.iter()) {
            if let Some(concrete_type) = slot_map.get(slot).cloned() {
                infer_generics(expression_type, &concrete_type, parameter_identifiers, &mut assignments, &self.generic_interner, &self.concrete_interner);
            }
        }
        if let Some(concrete_type) = slot_map.get(&output).cloned() {
            infer_generics(signature_output, &concrete_type, parameter_identifiers, &mut assignments, &self.generic_interner, &self.concrete_interner);
        }

        assignments.into_iter()
            .map(|option| option.unwrap_or(ConcreteType::Primitive(PrimitiveType::Unit)))
            .collect()
    }
}

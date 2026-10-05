use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use dashmap::DashMap;
use state_game_core::{Identifier, Namespace};

use crate::runtime_task::{
    event::{
        RuntimeTaskEvent,
        RuntimeTaskTrap, RuntimeTaskYield,
    },
    instruction::{ConcreteInstruction, FunctionIdentifier, RuntimeTaskIdentifier, Slot},
    runtime_task::RuntimeTask,
    types::RuntimeValue,
    verifier::virtual_machine_instruction_metadata::ConcreteFunctionMetadata,
};

// ---------------------------------------------------------------------------
// SchedulerOption
// ---------------------------------------------------------------------------

pub struct SchedulerOption {
    /// CPU 사용률 상한 (0–100). 현재는 tick 내부 루프 반복 횟수에 영향을 주지
    /// 않으며, 향후 OS 수준 CPU 측정을 붙일 자리를 예약한다.
    pub max_cpu_using: u8,
    /// 메모리 상한(MB). 슬롯 수 기반 근사치로 판단한다.
    pub max_ram_using: u64,
    /// 하나의 tick이 소비할 수 있는 최대 실행 시간.
    /// `Duration::MAX`로 설정하면 사실상 제한 없음.
    pub max_tick_duration: Duration,
    pub _non_exhaustive: NonExhaustive,
}

impl Default for SchedulerOption {
    fn default() -> Self {
        Self {
            max_cpu_using: 20,
            max_ram_using: 8192,
            max_tick_duration: Duration::MAX,
            _non_exhaustive: NonExhaustive,
        }
    }
}

pub struct NonExhaustive;

// ---------------------------------------------------------------------------
// 태스크 상태
// ---------------------------------------------------------------------------

/// 각 RuntimeTask가 현재 어떤 상태인지를 나타낸다.
enum TaskState {
    /// 실행 준비 완료.
    Ready,

    /// DefinedCall을 yield했으며, 피호출 함수가 완료되기를 기다리는 중.
    ///
    /// - `callee_id`: 실행 중인 자식 태스크 식별자
    /// - `destination_slots`: 자식 반환값을 기록할 슬롯 목록
    WaitingForReturn {
        callee_id: RuntimeTaskIdentifier,
        destination_slots: Vec<Slot>,
    },

    /// 정상 완료.
    Finished,

    /// Trap 발생으로 종료.
    Trapped(RuntimeTaskTrap),
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

pub struct Scheduler {
    /// 검증을 통과한 함수 목록.
    functions: Vec<(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)>,

    /// 현재 활성 태스크 목록.
    tasks: HashMap<RuntimeTaskIdentifier, RuntimeTask>,

    /// 각 태스크의 실행 상태.
    states: HashMap<RuntimeTaskIdentifier, TaskState>,

    /// 다음 태스크 식별자 발급용 카운터.
    next_task_id: RuntimeTaskIdentifier,

    /// 모든 태스크가 공유하는 전역 메모리.
    global_memory: Arc<DashMap<(Namespace, Identifier), RuntimeValue>>,

    /// 수정 가능한 네임스페이스 목록.
    modification_namespace_list: Arc<[Namespace]>,

    /// 이벤트 로그 수신자 (외부 관찰용; 현재는 드랍해도 무방).
    _log_receiver: crossbeam_channel::Receiver<RuntimeTaskEvent>,
    log_sender: crossbeam_channel::Sender<RuntimeTaskEvent>,
}

impl Scheduler {
    pub fn new(
        functions: Vec<(ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)>,
        global_memory: Arc<DashMap<(Namespace, Identifier), RuntimeValue>>,
        modification_namespace_list: Arc<[Namespace]>,
    ) -> Self {
        let (log_tx, log_rx) = unbounded();
        Self {
            functions,
            tasks: HashMap::new(),
            states: HashMap::new(),
            next_task_id: 0,
            global_memory,
            modification_namespace_list,
            _log_receiver: log_rx,
            log_sender: log_tx,
        }
    }

    // -----------------------------------------------------------------------
    // 공개 API
    // -----------------------------------------------------------------------

    /// 특정 함수 식별자로 최상위 태스크를 생성해 스케줄러에 등록한다.
    ///
    /// 반환값은 새로 생성된 태스크 식별자다.
    pub fn spawn(
        &mut self,
        function_identifier: FunctionIdentifier,
        input_slots: Vec<Arc<RuntimeValue>>,
    ) -> Option<RuntimeTaskIdentifier> {
        let (meta, instructions) = self.find_function(function_identifier)?;
        let slot_count = meta.slot_types.len();

        let id = self.alloc_task_id();
        let (sched_tx, sched_rx) = unbounded();

        let task = RuntimeTask::new(
            self.log_sender.clone(),
            sched_tx,
            sched_rx,
            id,
            instructions,
            self.global_memory.clone(),
            self.modification_namespace_list.clone(),
            slot_count,
        );

        // 입력 슬롯 주입 — slot 0..N에 순서대로 기록
        // (task가 mut이어야 하므로 insert 후 갱신)
        let mut task = task;
        for (i, v) in input_slots.into_iter().enumerate() {
            if i < task.slots.len() {
                task.slots[i] = v;
            }
        }

        self.tasks.insert(id, task);
        self.states.insert(id, TaskState::Ready);
        Some(id)
    }

    /// 한 tick을 실행한다.
    ///
    /// # 동작
    ///
    /// 1. Ready 상태인 태스크를 하나씩 `run_until_yield`로 실행한다.
    /// 2. 결과에 따라 상태를 전이한다.
    ///    - `Finished`         → 태스크를 Finished로 표시하고, 자신을 기다리는
    ///                           부모가 있으면 반환값을 전달해 Ready로 복귀시킨다.
    ///    - `Call { .. }`      → 피호출 함수를 찾아 새 자식 태스크를 생성하고
    ///                           현재 태스크를 WaitingForReturn으로 표시한다.
    ///                           피호출 함수가 없으면 Trapped로 표시한다.
    ///    - `Return { .. }`    → 부모 태스크를 찾아 반환값을 resume_call로 주입하고
    ///                           Ready로 복귀시킨다. 자신은 Finished로 표시한다.
    ///    - Trap               → 태스크를 Trapped로 표시한다.
    /// 3. `max_tick_duration`이 경과하면 나머지 Ready 태스크는 다음 tick으로 미룬다.
    /// 4. `max_ram_using`을 초과하면(슬롯 수 근사치) tick을 중단한다.
    ///
    /// # 반환
    ///
    /// tick 실행 중 Trap이 발생한 태스크의 (id, trap) 쌍 목록.
    pub fn tick(&mut self, option: SchedulerOption) -> Vec<(RuntimeTaskIdentifier, RuntimeTaskTrap)> {
        let deadline = Instant::now() + option.max_tick_duration;
        let ram_limit_bytes: u64 = option.max_ram_using * 1024 * 1024;

        let mut traps: Vec<(RuntimeTaskIdentifier, RuntimeTaskTrap)> = Vec::new();

        // Ready 태스크 목록을 미리 수집 (borrow 분리)
        let ready_ids: Vec<RuntimeTaskIdentifier> = self
            .states
            .iter()
            .filter_map(|(id, state)| matches!(state, TaskState::Ready).then_some(*id))
            .collect();

        'outer: for task_id in ready_ids {
            // ── 시간 제한 확인 ────────────────────────────────────────────────
            if Instant::now() >= deadline {
                break 'outer;
            }

            // ── 메모리 제한 확인 (슬롯 수 × Arc 크기 근사) ───────────────────
            if ram_limit_bytes < u64::MAX {
                let estimated = self.estimate_ram_usage();
                if estimated > ram_limit_bytes {
                    break 'outer;
                }
            }

            // ── 태스크 실행 ───────────────────────────────────────────────────
            let Some(task) = self.tasks.get_mut(&task_id) else {
                continue;
            };

            match task.run_until_yield() {
                // ── 정상 완료 ─────────────────────────────────────────────────
                Ok(RuntimeTaskYield::Finished) => {
                    self.states.insert(task_id, TaskState::Finished);
                    // 이 태스크를 기다리던 부모가 있으면 빈 결과로 재개
                    self.resume_parent_if_waiting(task_id, vec![]);
                }

                // ── 사용자 함수 호출 yield ────────────────────────────────────
                Ok(RuntimeTaskYield::Call {
                    function_identifier,
                    inputs,
                    destination_slots,
                }) => {
                    match self.find_function(function_identifier) {
                        Some((meta, instructions)) => {
                            let slot_count = meta.slot_types.len();
                            let child_id = self.alloc_task_id();
                            let (sched_tx, sched_rx) = unbounded();

                            // 입력값을 슬롯 번호 순서대로 정렬해 주입
                            let mut input_vec: Vec<(Slot, Arc<RuntimeValue>)> =
                                inputs.into_iter().collect();
                            input_vec.sort_by_key(|(slot, _)| *slot);
                            let ordered_inputs: Vec<Arc<RuntimeValue>> =
                                input_vec.into_iter().map(|(_, v)| v).collect();

                            let child = RuntimeTask::new(
                                self.log_sender.clone(),
                                sched_tx,
                                sched_rx,
                                child_id,
                                instructions,
                                self.global_memory.clone(),
                                self.modification_namespace_list.clone(),
                                slot_count,
                            );
                            // 입력 슬롯 주입
                            let mut child = child;
                            for (i, v) in ordered_inputs.into_iter().enumerate() {
                                if i < child.slots.len() {
                                    child.slots[i] = v;
                                }
                            }

                            self.tasks.insert(child_id, child);
                            self.states.insert(child_id, TaskState::Ready);
                            self.states.insert(
                                task_id,
                                TaskState::WaitingForReturn {
                                    callee_id: child_id,
                                    destination_slots,
                                },
                            );
                        }
                        None => {
                            // 호출 대상 함수를 찾지 못함 → Trap
                            let trap = RuntimeTaskTrap {
                                trapped_position: self
                                    .tasks
                                    .get(&task_id)
                                    .map(|t| t.instruction_pointer)
                                    .unwrap_or(0),
                                reason: crate::runtime_task::event::TrapReason::VerifierBug(
                                    format!("DefinedCall: function {function_identifier} not found"),
                                ),
                            };
                            traps.push((task_id, trap.clone()));
                            self.states.insert(task_id, TaskState::Trapped(trap));
                        }
                    }
                }

                // ── ReturnDefinedCall yield ───────────────────────────────────
                Ok(RuntimeTaskYield::Return {
                    function_identifier: _,
                    outputs,
                }) => {
                    self.states.insert(task_id, TaskState::Finished);
                    self.resume_parent_if_waiting(task_id, outputs);
                }

                // ── Trap ──────────────────────────────────────────────────────
                Err(trap) => {
                    traps.push((task_id, trap.clone()));
                    self.states.insert(task_id, TaskState::Trapped(trap));
                    // 이 태스크를 기다리던 부모가 있으면 빈 결과로 재개
                    // (부모가 오류를 감지할 수 있도록 Finished와 구분하지 않음;
                    //  정교한 오류 전파가 필요하다면 별도 상태를 추가한다)
                    self.resume_parent_if_waiting(task_id, vec![]);
                }
            }
        }

        traps
    }

    // -----------------------------------------------------------------------
    // 질의
    // -----------------------------------------------------------------------

    /// 모든 태스크가 종료(Finished 또는 Trapped) 상태인지 확인한다.
    pub fn is_idle(&self) -> bool {
        self.states.values().all(|s| matches!(s, TaskState::Finished | TaskState::Trapped(_)))
    }

    /// 특정 태스크의 슬롯 값을 읽는다.
    pub fn read_slots(&self, task_id: RuntimeTaskIdentifier) -> Option<&[Arc<RuntimeValue>]> {
        self.tasks.get(&task_id).map(|t| t.slots.as_slice())
    }

    // -----------------------------------------------------------------------
    // 내부 헬퍼
    // -----------------------------------------------------------------------

    /// function_identifier에 해당하는 함수를 찾아 (메타데이터, 명령 슬라이스) 를 반환한다.
    fn find_function(
        &self,
        id: FunctionIdentifier,
    ) -> Option<(&ConcreteFunctionMetadata, Arc<[ConcreteInstruction]>)> {
        self.functions
            .iter()
            .find(|(meta, _)| meta.function_identifier == id)
            .map(|(meta, instrs)| (meta, instrs.clone()))
    }

    /// 새 태스크 식별자를 발급한다.
    fn alloc_task_id(&mut self) -> RuntimeTaskIdentifier {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    /// 완료된 `child_id` 태스크를 기다리는 부모 태스크를 찾아 반환값을 주입하고
    /// Ready 상태로 전이한다.
    fn resume_parent_if_waiting(
        &mut self,
        child_id: RuntimeTaskIdentifier,
        outputs: Vec<Arc<RuntimeValue>>,
    ) {
        // WaitingForReturn { callee_id == child_id } 인 부모를 탐색
        let parent_id = self.states.iter().find_map(|(id, state)| {
            if let TaskState::WaitingForReturn { callee_id, .. } = state {
                if *callee_id == child_id {
                    return Some(*id);
                }
            }
            None
        });

        let Some(parent_id) = parent_id else { return };

        // destination_slots 추출
        let destination_slots =
            if let Some(TaskState::WaitingForReturn { destination_slots, .. }) =
                self.states.get(&parent_id)
            {
                destination_slots.clone()
            } else {
                return;
            };

        // 반환값을 destination_slots에 매핑
        let resume_map: HashMap<Slot, Arc<RuntimeValue>> = destination_slots
            .into_iter()
            .zip(outputs.into_iter())
            .collect();

        if let Some(parent_task) = self.tasks.get_mut(&parent_id) {
            // resume_call이 실패해도 태스크는 Ready로 표시 (검증 완료 명령 전제)
            let _ = parent_task.resume_call(resume_map);
        }

        self.states.insert(parent_id, TaskState::Ready);
    }

    /// 모든 활성 태스크의 슬롯 수 합계를 바이트 단위 RAM 사용 근사치로 반환한다.
    ///
    /// `RuntimeValue`의 실제 힙 크기를 측정하지 않고, 슬롯 1개당 `Arc` 포인터
    /// 크기(8 바이트)와 작은 상수 오버헤드를 더한 값으로 근사한다.
    fn estimate_ram_usage(&self) -> u64 {
        const BYTES_PER_SLOT: u64 = 64; // Arc + 내부 값 보수적 추정
        self.tasks
            .values()
            .map(|t| t.slots.len() as u64 * BYTES_PER_SLOT)
            .sum()
    }
}

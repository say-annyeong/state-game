주관적 진행도 예상치. 설계가 늘어나면 오히려 줄어들 예정. 100%시에는 API가 고정됨. 100%가 아니라면 API변경 및 동작 변경 가능성 존재. 100%의 경우 동결 표시가 없는 경우, 미미한 가능성으로 설계 변경 가능성 존재.

동결: 확정되어 어떠한 경우에도 설계가 수정되지 않는 상태.

state-game-compiler (0%)
- None

state-game-core (5%)
- ModificationSpecifications(30%) (비고: 주석 내용은 동결. 기본적인 제공 함수 3개도 동결. 확장 지점은 미정.)

state-game-mod (0%)
- None

state-game-runtime (75%)
- persistent_vector (100%) (비고: API동결 상태)
- runtime_task (60%)
  - verifier (80%)
  - instruction (95%) (비고: 함수 추가 이외의 다른 변경 사항 없을 예정)
  - runtime_task (80%)
  - scheduler (20%) (비고: 설계가 전부 반영되지 않음)
  - types (100%)
- runtime_engine (0%)
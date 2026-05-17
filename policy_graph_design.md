# 선언형 정책 그래프 설계 문서

## 개요

입력 값 · 함수 · 문맥을 기반으로 허용 여부를 결정하는 부분 평가 가능한 선언형 정책 그래프.
정책을 Rust 코드(클로저/트레이트)로 직접 정의하며, 논리식 AST 모델을 기반으로 동작한다.

---

## 핵심 원칙

| 레이어 | 역할 |
|--------|------|
| `AST` | 순수 구조 그래프. immutable. Data 직접 참조 불가. |
| `Data` | AST 외부 독립 공간. AST를 직접 포함 금지. Fn을 통해서만 생성. |
| `Fn` | AST ↔ Data 유일한 bridge. 반드시 최소 하나의 출력 생성. |

---

## 타입 정의

### `PolicyValue` — 결과 타입

```
Allow | Deny | Unknown
```

- 단순 열거형. 평가 결과만 표현.
- `Unknown`: Binding 미충족 상태. **평가 전 AST에서만 존재**.
- Kleene 3-valued logic 기반 AND / OR / NOT 연산.

**Kleene 연산 규칙**

| 연산 | 결과 |
|------|------|
| `Deny AND Unknown` | `Deny` |
| `Allow AND Unknown` | `Unknown` |
| `Allow OR Unknown` | `Allow` |
| `Deny OR Unknown` | `Unknown` |
| `NOT Unknown` | `Unknown` |

---

### `FnOutput` — Fn 반환 타입

```
FnOutput
  ├─ Ast(Expr)          // AST only
  ├─ Data(Data)         // Data only
  └─ Both(Expr, Data)   // AST + Data
```

- `(None, None)` 은 **타입 수준에서 완전 제거**.
- 세 상태만 허용되는 non-empty dual optional output.

---

### `Data`

```
Data(Box<dyn Any + Send + Sync>)
```

- AST 외부 독립 공간. Fn 체인 전용.
- AST를 직접 포함 금지.
- AST 의존은 결과 기반 간접 의존만 허용.
- Fn 노드 정의 시 기대 타입 명시 → graph 구성 시점에 타입 불일치 검출.

---

## AST 구조 (`Expr`)

```
Expr
  ├─ Binding(NodeId)
  │    외부 주입 대기 리프. 충족 전 Unknown.
  │
  ├─ Literal(PolicyValue)
  │    확정 리프.
  │
  ├─ Logic { id, op: LogicOp, children: Vec<Expr> }
  │    논리 연산 노드.
  │    DependencyGraph에 명시적으로 등록.
  │
  └─ Fn { id, fn_id, children: Vec<Expr>, params: Vec<ParamBinding> }
       함수 노드. 자식 노드가 입력 파라미터.
       반환: FnOutput
```

---

## 논리 연산자 (`LogicOp`)

내장(A)과 확장(B)을 혼합한 구조.

```
LogicOp
  ├─ And                        // 내장 (A)
  ├─ Or                         // 내장 (A)
  ├─ Not                        // 내장 (A)
  ├─ AtLeastN(usize)            // 내장 (A)
  └─ Custom(Arc<dyn CustomLogicOp>)  // 확장 (B)
```

- 간단한 연산: 내장 enum으로 컴파일 타임 확정.
- 복잡한 연산(`XOR`, `IMPLIES` 등): `CustomLogicOp` trait 구현으로 런타임 등록.

```rust
pub trait CustomLogicOp: Send + Sync {
    fn combine(&self, inputs: &[PolicyValue]) -> PolicyValue;
}
```

---

## Fn 노드 상세

### 시그니처

모든 Fn은 동일한 시그니처를 따른다.

```rust
fn(ast: Option<&Expr>, data: Option<&Data>, ctx: &EvalContext) -> FnOutput
```

### 자식 파라미터 매핑 (`ParamBinding`)

복수 자식일 때, 자식 인덱스와 파라미터 역할을 노드 정의 시 명시적으로 선언한다.

```
ParamBinding
  ├─ child_index: usize
  └─ role: ParamRole
       ├─ Ast       // 해당 자식의 Expr → ast 파라미터
       ├─ DataOnly  // 해당 자식의 Data → data 파라미터
       └─ Both      // Expr + Data 각각 전달
```

### 부모 타입별 FnOutput 처리 규칙

| FnOutput | 부모가 非Fn | 부모가 Fn |
|----------|------------|----------|
| `Ast(e)` | AST 삽입 후 평가 | `ast: Some(e), data: None` |
| `Data(d)` | placeholder 유지, evaluator ignore | `ast: None, data: Some(d)` |
| `Both(e, d)` | AST만 삽입, Data 버림 | `ast: Some(e), data: Some(d)` |

### 문자열 연산

`prefix`, `suffix`, `contains`는 Fn 노드로 구현.
`regex` 등 복잡한 연산은 외부가 `CustomLogicOp` 또는 Fn으로 등록.
문자열은 **값으로만 취급**. 의미 부여는 외부가 담당.

---

## 평가 구조

### `EvalContext`

읽기 전용. 쓰기 전면 금지 (pure function 보장).

```
EvalContext
  ├─ bindings:  HashMap<NodeId, PolicyValue>   // Fn 읽기 가능
  ├─ meta:      HashMap<String, String>        // Fn 읽기 가능
  └─ functions: FnRegistry                     // Fn 접근 불가 (pub 아님)
```

- Fn 노드가 접근 가능한 필드는 **노드 생성 시 화이트리스트로 선언**.
- `functions` 레지스트리는 `pub(crate)` — Fn 노드가 직접 접근 불가.

---

### `Expr` (평가 전 AST) vs `EvaluatedExpr` (평가 후)

| | `Expr` | `EvaluatedExpr` |
|---|---|---|
| 성격 | immutable 구조 | derived state (cache/overlay) |
| Unknown | 포함 가능 | **존재 불가** |
| 역할 | 정책 정의 | 평가 캐시 + 감사 |
| 수정 | 원본 유지, 재평가 가능 | diff 기반 부분 갱신 |

`EvaluatedExpr`는 AST가 아니라 **cache overlay**로 정의.
평가 후 AST는 모든 Binding이 충족된 시점에만 생성 가능.

---

### `EvaluatedExpr` — 캐시 + diff_log

```
EvaluatedExpr
  ├─ cache:    HashMap<NodeId, PolicyValue>   // 노드 단위 평가 캐시
  └─ diff_log: Vec<Diff>                      // 변경 사항 기록
```

```
Diff
  ├─ node_id: NodeId
  ├─ before:  PolicyValue
  └─ after:   PolicyValue
```

- `cache`: 노드 단위. `NodeId`를 키로 현재 평가 상태 저장.
- `diff_log`: audit + rollback 전용. Evaluator와 분리된 `AuditComponent`가 전담.

---

### `AuditComponent`

diff_log 읽기/쓰기 전담 별도 컴포넌트.

- **audit**: diff_log 순재생 → 변경 이력 조회.
- **rollback**: diff_log 역재생 → cache 복원.

---

## Dependency Graph (의존성 추적)

Logic 체인과 Fn 체인의 invalidate 전략을 분리.

| 체인 | 전략 | 이유 |
|------|------|------|
| Logic | **explicit** — `DependencyGraph` 참조 | 구조가 정적, 탐색 불필요 |
| Fn | **implicit** — Expr 트리 탐색 | 동적 AST 생성 가능성 |

```
DependencyGraph
  └─ parents: HashMap<NodeId, Vec<NodeId>>
       Logic 노드 생성 시 child → parent 등록
```

---

## Evaluator

```
Evaluator
  ├─ root:      Expr               // 원본 AST (immutable)
  ├─ evaluated: EvaluatedExpr      // 캐시 + diff_log
  └─ dep_graph: DependencyGraph    // Logic 체인 explicit 의존성
```

### 평가 흐름

```
eval(&ctx)
  └─ eval_expr(node)
       ├─ 캐시 히트 → 즉시 반환
       ├─ Binding → ctx.bindings 조회, 없으면 Unknown
       ├─ Logic   → 자식 평가 후 Kleene short-circuit 적용
       └─ Fn      → ParamBinding 기준 ast/data 수집 → 함수 호출 → FnOutput 처리
```

### invalidate와 short-circuit 분리

두 개념은 **완전히 분리**된 규칙.

**invalidate** (구조 규칙, 최적화 없음)
```
변경된 NodeId 확인
  ├─ Logic 조상: DependencyGraph 참조 → 탐색 없이 invalidate
  └─ Fn 조상:   Expr 트리 탐색 → 구조적 조상까지 invalidate
```

**short-circuit** (평가 규칙, eval 단계에서만)
```
And → 자식 중 Deny 확정 시 나머지 평가 중단
Or  → 자식 중 Allow 확정 시 나머지 평가 중단
```

### 재평가 흐름 (Expr 변경 시)

```
1. 변경된 NodeId 기준 invalidate 전파
     Logic 체인: DependencyGraph only
     Fn 체인:    Expr 트리 탐색
2. invalidate된 노드만 재평가
3. diff_log에 변경 사항 기록
```

- Fn 체인 invalidate: 구조적 조상까지 전파. 최적화 없음 (구조 규칙).
- Logic 체인 invalidate: `DependencyGraph`만 참조. 탐색 없음.

---

## Graph 분리

```
PolicyGraph   root: Expr → 출력: PolicyValue
DataGraph     root: Expr → 출력: Data (외부 호출자가 최종 해석)
```

- `PolicyGraph<T>` 제네릭 아님. 반환 타입 고정.
- `fn() -> Data`로 끝나는 트리: `DataGraph`로만 사용.
- `DataGraph` 출력 `Data`의 최종 해석은 **호출자(graph 외부)** 담당.

---

## 전체 구조 다이어그램

```
┌─────────────────────────────────────────────────────────┐
│                        PolicyGraph                      │
│                                                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │                   Evaluator                      │   │
│  │                                                  │   │
│  │  root: Expr (immutable AST)                      │   │
│  │  ┌─────────────────────────────────────────┐     │   │
│  │  │  Logic { And, Or, Not, AtLeastN, Custom }│     │   │
│  │  │    ├─ Binding(NodeId) ←── EvalContext    │     │   │
│  │  │    ├─ Literal(PolicyValue)               │     │   │
│  │  │    └─ Fn ──────────────────────────────┐│     │   │
│  │  │         ├─ fn() → Ast(Expr)            ││     │   │
│  │  │         ├─ fn() → Data(Data) ──────────┼┼──┐  │   │
│  │  │         └─ fn() → Both(Expr, Data) ────┼┼──┤  │   │
│  │  └─────────────────────────────────────────┘│  │  │   │
│  │                                              │  │  │   │
│  │  evaluated: EvaluatedExpr                    │  │  │   │
│  │  ┌──────────────────────────────────┐        │  │  │   │
│  │  │ cache:    NodeId → PolicyValue   │        │  │  │   │
│  │  │ diff_log: Vec<Diff>              │◄───────┘  │  │   │
│  │  └──────────────────────────────────┘           │  │   │
│  │            ▲ AuditComponent                     │  │   │
│  │                                                 │  │   │
│  │  dep_graph: DependencyGraph                     │  │   │
│  │  ┌──────────────────────────────────┐           │  │   │
│  │  │ Logic chain: explicit parents    │           │  │   │
│  │  │ Fn chain:    implicit traversal  │           │  │   │
│  │  └──────────────────────────────────┘           │  │   │
│  └──────────────────────────────────────────────── ┘  │   │
│                                                        │   │
│                              Data side-channel ────────┘   │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
                    PolicyValue
               (Allow | Deny | Unknown)
```

---

## 설계 결정 요약

| 항목 | 결정 |
|------|------|
| 문자열 의미 부여 | 외부. 구조는 값으로만 취급 |
| 문자열 내장 연산 | `prefix`, `suffix`, `contains` |
| 논리 연산자 | 내장 enum + 확장 trait 혼합 |
| Fn 반환 타입 | `FnOutput` enum (None,None 타입 제거) |
| Data 흐름 | Fn 체인 전용. AST 레이어 진입 불가 |
| invalidate | 구조 규칙. short-circuit과 완전 분리 |
| short-circuit | eval 단계에서만. Kleene logic 적용 |
| Logic dependency | explicit (`DependencyGraph`) |
| Fn dependency | implicit (Expr 트리 탐색) |
| 히스토리 | cache + diff_log (full snapshot 없음) |
| audit/rollback | `AuditComponent` 전담. Evaluator와 분리 |
| Graph 반환 타입 | `PolicyGraph → PolicyValue` / `DataGraph → Data` |
| EvalContext 접근 | bindings/meta 읽기 가능. functions 접근 불가. 쓰기 금지 |

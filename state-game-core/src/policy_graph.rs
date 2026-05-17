//! 선언형 정책 그래프
//!
//! 핵심 구조:
//!   AST  = 순수 구조 그래프 (immutable)
//!   Data = AST 외부 독립 공간 (side-channel)
//!   Fn   = AST ↔ Data 유일한 bridge
//!
//! PolicyValue  = Allow | Deny | Unknown
//! FnOutput     = Ast(Expr) | Data(Data) | Both(Expr, Data)  — (None,None) 타입 수준 제거
//! EvaluatedExpr = cache(NodeId → PolicyValue) + diff_log (audit + rollback)
//! DependencyGraph: Logic 체인 explicit / Fn 체인 implicit

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────
// 1. 기본 타입
// ─────────────────────────────────────────────────────────────

/// 노드 고유 ID — Binding 추적 / 캐시 키 / dependency graph 키
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// 함수 식별자
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnId(pub String);

/// AST 외부 독립 공간. Fn 체인 전용.
/// AST를 직접 포함 금지. AST 의존은 결과 기반 간접 의존만 허용.
pub struct Data(pub Box<dyn Any + Send + Sync>);

impl Data {
    pub fn new<T: Any + Send + Sync>(val: T) -> Self {
        Self(Box::new(val))
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }
}

// ─────────────────────────────────────────────────────────────
// 2. PolicyValue — 결과 타입 (단순 열거형)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyValue {
    Allow,
    Deny,
    /// Binding 미충족 — 평가 전 AST에서만 존재
    Unknown,
}

impl PolicyValue {
    /// Kleene AND
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Deny,  _           ) => Self::Deny,
            (_,           Self::Deny  ) => Self::Deny,
            (Self::Allow, Self::Allow ) => Self::Allow,
            _                          => Self::Unknown,
        }
    }

    /// Kleene OR
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Allow, _           ) => Self::Allow,
            (_,           Self::Allow ) => Self::Allow,
            (Self::Deny,  Self::Deny  ) => Self::Deny,
            _                          => Self::Unknown,
        }
    }

    pub fn not(self) -> Self {
        match self {
            Self::Allow   => Self::Deny,
            Self::Deny    => Self::Allow,
            Self::Unknown => Self::Unknown,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 3. LogicOp — 내장 enum + 확장 trait 혼합
// ─────────────────────────────────────────────────────────────

/// 복잡한 논리 연산자 확장 trait (B: 런타임 등록 가능)
pub trait CustomLogicOp: Send + Sync {
    fn combine(&self, inputs: &[PolicyValue]) -> PolicyValue;
}

/// 논리 연산자 — 간단한 것은 내장 enum (A), 복잡한 것은 trait (B)로 위임
pub enum LogicOp {
    // ── 내장 (A) ──────────────────────────────────────────
    And,
    Or,
    Not,
    AtLeastN(usize),
    // ── 확장 (B) ──────────────────────────────────────────
    Custom(Arc<dyn CustomLogicOp>),
}

impl LogicOp {
    pub fn combine(&self, inputs: &[PolicyValue]) -> PolicyValue {
        match self {
            Self::And => inputs.iter().copied().fold(PolicyValue::Allow, PolicyValue::and),
            Self::Or  => inputs.iter().copied().fold(PolicyValue::Deny,  PolicyValue::or),
            Self::Not => {
                assert_eq!(inputs.len(), 1, "Not requires exactly one input");
                inputs[0].not()
            }
            Self::AtLeastN(n) => {
                let count = inputs.iter().filter(|&&v| v == PolicyValue::Allow).count();
                if count >= *n { PolicyValue::Allow } else { PolicyValue::Deny }
            }
            Self::Custom(op) => op.combine(inputs),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 4. FnOutput — (None, None) 타입 수준 완전 제거
// ─────────────────────────────────────────────────────────────

/// Fn 노드의 반환 타입.
/// 세 상태만 허용 — (None, None) 불가능.
pub enum FnOutput {
    /// AST only
    Ast(Expr),
    /// Data only
    Data(Data),
    /// AST + Data
    Both(Expr, Data),
}

impl FnOutput {
    pub fn into_parts(self) -> (Option<Expr>, Option<Data>) {
        match self {
            Self::Ast(e)     => (Some(e), None),
            Self::Data(d)    => (None,    Some(d)),
            Self::Both(e, d) => (Some(e), Some(d)),
        }
    }
    pub fn expr_ref(&self) -> Option<&Expr> {
        match self { Self::Ast(e) | Self::Both(e, _) => Some(e), _ => None }
    }
    pub fn data_ref(&self) -> Option<&Data> {
        match self { Self::Data(d) | Self::Both(_, d) => Some(d), _ => None }
    }
}

// ─────────────────────────────────────────────────────────────
// 5. ParamBinding — 자식 인덱스 → 파라미터 역할 매핑
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ParamRole {
    /// 해당 자식의 Expr → ast 파라미터
    Ast,
    /// 해당 자식의 Data → data 파라미터
    DataOnly,
    /// 해당 자식의 Expr + Data → 각각 전달
    Both,
}

#[derive(Debug, Clone)]
pub struct ParamBinding {
    pub child_index: usize,
    pub role:        ParamRole,
}

// ─────────────────────────────────────────────────────────────
// 6. Expr — 평가 전 AST (immutable)
// ─────────────────────────────────────────────────────────────

pub enum Expr {
    /// 외부 주입 대기 리프. 충족 전 Unknown.
    Binding(NodeId),

    /// 확정 리프
    Literal(PolicyValue),

    /// 논리 연산 노드 (Logic 체인 — DependencyGraph에 명시 등록)
    Logic {
        id:       NodeId,
        op:       LogicOp,
        children: Vec<Expr>,
    },

    /// 함수 노드 — AST ↔ Data 유일한 bridge
    /// 자식 노드가 입력 파라미터.
    /// 반환: FnOutput (Ast | Data | Both)
    Fn {
        id:       NodeId,
        fn_id:    FnId,
        children: Vec<Expr>,
        /// 자식 인덱스 → 파라미터 역할 명시적 매핑
        params:   Vec<ParamBinding>,
    },
}

impl Expr {
    pub fn node_id(&self) -> Option<NodeId> {
        match self {
            Self::Binding(id)        => Some(*id),
            Self::Literal(_)         => None,
            Self::Logic { id, .. }   => Some(*id),
            Self::Fn    { id, .. }   => Some(*id),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 7. EvalContext — 읽기 전용, 쓰기 전면 금지
// ─────────────────────────────────────────────────────────────

type FnRegistry = HashMap<
    FnId,
    Arc<dyn Fn(Option<&Expr>, Option<&Data>, &EvalContext) -> FnOutput + Send + Sync>,
>;

pub struct EvalContext {
    /// Binding 충족 값 — Fn 읽기 가능
    pub bindings:  HashMap<NodeId, PolicyValue>,
    /// 문맥 메타데이터 — Fn 읽기 가능
    pub meta:      HashMap<String, String>,
    /// 함수 레지스트리 — Fn 접근 불가 (pub 아님)
    functions:     FnRegistry,
}

impl EvalContext {
    pub fn new() -> Self {
        Self { bindings: HashMap::new(), meta: HashMap::new(), functions: HashMap::new() }
    }

    pub fn bind(mut self, id: NodeId, val: PolicyValue) -> Self {
        self.bindings.insert(id, val);
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.meta.insert(key.into(), val.into());
        self
    }

    pub fn register_fn(
        &mut self,
        id: FnId,
        f: impl Fn(Option<&Expr>, Option<&Data>, &EvalContext) -> FnOutput + Send + Sync + 'static,
    ) {
        self.functions.insert(id, Arc::new(f));
    }

    /// Fn 노드 전용 내부 호출 — functions 필드 직접 노출 없음
    pub(crate) fn call_fn(
        &self,
        id: &FnId,
        ast: Option<&Expr>,
        data: Option<&Data>,
    ) -> Option<FnOutput> {
        self.functions.get(id).map(|f| f(ast, data, self))
    }
}

// ─────────────────────────────────────────────────────────────
// 8. EvaluatedExpr — 노드 단위 캐시 + diff_log
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Diff {
    pub node_id: NodeId,
    pub before:  PolicyValue,
    pub after:   PolicyValue,
}

/// EvaluatedExpr = cache overlay (derived state) + diff_log
/// audit:    diff_log 순재생
/// rollback: diff_log 역재생 → cache 복원
/// AuditComponent가 diff_log 읽기/쓰기 전담
pub struct EvaluatedExpr {
    cache:    HashMap<NodeId, PolicyValue>,
    diff_log: Vec<Diff>,
}

impl EvaluatedExpr {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), diff_log: Vec::new() }
    }

    pub(crate) fn set(&mut self, id: NodeId, val: PolicyValue) {
        let before = self.cache.get(&id).copied().unwrap_or(PolicyValue::Unknown);
        if before != val {
            self.diff_log.push(Diff { node_id: id, before, after: val });
            self.cache.insert(id, val);
        }
    }

    pub(crate) fn get(&self, id: &NodeId) -> Option<PolicyValue> {
        self.cache.get(id).copied()
    }

    pub(crate) fn invalidate(&mut self, id: &NodeId) {
        if let Some(old) = self.cache.remove(id) {
            self.diff_log.push(Diff {
                node_id: *id,
                before:  old,
                after:   PolicyValue::Unknown,
            });
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 9. AuditComponent — diff_log 전담 (별도 컴포넌트)
// ─────────────────────────────────────────────────────────────

pub struct AuditComponent;

impl AuditComponent {
    /// diff_log 순재생 → 변경 이력 조회
    pub fn audit(evaluated: &EvaluatedExpr) -> &[Diff] {
        &evaluated.diff_log
    }

    /// diff_log 역재생 → 특정 steps만큼 cache 복원
    pub fn rollback(evaluated: &mut EvaluatedExpr, steps: usize) {
        let from = evaluated.diff_log.len().saturating_sub(steps);
        let reverts: Vec<Diff> = evaluated.diff_log.drain(from..).collect();
        for diff in reverts.into_iter().rev() {
            if diff.before == PolicyValue::Unknown {
                evaluated.cache.remove(&diff.node_id);
            } else {
                evaluated.cache.insert(diff.node_id, diff.before);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 10. DependencyGraph — Logic: explicit / Fn: implicit
// ─────────────────────────────────────────────────────────────

pub struct DependencyGraph {
    /// Logic 체인 전용: child NodeId → 부모 NodeId 목록
    /// invalidate 시 탐색 없이 이 맵만 참조
    parents: HashMap<NodeId, Vec<NodeId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self { parents: HashMap::new() }
    }

    pub fn register_parent(&mut self, child: NodeId, parent: NodeId) {
        self.parents.entry(child).or_default().push(parent);
    }

    /// Logic 체인 조상 목록 반환 (탐색 없음)
    pub fn logic_ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut queue  = vec![id];
        while let Some(cur) = queue.pop() {
            if let Some(ps) = self.parents.get(&cur) {
                for &p in ps {
                    result.push(p);
                    queue.push(p);
                }
            }
        }
        result
    }
}

// ─────────────────────────────────────────────────────────────
// 11. Evaluator
// ─────────────────────────────────────────────────────────────

pub struct Evaluator {
    pub root:      Expr,
    pub evaluated: EvaluatedExpr,
    pub dep_graph: DependencyGraph,
}

impl Evaluator {
    pub fn new(root: Expr, dep_graph: DependencyGraph) -> Self {
        Self { root, evaluated: EvaluatedExpr::new(), dep_graph }
    }

    pub fn eval(&mut self, ctx: &EvalContext) -> PolicyValue {
        self.eval_expr(&self.root as *const Expr, ctx)
            .unwrap_or(PolicyValue::Unknown)
    }

    /// 변경된 노드 기준 invalidate + 재평가
    pub fn invalidate_and_reeval(&mut self, changed_id: NodeId, ctx: &EvalContext) -> PolicyValue {
        // 1. Logic 체인 invalidate (explicit — DependencyGraph 참조)
        for id in self.dep_graph.logic_ancestors(changed_id) {
            self.evaluated.invalidate(&id);
        }
        self.evaluated.invalidate(&changed_id);

        // 2. Fn 체인 invalidate (implicit — Expr 트리 탐색)
        Self::invalidate_fn_chain(&mut self.evaluated, &self.root, changed_id);

        // 3. 재평가
        self.eval(ctx)
    }

    /// Fn 체인 조상 invalidate (재귀 탐색)
    /// 구조 규칙만 적용 — short-circuit 없음
    fn invalidate_fn_chain(ev: &mut EvaluatedExpr, expr: &Expr, target: NodeId) -> bool {
        match expr {
            Expr::Binding(id)    => *id == target,
            Expr::Literal(_)     => false,
            Expr::Logic { id, children, .. } => {
                // Logic 자식 탐색 (Fn 체인 연결 가능성)
                let hit = children.iter().any(|c| Self::invalidate_fn_chain(ev, c, target));
                if hit { ev.invalidate(id); }
                hit
            }
            Expr::Fn { id, children, .. } => {
                let hit = children.iter().any(|c| Self::invalidate_fn_chain(ev, c, target))
                    || id.0 == target.0;
                if hit { ev.invalidate(id); }
                hit
            }
        }
    }

    /// 노드 평가
    /// - 캐시 히트 시 즉시 반환
    /// - Logic: Kleene short-circuit (eval 단계에서만)
    /// - Fn: ParamBinding 기준 ast/data 수집 후 함수 호출
    fn eval_expr(&mut self, expr_ptr: *const Expr, ctx: &EvalContext) -> Option<PolicyValue> {
        // SAFETY: Evaluator가 root를 소유하며 eval 중 변경 없음
        let expr = unsafe { &*expr_ptr };
        match expr {
            Expr::Literal(v) => Some(*v),

            Expr::Binding(id) => {
                if let Some(cached) = self.evaluated.get(id) {
                    return Some(cached);
                }
                let val = ctx.bindings.get(id).copied().unwrap_or(PolicyValue::Unknown);
                self.evaluated.set(*id, val);
                Some(val)
            }

            Expr::Logic { id, op, children } => {
                if let Some(cached) = self.evaluated.get(id) {
                    return Some(cached);
                }

                let mut collected = Vec::with_capacity(children.len());
                for child in children {
                    let v = self.eval_expr(child as *const Expr, ctx)
                        .unwrap_or(PolicyValue::Unknown);
                    // Kleene short-circuit — eval 단계에서만 적용
                    // (invalidate는 구조 규칙으로 분리되어 있음)
                    match op {
                        LogicOp::And | LogicOp::Custom(_) if v == PolicyValue::Deny => {
                            self.evaluated.set(*id, PolicyValue::Deny);
                            return Some(PolicyValue::Deny);
                        }
                        LogicOp::Or if v == PolicyValue::Allow => {
                            self.evaluated.set(*id, PolicyValue::Allow);
                            return Some(PolicyValue::Allow);
                        }
                        _ => {}
                    }
                    collected.push(v);
                }

                let result = op.combine(&collected);
                self.evaluated.set(*id, result);
                Some(result)
            }

            Expr::Fn { id, fn_id, children, params } => {
                if let Some(cached) = self.evaluated.get(id) {
                    return Some(cached);
                }

                // ParamBinding에 따라 자식 평가 결과를 ast/data로 분리
                let mut ast_input:  Option<Expr> = None;
                let mut data_input: Option<Data> = None;

                for binding in params {
                    let Some(child) = children.get(binding.child_index) else { continue };

                    // 자식이 Fn이면 FnOutput으로, 아니면 PolicyValue→Literal로 래핑
                    let output = self.eval_child_as_fn_output(child as *const Expr, ctx);
                    let Some(output) = output else { continue };

                    let (expr_part, data_part) = output.into_parts();
                    match binding.role {
                        ParamRole::Ast      => { ast_input  = expr_part; }
                        ParamRole::DataOnly => { data_input = data_part; }
                        ParamRole::Both     => {
                            if ast_input.is_none()  { ast_input  = expr_part; }
                            if data_input.is_none() { data_input = data_part; }
                        }
                    }
                }

                // 함수 호출
                let output = ctx.call_fn(fn_id, ast_input.as_ref(), data_input.as_ref());

                match output {
                    None => None,

                    Some(FnOutput::Data(_)) => {
                        // 부모가 非Fn: placeholder 유지, evaluator ignore
                        None
                    }

                    Some(FnOutput::Ast(new_expr)) | Some(FnOutput::Both(new_expr, _)) => {
                        // 동적 AST 삽입 후 즉시 평가
                        // Both의 Data는 부모가 非Fn이므로 버림
                        let val = self.eval_expr(&new_expr as *const Expr, ctx)
                            .unwrap_or(PolicyValue::Unknown);
                        self.evaluated.set(*id, val);
                        Some(val)
                    }
                }
            }
        }
    }

    /// 자식 노드를 FnOutput으로 평가
    /// Fn 노드 → 함수 호출 결과 그대로
    /// 非Fn 노드 → PolicyValue로 평가 후 Ast(Literal)로 래핑
    fn eval_child_as_fn_output(
        &mut self,
        expr_ptr: *const Expr,
        ctx: &EvalContext,
    ) -> Option<FnOutput> {
        let expr = unsafe { &*expr_ptr };
        match expr {
            Expr::Fn { fn_id, children, params, .. } => {
                let mut ast_input:  Option<Expr> = None;
                let mut data_input: Option<Data> = None;

                for binding in params {
                    let Some(child) = children.get(binding.child_index) else { continue };
                    let output = self.eval_child_as_fn_output(child as *const Expr, ctx);
                    let Some(output) = output else { continue };
                    let (ep, dp) = output.into_parts();
                    match binding.role {
                        ParamRole::Ast      => { ast_input  = ep; }
                        ParamRole::DataOnly => { data_input = dp; }
                        ParamRole::Both     => {
                            if ast_input.is_none()  { ast_input  = ep; }
                            if data_input.is_none() { data_input = dp; }
                        }
                    }
                }

                ctx.call_fn(fn_id, ast_input.as_ref(), data_input.as_ref())
            }
            other => {
                let val = self.eval_expr(other as *const Expr, ctx)
                    .unwrap_or(PolicyValue::Unknown);
                Some(FnOutput::Ast(Expr::Literal(val)))
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 12. PolicyGraph / DataGraph
// ─────────────────────────────────────────────────────────────

/// 출력: PolicyValue
pub struct PolicyGraph {
    pub evaluator: Evaluator,
}

impl PolicyGraph {
    pub fn new(root: Expr, dep_graph: DependencyGraph) -> Self {
        Self { evaluator: Evaluator::new(root, dep_graph) }
    }

    pub fn evaluate(&mut self, ctx: &EvalContext) -> PolicyValue {
        self.evaluator.eval(ctx)
    }

    pub fn invalidate_and_reeval(&mut self, changed_id: NodeId, ctx: &EvalContext) -> PolicyValue {
        self.evaluator.invalidate_and_reeval(changed_id, ctx)
    }

    pub fn audit(&self) -> &[Diff] {
        AuditComponent::audit(&self.evaluator.evaluated)
    }

    pub fn rollback(&mut self, steps: usize) {
        AuditComponent::rollback(&mut self.evaluator.evaluated, steps);
    }
}

/// 출력: Data — 외부 호출자가 최종 해석
pub struct DataGraph {
    pub evaluator: Evaluator,
}

impl DataGraph {
    pub fn new(root: Expr, dep_graph: DependencyGraph) -> Self {
        Self { evaluator: Evaluator::new(root, dep_graph) }
    }

    pub fn evaluate(&mut self, ctx: &EvalContext) -> Option<Data> {
        let root_ptr = &self.evaluator.root as *const Expr;
        let output = self.evaluator.eval_child_as_fn_output(root_ptr, ctx)?;
        let (_, data) = output.into_parts();
        data
    }
}

// ─────────────────────────────────────────────────────────────
// 13. 테스트
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(n: u64) -> NodeId { NodeId(n) }
    fn fid(s: &str) -> FnId  { FnId(s.into()) }

    // ── PolicyValue Kleene laws ───────────────────────────────

    #[test]
    fn kleene_and() {
        assert_eq!(PolicyValue::Deny.and(PolicyValue::Unknown),  PolicyValue::Deny);
        assert_eq!(PolicyValue::Allow.and(PolicyValue::Unknown), PolicyValue::Unknown);
        assert_eq!(PolicyValue::Allow.and(PolicyValue::Allow),   PolicyValue::Allow);
    }

    #[test]
    fn kleene_or() {
        assert_eq!(PolicyValue::Allow.or(PolicyValue::Unknown),  PolicyValue::Allow);
        assert_eq!(PolicyValue::Deny.or(PolicyValue::Unknown),   PolicyValue::Unknown);
        assert_eq!(PolicyValue::Deny.or(PolicyValue::Deny),      PolicyValue::Deny);
    }

    #[test]
    fn kleene_not() {
        assert_eq!(PolicyValue::Allow.not(),   PolicyValue::Deny);
        assert_eq!(PolicyValue::Deny.not(),    PolicyValue::Allow);
        assert_eq!(PolicyValue::Unknown.not(), PolicyValue::Unknown);
    }

    // ── 기본 평가 ─────────────────────────────────────────────

    #[test]
    fn literal_evaluates() {
        let mut g = PolicyGraph::new(
            Expr::Literal(PolicyValue::Allow),
            DependencyGraph::new(),
        );
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Allow);
    }

    #[test]
    fn binding_resolves_from_context() {
        let id = nid(1);
        let mut g = PolicyGraph::new(Expr::Binding(id), DependencyGraph::new());
        let ctx = EvalContext::new().bind(id, PolicyValue::Allow);
        assert_eq!(g.evaluate(&ctx), PolicyValue::Allow);
    }

    #[test]
    fn binding_unknown_when_missing() {
        let mut g = PolicyGraph::new(Expr::Binding(nid(1)), DependencyGraph::new());
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Unknown);
    }

    // ── Logic 노드 ────────────────────────────────────────────

    #[test]
    fn logic_and_deny_short_circuit() {
        let id = nid(10);
        let mut dep = DependencyGraph::new();
        dep.register_parent(nid(99), id);

        let expr = Expr::Logic {
            id,
            op: LogicOp::And,
            children: vec![
                Expr::Literal(PolicyValue::Deny),
                Expr::Binding(nid(99)), // Unknown
            ],
        };
        let mut g = PolicyGraph::new(expr, dep);
        // Deny AND Unknown → Deny (short-circuit)
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Deny);
    }

    #[test]
    fn logic_or_allow_short_circuit() {
        let id = nid(11);
        let expr = Expr::Logic {
            id,
            op: LogicOp::Or,
            children: vec![
                Expr::Literal(PolicyValue::Allow),
                Expr::Binding(nid(99)), // Unknown
            ],
        };
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        // Allow OR Unknown → Allow (short-circuit)
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Allow);
    }

    #[test]
    fn logic_at_least_n() {
        let expr = Expr::Logic {
            id: nid(20),
            op: LogicOp::AtLeastN(2),
            children: vec![
                Expr::Literal(PolicyValue::Allow),
                Expr::Literal(PolicyValue::Allow),
                Expr::Literal(PolicyValue::Deny),
            ],
        };
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Allow);
    }

    #[test]
    fn logic_custom_op() {
        // XOR: 홀수 개 Allow일 때만 Allow
        struct Xor;
        impl CustomLogicOp for Xor {
            fn combine(&self, inputs: &[PolicyValue]) -> PolicyValue {
                let count = inputs.iter().filter(|&&v| v == PolicyValue::Allow).count();
                if count % 2 == 1 { PolicyValue::Allow } else { PolicyValue::Deny }
            }
        }

        let expr = Expr::Logic {
            id: nid(30),
            op: LogicOp::Custom(Arc::new(Xor)),
            children: vec![
                Expr::Literal(PolicyValue::Allow),
                Expr::Literal(PolicyValue::Allow), // 2개 → Deny
            ],
        };
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        assert_eq!(g.evaluate(&EvalContext::new()), PolicyValue::Deny);
    }

    // ── Fn 노드 ───────────────────────────────────────────────

    #[test]
    fn fn_ast_output_evaluated() {
        let fn_id = fid("always_allow");
        let expr = Expr::Fn {
            id: nid(40),
            fn_id: fn_id.clone(),
            children: vec![],
            params: vec![],
        };
        let mut ctx = EvalContext::new();
        ctx.register_fn(fn_id, |_, _, _| {
            FnOutput::Ast(Expr::Literal(PolicyValue::Allow))
        });
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        assert_eq!(g.evaluate(&ctx), PolicyValue::Allow);
    }

    #[test]
    fn fn_data_only_ignored_by_non_fn_parent() {
        // 非Fn 부모 아래 fn() -> Data → ignore → And(Allow, None) → Unknown
        let fn_id = fid("emit_data");
        let expr = Expr::Logic {
            id: nid(50),
            op: LogicOp::And,
            children: vec![
                Expr::Literal(PolicyValue::Allow),
                Expr::Fn {
                    id: nid(51),
                    fn_id: fn_id.clone(),
                    children: vec![],
                    params: vec![],
                },
            ],
        };
        let mut ctx = EvalContext::new();
        ctx.register_fn(fn_id, |_, _, _| {
            FnOutput::Data(Data::new(42u32))
        });
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        // fn() -> Data는 ignore → And(Allow, Unknown) → Unknown
        assert_eq!(g.evaluate(&ctx), PolicyValue::Unknown);
    }

    #[test]
    fn fn_both_data_discarded_by_non_fn_parent() {
        // 非Fn 부모 아래 fn() -> (AST, Data) → AST만 삽입, Data 버림
        let fn_id = fid("both_output");
        let expr = Expr::Fn {
            id: nid(60),
            fn_id: fn_id.clone(),
            children: vec![],
            params: vec![],
        };
        let mut ctx = EvalContext::new();
        ctx.register_fn(fn_id, |_, _, _| {
            FnOutput::Both(
                Expr::Literal(PolicyValue::Deny),
                Data::new("side_data"),
            )
        });
        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        assert_eq!(g.evaluate(&ctx), PolicyValue::Deny);
    }

    #[test]
    fn fn_param_binding_passes_data_to_parent_fn() {
        // fn_child → Data("hello")
        // fn_parent receives data: Some("hello"), returns Ast(Allow)
        let child_id = fid("child");
        let parent_id = fid("parent");

        let expr = Expr::Fn {
            id: nid(70),
            fn_id: parent_id.clone(),
            children: vec![
                Expr::Fn {
                    id: nid(71),
                    fn_id: child_id.clone(),
                    children: vec![],
                    params: vec![],
                },
            ],
            params: vec![ParamBinding { child_index: 0, role: ParamRole::DataOnly }],
        };

        let mut ctx = EvalContext::new();
        ctx.register_fn(child_id, |_, _, _| {
            FnOutput::Data(Data::new(99u32))
        });
        ctx.register_fn(parent_id, |_, data, _| {
            let val = data
                .and_then(|d| d.downcast_ref::<u32>())
                .map(|&n| if n == 99 { PolicyValue::Allow } else { PolicyValue::Deny })
                .unwrap_or(PolicyValue::Unknown);
            FnOutput::Ast(Expr::Literal(val))
        });

        let mut g = PolicyGraph::new(expr, DependencyGraph::new());
        assert_eq!(g.evaluate(&ctx), PolicyValue::Allow);
    }

    // ── EvaluatedExpr 캐시 + diff_log ────────────────────────

    #[test]
    fn cache_hit_avoids_reeval() {
        let id = nid(1);
        let mut g = PolicyGraph::new(Expr::Binding(id), DependencyGraph::new());
        let ctx = EvalContext::new().bind(id, PolicyValue::Allow);
        g.evaluate(&ctx);
        // 두 번째 evaluate — 캐시에서 반환
        assert_eq!(g.evaluate(&ctx), PolicyValue::Allow);
        // diff_log는 첫 평가 시 1개만 기록
        assert_eq!(g.audit().len(), 1);
    }

    #[test]
    fn invalidate_and_reeval_updates_cache() {
        let id = nid(1);
        let mut g = PolicyGraph::new(Expr::Binding(id), DependencyGraph::new());

        let ctx1 = EvalContext::new().bind(id, PolicyValue::Allow);
        g.evaluate(&ctx1);

        let ctx2 = EvalContext::new().bind(id, PolicyValue::Deny);
        let result = g.invalidate_and_reeval(id, &ctx2);
        assert_eq!(result, PolicyValue::Deny);
    }

    #[test]
    fn rollback_restores_previous_state() {
        let id = nid(1);
        let mut g = PolicyGraph::new(Expr::Binding(id), DependencyGraph::new());

        let ctx1 = EvalContext::new().bind(id, PolicyValue::Allow);
        g.evaluate(&ctx1);

        let ctx2 = EvalContext::new().bind(id, PolicyValue::Deny);
        g.invalidate_and_reeval(id, &ctx2);

        let before_rollback = g.audit().len();
        g.rollback(2); // invalidate + set 두 diff 역재생
        // rollback 후 diff_log 감소
        assert!(g.audit().len() < before_rollback);
    }

    // ── FnOutput enum ─────────────────────────────────────────

    #[test]
    fn fn_output_no_none_none() {
        // 컴파일 타임에 (None, None) 불가 — 세 케이스만 존재함을 확인
        let _ast  = FnOutput::Ast(Expr::Literal(PolicyValue::Allow));
        let _data = FnOutput::Data(Data::new(0u8));
        let _both = FnOutput::Both(Expr::Literal(PolicyValue::Deny), Data::new(0u8));
    }
}

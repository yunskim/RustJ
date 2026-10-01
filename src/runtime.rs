use crate::{
    error::{
        ArgumentRole, ArgumentSummary, DiagnosticPhase, DiagnosticValence, Error, ErrorContext,
        Result,
    },
    kernels,
    semantic::{FunctionEntity, FunctionHead, FunctionOperand},
    value::Value,
};
use std::collections::HashMap;

pub struct Engine {
    names: HashMap<String, Binding>,
    pool: crate::pool::OutputPool,
}

enum SymbolValue {
    Noun(Value),
    Verb(crate::semantic::Verb),
}

struct ResolvedVerb {
    id: crate::primitive::PrimitiveId,
    reduce: bool,
    rank: Option<[i64; 3]>,
}

fn argument_summary(role: ArgumentRole, value: &Value) -> ArgumentSummary {
    ArgumentSummary {
        role,
        type_code: value.type_code(),
        shape: value.shape().to_vec(),
    }
}

fn operation_label(verb: &ResolvedVerb) -> String {
    let base = verb.id.spelling();
    if verb.reduce {
        format!("{base}/")
    } else if let Some(rank) = verb.rank {
        format!("{base}\"{} {} {}", rank[0], rank[1], rank[2])
    } else {
        base.to_owned()
    }
}


struct Binding {
    value: SymbolValue,
    version: crate::semantic::NameVersion,
}

impl Default for Engine {
    fn default() -> Self {
        Self::with_output_cache_limit(64 * 1024 * 1024)
    }
}
impl Engine {
    /// Limits retained integer payload bytes. Zero disables caching.
    pub fn with_output_cache_limit(bytes: usize) -> Self {
        Self {
            names: HashMap::new(),
            pool: crate::pool::OutputPool::new(bytes),
        }
    }
    /// Retained payload capacity in bytes and cumulative reuse count.
    pub fn output_cache_stats(&self) -> (usize, usize) {
        self.pool.stats()
    }
    /// Return cached buffers to the allocator; RSS may not decrease.
    pub fn clear_output_cache(&mut self) {
        self.pool.clear();
    }

    pub fn new() -> Self {
        Self::default()
    }
    /// Inspect bindings without execution or mutation. Versions are Engine-local.
    /// Stable machine API: diagnostic wrappers are stripped before return.
    pub fn prepare_semantic(&self, source: &str) -> Result<crate::semantic::BoundProgram> {
        self.prepare_semantic_diagnostic(source)
            .map_err(Error::into_unlocated)
    }

    pub fn prepare_semantic_diagnostic(
        &self,
        source: &str,
    ) -> Result<crate::semantic::BoundProgram> {
        crate::semantic::bind(
            crate::semantic::parse_analysis(source, &|name| match &self.names.get(name)?.value {
                SymbolValue::Noun(value) => Some(value.clone()),
                SymbolValue::Verb(_) => None,
            })?,
            |name| self.binding_version(name),
        )
        .map_err(|error| error.in_phase(DiagnosticPhase::SemanticAnalysis))
    }

    /// Build the J-grammar-preserving applied computation graph.
    /// This IR keeps Hook/Fork/@:/modifier identity while also exposing their
    /// applied stage/branch graph, propagated static facts and graph-analysis
    /// contracts before execution/basis lowering.
    pub fn analyze_j_graph(&self, source: &str) -> Result<crate::j_graph_ir::Plan> {
        self.analyze_j_graph_diagnostic(source)
            .map_err(Error::into_unlocated)
    }

    pub fn analyze_j_graph_diagnostic(
        &self,
        source: &str,
    ) -> Result<crate::j_graph_ir::Plan> {
        crate::j_graph_ir::Plan::from_bound_with_graph_facts(
            self.prepare_semantic_diagnostic(source)?,
            &|name| match self.names.get(name).map(|binding| &binding.value) {
                Some(SymbolValue::Noun(value)) => crate::j_graph_ir::GraphFacts::of(value),
                _ => crate::j_graph_ir::GraphFacts::default(),
            },
        )
        .map_err(|error| error.in_phase(DiagnosticPhase::SemanticAnalysis))
    }

    /// Analyze both compiler IR views: the J-grammar graph and the
    /// execution-oriented logical plan derived from it.
    pub fn analyze_compilation(
        &self,
        source: &str,
    ) -> Result<crate::analysis::CompilationAnalysis> {
        self.analyze_compilation_diagnostic(source)
            .map_err(Error::into_unlocated)
    }

    pub fn analyze_compilation_diagnostic(
        &self,
        source: &str,
    ) -> Result<crate::analysis::CompilationAnalysis> {
        let j_graph = self.analyze_j_graph_diagnostic(source)?;
        let graph_rewrites = j_graph.rewrite_candidates();
        let execution = crate::analysis::lower_graph(j_graph.clone(), &|name| match self
            .names
            .get(name)
            .map(|b| &b.value)
        {
            Some(SymbolValue::Noun(value)) => crate::facts::Facts::of(value),
            _ => crate::facts::Facts::default(),
        })
        .map_err(|error| error.in_phase(DiagnosticPhase::SemanticAnalysis))?;
        Ok(crate::analysis::CompilationAnalysis {
            j_graph,
            graph_rewrites,
            execution,
        })
    }

    /// Backward-compatible projection of analyze_compilation() returning only
    /// the execution-oriented logical plan.
    pub fn analyze(&self, source: &str) -> Result<crate::analysis::LogicalPlan> {
        self.analyze_compilation(source).map(|analysis| analysis.execution)
    }

    /// Build the A3-v0 operation/value-separated single-block logical IR.
    /// This is still inspection-only and does not execute kernels.
    pub fn analyze_a3(&self, source: &str) -> Result<crate::logical_ir::Plan> {
        let transition = self.analyze(source)?;
        let plan = crate::logical_ir::Plan::from_transition(&transition);
        plan.verify()
            .map_err(|error| Error::Unsupported(error.to_string()))?;
        Ok(plan)
    }

    /// Compiler-facing analysis path retaining the same structured diagnostic
    /// context used by the interpreter and future JIT.
    pub fn analyze_diagnostic(&self, source: &str) -> Result<crate::analysis::LogicalPlan> {
        self.analyze_compilation_diagnostic(source)
            .map(|analysis| analysis.execution)
    }

    pub fn binding_version(&self, name: &str) -> Option<crate::semantic::NameVersion> {
        self.names.get(name).map(|binding| binding.version)
    }

    fn commit_binding(&mut self, name: String, value: SymbolValue) -> Result<()> {
        let version = crate::semantic::NameVersion(
            self.binding_version(&name)
                .map_or(0, |v| v.0)
                .checked_add(1)
                .ok_or(Error::Limit)?,
        );
        let binding = Binding {
            value: match value {
                SymbolValue::Noun(v) => SymbolValue::Noun(v.into_shared()),
                v => v,
            },
            version,
        };
        if let Some(old) = self.names.insert(name, binding) {
            if let SymbolValue::Noun(value) = old.value {
                self.pool.retire(value);
            }
        }
        Ok(())
    }

    /// Reference execution with stable machine-readable J errors.
    pub fn eval_semantic_reference(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, false)
            .map_err(Error::into_unlocated)
    }

    /// Normal execution with stable machine-readable J errors.
    pub fn eval(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, true)
            .map_err(Error::into_unlocated)
    }

    /// Same interpreter path as `eval`, retaining source provenance for
    /// Python-style human diagnostics. Future JIT/interpreter frontends should
    /// reuse this contract rather than invent a separate error path.
    pub fn eval_diagnostic(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, true)
    }

    pub fn eval_semantic_reference_diagnostic(
        &mut self,
        source: &str,
    ) -> Result<Option<Value>> {
        self.eval_program(source, false)
    }

    fn eval_program(&mut self, source: &str, pooled: bool) -> Result<Option<Value>> {
        let program =
            crate::semantic::parse_runtime(source, &|name| match &self.names.get(name)?.value {
                SymbolValue::Noun(value) => Some(value.clone()),
                SymbolValue::Verb(_) => None,
            })?;
        let Some(expr) = program.expression else {
            return Ok(None);
        };
        // Static binding is an analysis API. Eager lookup here would reorder
        // runtime errors relative to failures in right-hand arguments.
        let value = match expr.kind {
            crate::semantic::ExprKind::VerbValue(verb) => SymbolValue::Verb(verb),
            _ => SymbolValue::Noun(self.interpret_ir(expr, pooled, 0)?),
        };
        if let Some(name) = program.assignment {
            self.commit_binding(name, value)?;
            Ok(None)
        } else {
            match value {
                SymbolValue::Noun(value) => Ok(Some(value)),
                SymbolValue::Verb(_) => Err(Error::Unsupported("verb result display".into())),
            }
        }
    }

    fn resolve_verb(&self, verb: crate::semantic::Verb) -> Result<ResolvedVerb> {
        self.resolve_function_entity(&verb.entity, 0)
    }

    fn resolve_function_entity(
        &self,
        function: &FunctionEntity,
        depth: usize,
    ) -> Result<ResolvedVerb> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }

        match &function.head {
            FunctionHead::PrimitiveVerb(id) => Ok(ResolvedVerb {
                id: *id,
                reduce: false,
                rank: None,
            }),
            FunctionHead::NameRef(name) => {
                let binding = self.names.get(name).ok_or_else(|| {
                    Error::Value(name.clone()).with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_current_name(name.clone()),
                    )
                })?;
                let SymbolValue::Verb(target) = &binding.value else {
                    return Err(
                        Error::Domain.with_context(
                            ErrorContext::phase(DiagnosticPhase::Runtime)
                                .with_current_name(name.clone()),
                        ),
                    );
                };
                self.resolve_function_entity(&target.entity, depth + 1)
            }
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
                let Some(FunctionOperand::Function(operand)) = function.operands.first() else {
                    return Err(Error::Unsupported("malformed insert semantic entity".into()));
                };
                let mut resolved = self.resolve_function_entity(operand, depth + 1)?;
                if resolved.reduce || resolved.rank.is_some() {
                    return Err(Error::Unsupported(
                        "runtime subset cannot flatten insert over a derived modifier".into(),
                    ));
                }
                resolved.reduce = true;
                Ok(resolved)
            }
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
                let [
                    FunctionOperand::Function(operand),
                    FunctionOperand::Noun { value, .. },
                ] = function.operands.as_slice()
                else {
                    return Err(Error::Unsupported("malformed rank semantic entity".into()));
                };
                let mut resolved = self.resolve_function_entity(operand, depth + 1)?;
                if resolved.rank.is_some() {
                    return Err(Error::Unsupported(
                        "runtime subset cannot flatten nested rank modifiers".into(),
                    ));
                }
                if value.is_empty() || value.len() > 3 {
                    return Err(Error::Length);
                }
                let at = |i| value.int_at(i);
                resolved.rank = Some(match value.len() {
                    1 => [at(0)?, at(0)?, at(0)?],
                    2 => [at(1)?, at(0)?, at(1)?],
                    _ => [at(0)?, at(1)?, at(2)?],
                });
                Ok(resolved)
            }
            FunctionHead::PrimitiveAdverb(_)
            | FunctionHead::PrimitiveConjunction(_)
            | FunctionHead::Hook
            | FunctionHead::Fork => Err(
                Error::Unsupported("derived train runtime lowering not implemented".into())
                    .in_phase(DiagnosticPhase::Runtime),
            ),
        }
    }

    fn interpret_ir(
        &mut self,
        expr: crate::semantic::Expr,
        pooled: bool,
        depth: usize,
    ) -> Result<Value> {
        use crate::semantic::ExprKind as Expr;
        let span = expr.span.clone();
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit.at(span));
        }
        let result = (|| -> Result<Value> {
            match expr.kind {
            Expr::Group(inner) => self.interpret_ir(*inner, pooled, depth + 1),
            Expr::Literal(v) => Ok(v),
            Expr::VerbValue(_) => Err(Error::Domain),
            Expr::ReadName(name) => match self.names.get(&name) {
                Some(Binding {
                    value: SymbolValue::Noun(value),
                    ..
                }) => Ok(value.clone()),
                Some(_) => Err(Error::Domain),
                None => Err(Error::Value(name)),
            },
            Expr::Monad { verb, argument } => {
                let verb_span = verb.span.clone();
                let y = self.interpret_ir(*argument, pooled, depth + 1)?;
                let y_summary = argument_summary(ArgumentRole::Y, &y);
                let verb = self.resolve_verb(verb)?;
                let operation = operation_label(&verb);
                let call = if let Some(rank) = verb.rank {
                    kernels::ranked(verb.id.spelling(), verb.reduce, rank[0], y)
                } else if verb.reduce {
                    kernels::reduce(verb.id.spelling(), y)
                } else {
                    kernels::monad(verb.id.spelling(), y)
                };
                call.map_err(|error| {
                    error.with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_span(verb_span)
                            .executing(operation, DiagnosticValence::Monad)
                            .with_argument(y_summary),
                    )
                })
            }
            Expr::Dyad { verb, left, right } => {
                let verb_span = verb.span.clone();
                let y = self.interpret_ir(*right, pooled, depth + 1)?;
                let x = self.interpret_ir(*left, pooled, depth + 1)?;
                let x_summary = argument_summary(ArgumentRole::X, &x);
                let y_summary = argument_summary(ArgumentRole::Y, &y);
                let verb = self.resolve_verb(verb)?;
                let operation = operation_label(&verb);
                let call = if let Some(rank) = verb.rank {
                    kernels::ranked_dyad_ranks(verb.id.spelling(), rank[1], rank[2], x, y)
                } else if pooled && !x.is_sparse() && !y.is_sparse() {
                    match verb.id {
                        crate::primitive::PrimitiveId::Add => {
                            kernels::atomic_with_pool(kernels::Op::Add, x, y, &mut self.pool)
                        }
                        crate::primitive::PrimitiveId::Subtract => {
                            kernels::atomic_with_pool(kernels::Op::Sub, x, y, &mut self.pool)
                        }
                        crate::primitive::PrimitiveId::Multiply => {
                            kernels::atomic_with_pool(kernels::Op::Mul, x, y, &mut self.pool)
                        }
                        _ => kernels::dyad(verb.id.spelling(), x, y),
                    }
                } else {
                    kernels::dyad(verb.id.spelling(), x, y)
                };
                call.map_err(|error| {
                    error.with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_span(verb_span)
                            .executing(operation, DiagnosticValence::Dyad)
                            .with_argument(x_summary)
                            .with_argument(y_summary),
                    )
                })
            }
            }
        })();
        result.map_err(|error| error.at(span))
    }
}

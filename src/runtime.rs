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
    primitives: crate::primitive::PrimitiveContext,
}

/// Execution result plus optional source-operation observations, including failure.
pub struct CapturedEvaluation {
    pub result: Result<Option<Value>>,
    pub capture: crate::parser_capture::ParseCapture,
}

struct EngineParserHost<'a> {
    engine: &'a mut Engine,
    pooled: bool,
}
impl crate::parser::RuntimeParserHost for EngineParserHost<'_> {
    fn lookup(&mut self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        self.engine.parser_name_binding(name)
    }
    fn version(&self, name: &str) -> Option<crate::semantic::NameVersion> {
        self.engine.binding_version(name)
    }
    fn apply(&mut self, expression: crate::semantic::Expr) -> Result<Value> {
        self.engine.interpret_ir(expression, self.pooled, 0)
    }
    fn resolve_modifier(
        &mut self,
        name: &str,
        expected: crate::semantic::FunctionPartOfSpeech,
    ) -> Result<crate::parser::ResolvedModifier> {
        let mut current = name.to_owned();
        let mut bindings = Vec::new();
        for _ in 0..=crate::semantic::MAX_EXPR_DEPTH {
            let binding = self
                .engine
                .names
                .get(&current)
                .ok_or_else(|| Error::Value(current.clone()))?;
            let SymbolValue::Modifier(function) = &binding.value else {
                return Err(Error::Domain);
            };
            if function.result_pos != expected {
                return Err(Error::Domain);
            }
            bindings.push((current.clone(), binding.version));
            if let FunctionHead::NameRef(next) = &function.head {
                current = next.clone();
            } else {
                if !function.is_known_modifier() {
                    return Err(Error::Unsupported(
                        "derived modifier construction executor".into(),
                    ));
                }
                return Ok(crate::parser::ResolvedModifier {
                    function: function.clone(),
                    bindings,
                });
            }
        }
        Err(Error::Limit)
    }
    fn assign(
        &mut self,
        name: &str,
        value: crate::parser::AssignedValue,
    ) -> Result<crate::parser::AssignedValue> {
        use crate::parser::AssignedValue;
        let (binding, returned) = match value {
            AssignedValue::Noun(value) => {
                let value = value.into_shared();
                let returned = value.clone();
                (SymbolValue::Noun(value), AssignedValue::Noun(returned))
            }
            AssignedValue::Verb(verb) => {
                (SymbolValue::Verb(verb.clone()), AssignedValue::Verb(verb))
            }
            AssignedValue::Modifier(function) => (
                SymbolValue::Modifier(function.clone()),
                AssignedValue::Modifier(function),
            ),
        };
        self.engine.commit_binding(name.to_owned(), binding)?;
        Ok(returned)
    }
}

enum SymbolValue {
    Noun(Value),
    Verb(crate::semantic::Verb),
    Modifier(std::sync::Arc<FunctionEntity>),
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
            primitives: crate::primitive::PrimitiveContext::core(),
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
    /// Create an Engine with a compile-profile primitive context.
    ///
    /// Extension names still enter enqueue as ordinary NAMEs; this context is
    /// consulted only during parser-time name lookup after user bindings.
    pub fn with_primitive_context(primitives: crate::primitive::PrimitiveContext) -> Self {
        Self {
            names: HashMap::new(),
            pool: crate::pool::OutputPool::new(64 * 1024 * 1024),
            primitives,
        }
    }

    fn parser_name_binding(&self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        if let Some(binding) = self.names.get(name) {
            return Some(match &binding.value {
                SymbolValue::Noun(value) => crate::parser::ParserNameBinding::Noun(value.clone()),
                SymbolValue::Modifier(function) => {
                    crate::parser::ParserNameBinding::Function(function.result_pos)
                }
                SymbolValue::Verb(_) => crate::parser::ParserNameBinding::Function(
                    crate::semantic::FunctionPartOfSpeech::Verb,
                ),
            });
        }
        self.primitives
            .resolve_extension_binding(name)
            .map(|handle| crate::parser::ParserNameBinding::Function(handle.result_pos.into()))
    }
    fn parser_analysis_binding(&self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        if let Some(Binding {
            value: SymbolValue::Modifier(function),
            version,
        }) = self.names.get(name)
        {
            return Some(crate::parser::ParserNameBinding::KnownModifier {
                function: function.clone(),
                version: *version,
            });
        }
        self.parser_name_binding(name)
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
            crate::parser::parse_analysis(source, &|name| self.parser_analysis_binding(name))?,
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

    pub fn analyze_j_graph_diagnostic(&self, source: &str) -> Result<crate::j_graph_ir::Plan> {
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
    ) -> Result<crate::compilation::CompilationAnalysis> {
        self.analyze_compilation_diagnostic(source)
            .map_err(Error::into_unlocated)
    }

    pub fn analyze_compilation_diagnostic(
        &self,
        source: &str,
    ) -> Result<crate::compilation::CompilationAnalysis> {
        let j_graph = self.analyze_j_graph_diagnostic(source)?;
        let graph_rewrites = j_graph.rewrite_candidates();
        let graph_rewrite_resources =
            crate::j_graph_resource::evaluate_rewrite_candidates(&j_graph, &graph_rewrites)
                .map_err(|message| {
                    Error::Unsupported(message.into()).in_phase(DiagnosticPhase::SemanticAnalysis)
                })?;
        let logical = crate::analysis::lower_graph(j_graph.clone(), &|name| match self
            .names
            .get(name)
            .map(|b| &b.value)
        {
            Some(SymbolValue::Noun(value)) => crate::facts::Facts::of(value),
            _ => crate::facts::Facts::default(),
        })
        .map_err(|error| error.in_phase(DiagnosticPhase::SemanticAnalysis))?;
        Ok(crate::compilation::CompilationAnalysis {
            j_graph,
            graph_rewrites,
            graph_rewrite_resources,
            logical,
        })
    }

    /// Canonical compiler analysis API.
    ///
    /// `analyze_a3` is retained as an explicit A3-named alias.
    pub fn analyze(&self, source: &str) -> Result<crate::logical_ir::Plan> {
        self.analyze_compilation(source)
            .map(|analysis| analysis.logical)
    }

    /// Build the canonical A3-v0 operation/value-separated logical IR.
    /// This is inspection-only and does not execute kernels.
    pub fn analyze_a3(&self, source: &str) -> Result<crate::logical_ir::Plan> {
        self.analyze_compilation(source)
            .map(|analysis| analysis.logical)
    }

    /// Compiler-facing canonical Logical IR path retaining the same structured
    /// diagnostic context used by the interpreter and future JIT.
    pub fn analyze_diagnostic(&self, source: &str) -> Result<crate::logical_ir::Plan> {
        self.analyze_compilation_diagnostic(source)
            .map(|analysis| analysis.logical)
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
        self.eval_program(source, false, None)
            .map_err(Error::into_unlocated)
    }

    /// Normal execution with stable machine-readable J errors.
    pub fn eval(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, true, None)
            .map_err(Error::into_unlocated)
    }

    /// Same interpreter path as `eval`, retaining source provenance for
    /// Python-style human diagnostics. Future JIT/interpreter frontends should
    /// reuse this contract rather than invent a separate error path.
    pub fn eval_diagnostic(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, true, None)
    }

    pub fn eval_semantic_reference_diagnostic(&mut self, source: &str) -> Result<Option<Value>> {
        self.eval_program(source, false, None)
    }

    /// Capture is observational: the same parser/kernel path executes either way.
    /// Input/intermediate facts and edges are retained, not array snapshots.
    pub fn eval_captured(&mut self, source: &str) -> CapturedEvaluation {
        let mut capture = crate::parser_capture::ParseCapture::default();
        let result = self.eval_program(source, true, Some(&mut capture));
        if let Err(error) = &result {
            capture.failure = Some(crate::parser_capture::CaptureFailure {
                kind: error.kind().into(),
                context: error.context().cloned(),
            });
        }
        CapturedEvaluation { result, capture }
    }

    fn eval_program(
        &mut self,
        source: &str,
        pooled: bool,
        capture: Option<&mut crate::parser_capture::ParseCapture>,
    ) -> Result<Option<Value>> {
        let program = crate::parser::parse_runtime_host(
            source,
            &mut EngineParserHost {
                engine: self,
                pooled,
            },
            capture,
        )?;
        let Some(expr) = program.expression else {
            return Ok(None);
        };
        // Static binding is an analysis API. Eager lookup here would reorder
        // runtime errors relative to failures in right-hand arguments.
        let value = match expr.kind {
            crate::semantic::ExprKind::VerbValue(verb) => SymbolValue::Verb(verb),
            crate::semantic::ExprKind::ModifierValue(function) => SymbolValue::Modifier(function),
            crate::semantic::ExprKind::Literal(value) => SymbolValue::Noun(value),
            // Parentheses only wrap completed nouns; no kernel replay occurs.
            _ => SymbolValue::Noun(self.interpret_ir(expr, pooled, 0)?),
        };
        if program.assignment.is_some() {
            // Runtime row 7 already committed the value. Even a later parser
            // exit error must not roll back that J-visible assignment.
            Ok(None)
        } else {
            match value {
                SymbolValue::Noun(value) => Ok(Some(value)),
                SymbolValue::Verb(_) => Err(Error::Unsupported("verb result display".into())),
                SymbolValue::Modifier(_) => {
                    Err(Error::Unsupported("modifier result display".into()))
                }
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
                    return Err(Error::Domain.with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_current_name(name.clone()),
                    ));
                };
                self.resolve_function_entity(&target.entity, depth + 1)
            }
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
                let Some(FunctionOperand::Function(operand)) = function.operands.first() else {
                    return Err(Error::Unsupported(
                        "malformed insert semantic entity".into(),
                    ));
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
                resolved.rank = Some(crate::semantic::rank_noun_contract(value)?);
                Ok(resolved)
            }
            FunctionHead::PrimitiveAdverb(_)
            | FunctionHead::PrimitiveConjunction(_)
            | FunctionHead::ModifierTrain
            | FunctionHead::Hook
            | FunctionHead::Fork => Err(Error::Unsupported(
                "derived train runtime lowering not implemented".into(),
            )
            .in_phase(DiagnosticPhase::Runtime)),
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
                Expr::VerbValue(_) | Expr::ModifierValue(_) => Err(Error::Domain),
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

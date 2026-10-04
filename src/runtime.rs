use crate::{
    error::{
        ArgumentRole, ArgumentSummary, DiagnosticPhase, DiagnosticValence, Error, ErrorContext,
        Result,
    },
    kernels,
    semantic::{FunctionEntity, FunctionHead, FunctionOperand, FunctionPartOfSpeech, JEntity},
    value::Value,
};
use std::collections::HashMap;

pub struct Engine {
    names: HashMap<String, Binding>,
    pool: crate::pool::OutputPool,
    primitives: crate::primitive::PrimitiveContext,
    definition_depth: usize,
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
    fn stacked_modifier(
        &self,
        name: &str,
    ) -> Option<(std::sync::Arc<FunctionEntity>, crate::semantic::NameVersion)> {
        let binding = self.engine.names.get(name)?;
        let JEntity::Function(function) = &binding.value else {
            return None;
        };
        if function.result_pos == FunctionPartOfSpeech::Verb {
            return None;
        }
        function
            .is_nameless_modifier()
            .then(|| (function.clone(), binding.version))
    }
    fn lookup(&mut self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        self.engine.parser_name_binding(name)
    }
    fn gerund_binding(&self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
        Ok(self.engine.parser_name_binding(name))
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
            let JEntity::Function(function) = &binding.value else {
                return Err(Error::Domain);
            };
            if function.result_pos == FunctionPartOfSpeech::Verb || function.result_pos != expected
            {
                return Err(Error::Domain);
            }
            bindings.push((current.clone(), binding.version));
            if let FunctionHead::NameRef(next) = &function.head {
                current = next.clone();
            } else {
                if !function.is_known_modifier()
                    && !matches!(function.head, FunctionHead::ExplicitDefinition(_))
                {
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
    fn apply_definition(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: FunctionOperand,
        right: Option<FunctionOperand>,
    ) -> Result<JEntity> {
        self.engine
            .invoke_modifier(operator, left, right, self.pooled)
    }
    fn assign(&mut self, name: &str, value: JEntity) -> Result<JEntity> {
        self.engine.commit_binding(name.to_owned(), value)
    }
}

/// A read-only operand frame. Ordinary globals still resolve in the calling
/// Engine; mnuv substitute their concrete values without changing that namespace.
struct ModifierFrame<'a> {
    parent: EngineParserHost<'a>,
    left: FunctionOperand,
    right: Option<FunctionOperand>,
}
impl ModifierFrame<'_> {
    fn operand(&self, name: &str) -> Option<&FunctionOperand> {
        match name {
            "u" => Some(&self.left),
            "v" => self.right.as_ref(),
            "m" if matches!(self.left, FunctionOperand::Noun { .. }) => Some(&self.left),
            "n" => self
                .right
                .as_ref()
                .filter(|value| matches!(value, FunctionOperand::Noun { .. })),
            _ => None,
        }
    }
}
impl crate::parser::RuntimeParserHost for ModifierFrame<'_> {
    fn lookup(&mut self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        match self.operand(name) {
            Some(FunctionOperand::Noun { value, .. }) => {
                Some(crate::parser::ParserNameBinding::Noun(value.clone()))
            }
            Some(FunctionOperand::Function(function)) => Some(
                crate::parser::ParserNameBinding::Function(function.result_pos),
            ),
            None => self.parent.lookup(name),
        }
    }
    fn operand_function(&self, name: &str) -> Option<std::sync::Arc<FunctionEntity>> {
        match self.operand(name) {
            Some(FunctionOperand::Function(function)) => Some(function.clone()),
            _ => None,
        }
    }
    fn version(&self, name: &str) -> Option<crate::semantic::NameVersion> {
        if self.operand(name).is_some() {
            None
        } else {
            self.parent.version(name)
        }
    }
    fn gerund_binding(&self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
        match self.operand(name) {
            Some(FunctionOperand::Noun { value, .. }) => {
                Ok(Some(crate::parser::ParserNameBinding::Noun(value.clone())))
            }
            Some(FunctionOperand::Function(function)) => Ok(Some(
                crate::parser::ParserNameBinding::Function(function.result_pos),
            )),
            None => self.parent.gerund_binding(name),
        }
    }
    fn apply(&mut self, expression: crate::semantic::Expr) -> Result<Value> {
        self.parent.apply(expression)
    }
    fn resolve_modifier(
        &mut self,
        name: &str,
        expected: FunctionPartOfSpeech,
    ) -> Result<crate::parser::ResolvedModifier> {
        if let Some(FunctionOperand::Function(function)) = self.operand(name) {
            if function.result_pos != expected {
                return Err(Error::Domain);
            }
            return Ok(crate::parser::ResolvedModifier {
                function: function.clone(),
                bindings: Vec::new(),
            });
        }
        self.parent.resolve_modifier(name, expected)
    }
    fn apply_definition(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: FunctionOperand,
        right: Option<FunctionOperand>,
    ) -> Result<JEntity> {
        self.parent.apply_definition(operator, left, right)
    }
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
    value: JEntity,
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
            definition_depth: 0,
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
            definition_depth: 0,
        }
    }

    fn invoke_modifier(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: FunctionOperand,
        right: Option<FunctionOperand>,
        pooled: bool,
    ) -> Result<JEntity> {
        let FunctionHead::ExplicitDefinition(code) = &operator.head else {
            return Err(Error::Domain);
        };
        if code.operator_definition {
            return Err(Error::Unsupported(
                "explicit operator x/y invocation".into(),
            ));
        }
        let (section, controls) = if right.is_some() {
            (&code.dyad, &code.dyad_controls)
        } else {
            (&code.monad, &code.monad_controls)
        };
        if section.is_empty() {
            return Err(Error::Valence);
        }
        if section.len() != 1
            || controls
                .iter()
                .any(|node| node.kind != crate::definition_flow::ControlKind::Body)
        {
            return Err(Error::Unsupported(
                "explicit modifier control or multiple sentences".into(),
            ));
        }
        let sentence = &code.sentences[section.start];
        if sentence
            .words
            .iter()
            .any(|word| word.flags.local_assignment || word.flags.global_assignment)
        {
            return Err(Error::Unsupported(
                "explicit modifier assignment scope".into(),
            ));
        }
        // Each invocation currently nests the shared parser. Keep a conservative
        // Windows stack bound until the general executor uses explicit frames.
        const MAX_MODIFIER_INVOCATION_DEPTH: usize = 8;
        if self.definition_depth >= MAX_MODIFIER_INVOCATION_DEPTH {
            return Err(Error::Limit);
        }
        self.definition_depth += 1;
        let result = (|| {
            let mut frame = ModifierFrame {
                parent: EngineParserHost {
                    engine: self,
                    pooled,
                },
                left,
                right,
            };
            // Unbound special names must never fall through to a global alias.
            // Full undefined-local diagnostics wait for the general scope executor.
            for word in &sentence.words {
                let name = &code.body[word.span.clone()];
                if word.flags.lookup_name
                    && matches!(name, "u" | "v" | "m" | "n" | "x" | "y")
                    && frame.operand(name).is_none()
                {
                    return Err(Error::Unsupported(
                        "undefined explicit operand alias".into(),
                    ));
                }
            }
            let body = &code.body[sentence.span.clone()];
            if !matches!(
                crate::parser::frame_definition_input(body)?,
                crate::parser::InputFrame::Sentence
            ) {
                return Err(Error::Unsupported(
                    "nested explicit modifier definition scope".into(),
                ));
            }
            let program = crate::parser::parse_runtime_host(body, &mut frame, None)?;
            let expression = program
                .expression
                .ok_or_else(|| Error::Unsupported("empty explicit modifier result".into()))?;
            match expression.kind {
                crate::semantic::ExprKind::Literal(value) => Ok(JEntity::Noun(value)),
                crate::semantic::ExprKind::VerbValue(verb) => Ok(JEntity::Function(verb.entity)),
                crate::semantic::ExprKind::ModifierValue(function) => {
                    Ok(JEntity::Function(function))
                }
                _ => Ok(JEntity::Noun(crate::parser::RuntimeParserHost::apply(
                    &mut frame, expression,
                )?)),
            }
        })();
        self.definition_depth -= 1;
        result
    }

    fn parser_name_binding(&self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        if let Some(binding) = self.names.get(name) {
            return Some(match &binding.value {
                JEntity::Noun(value) => crate::parser::ParserNameBinding::Noun(value.clone()),
                JEntity::Function(function) => {
                    crate::parser::ParserNameBinding::Function(function.result_pos)
                }
            });
        }
        self.primitives
            .resolve_extension_binding(name)
            .map(|handle| crate::parser::ParserNameBinding::Function(handle.result_pos.into()))
    }
    fn parser_analysis_binding(&self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        if let Some(Binding {
            value: JEntity::Function(function),
            version,
        }) = self.names.get(name)
        {
            // Unknown application semantics do not prevent transporting the
            // current POS-bearing function name through a static assignment.
            if function.result_pos == FunctionPartOfSpeech::Verb || !function.is_known_modifier() {
                return self.parser_name_binding(name);
            }
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
                Some(JEntity::Noun(value)) => crate::j_graph_ir::GraphFacts::of(value),
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
            Some(JEntity::Noun(value)) => crate::facts::Facts::of(value),
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

    fn commit_binding(&mut self, name: String, value: JEntity) -> Result<JEntity> {
        let version = crate::semantic::NameVersion(
            self.binding_version(&name)
                .map_or(0, |v| v.0)
                .checked_add(1)
                .ok_or(Error::Limit)?,
        );
        // Freeze once before sharing; an owned noun clone would copy data.
        let returned = match value {
            JEntity::Noun(value) => JEntity::Noun(value.into_shared()),
            function => function,
        };
        let stored = match &returned {
            JEntity::Noun(value) => JEntity::Noun(value.clone()),
            JEntity::Function(function) => JEntity::Function(function.clone()),
        };
        let binding = Binding {
            value: stored,
            version,
        };
        if let Some(old) = self.names.insert(name, binding) {
            if let JEntity::Noun(value) = old.value {
                self.pool.retire(value);
            }
        }
        Ok(returned)
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
            crate::semantic::ExprKind::VerbValue(verb) => JEntity::Function(verb.entity),
            crate::semantic::ExprKind::ModifierValue(function) => JEntity::Function(function),
            crate::semantic::ExprKind::Literal(value) => JEntity::Noun(value),
            // Parentheses only wrap completed nouns; no kernel replay occurs.
            _ => JEntity::Noun(self.interpret_ir(expr, pooled, 0)?),
        };
        if program.assignment.is_some() {
            // Runtime row 7 already committed the value. Even a later parser
            // exit error must not roll back that J-visible assignment.
            Ok(None)
        } else {
            match value {
                JEntity::Noun(value) => Ok(Some(value)),
                JEntity::Function(function) => Err(Error::Unsupported(
                    if function.result_pos == FunctionPartOfSpeech::Verb {
                        "verb result display"
                    } else {
                        "modifier result display"
                    }
                    .into(),
                )),
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
                let JEntity::Function(target) = &binding.value else {
                    return Err(Error::Domain.with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_current_name(name.clone()),
                    ));
                };
                if target.result_pos != FunctionPartOfSpeech::Verb {
                    return Err(Error::Domain.with_context(
                        ErrorContext::phase(DiagnosticPhase::Runtime)
                            .with_current_name(name.clone()),
                    ));
                }
                self.resolve_function_entity(target, depth + 1)
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
            | FunctionHead::DefinitionConstructor(_)
            | FunctionHead::ExplicitDefinition(_)
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
                        value: JEntity::Noun(value),
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

#[cfg(test)]
mod entity_binding_tests {
    use super::*;
    use crate::{Data, parser::ParserNameBinding, semantic::NameVersion};
    use std::sync::Arc;

    #[test]
    fn commit_shares_returned_and_stored_rhs_without_noun_copy() {
        let mut engine = Engine::new();
        let noun = Value::ints([65536], (0..65536).collect()).unwrap();
        let Data::Int(data) = noun.data() else {
            panic!()
        };
        let original = data.as_ptr();
        let returned = engine
            .commit_binding("rhs".into(), JEntity::Noun(noun))
            .unwrap();
        for entity in [&returned, &engine.names["rhs"].value] {
            let JEntity::Noun(value) = entity else {
                panic!()
            };
            let Data::Int(data) = value.data() else {
                panic!()
            };
            assert_eq!(data.as_ptr(), original);
            assert_eq!(value.int_at(65535).unwrap(), 65535);
        }
        drop(returned);
        for (index, function) in [
            FunctionEntity::primitive(crate::primitive::PrimitiveId::Add, 3..4),
            FunctionEntity::primitive_adverb(crate::primitive::AdverbId::Insert, 5..6),
            FunctionEntity::primitive_conjunction(crate::primitive::ConjunctionId::Rank, 7..8),
        ]
        .into_iter()
        .enumerate()
        {
            let returned = engine
                .commit_binding("rhs".into(), JEntity::Function(function.clone()))
                .unwrap();
            let JEntity::Function(returned) = returned else {
                panic!()
            };
            let JEntity::Function(stored) = &engine.names["rhs"].value else {
                panic!()
            };
            assert!(Arc::ptr_eq(&returned, stored));
            assert!(Arc::ptr_eq(&function, stored));
            assert_eq!(stored.span, function.span);
            let Some(ParserNameBinding::Function(pos)) = engine.parser_name_binding("rhs") else {
                panic!()
            };
            assert_eq!(pos, function.result_pos);
            assert_eq!(
                engine.binding_version("rhs"),
                Some(NameVersion(index as u64 + 2))
            );
        }
    }

    #[test]
    fn failed_version_increment_preserves_binding_and_retirement_state() {
        let mut engine = Engine::with_output_cache_limit(4096);
        engine.eval("kept=:i.256").unwrap();
        engine.names.get_mut("kept").unwrap().version = NameVersion(u64::MAX);
        let before = engine.eval("kept").unwrap().unwrap();
        let stats = engine.output_cache_stats();
        for replacement in [
            JEntity::Noun(Value::scalar(9)),
            JEntity::Function(FunctionEntity::primitive(
                crate::primitive::PrimitiveId::Add,
                0..1,
            )),
        ] {
            assert_eq!(
                engine
                    .commit_binding("kept".into(), replacement)
                    .unwrap_err()
                    .kind(),
                "limit error"
            );
            assert_eq!(engine.binding_version("kept"), Some(NameVersion(u64::MAX)));
            assert_eq!(engine.eval("kept").unwrap().unwrap().json(), before.json());
            assert_eq!(engine.output_cache_stats(), stats);
        }
        let report = engine.eval_captured("kept=:2+3");
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), "limit error");
        assert!(
            !report
                .capture
                .events
                .iter()
                .any(|event| matches!(event, crate::parser_capture::CaptureEvent::Commit { .. }))
        );
        assert_eq!(engine.output_cache_stats(), stats);
    }

    #[test]
    fn unified_function_bindings_enforce_lookup_pos_and_error_context() {
        use crate::parser::RuntimeParserHost;
        let mut engine = Engine::new();
        let reference = FunctionEntity::name_ref("target".into(), FunctionPartOfSpeech::Verb, 0..6);
        for source in ["target=:/", "target=:\""] {
            engine.eval(source).unwrap();
            let error = engine.resolve_function_entity(&reference, 0).err().unwrap();
            assert_eq!(error.kind(), "domain error");
            assert_eq!(
                error.context().unwrap().current_name.as_deref(),
                Some("target")
            );
        }
        engine.eval("target=:+").unwrap();
        let Some(ParserNameBinding::Function(pos)) = engine.parser_analysis_binding("target")
        else {
            panic!()
        };
        assert_eq!(pos, FunctionPartOfSpeech::Verb);
        let mut host = EngineParserHost {
            engine: &mut engine,
            pooled: false,
        };
        assert!(host.stacked_modifier("target").is_none());
        assert_eq!(
            host.resolve_modifier("target", FunctionPartOfSpeech::Adverb)
                .err()
                .unwrap()
                .kind(),
            "domain error"
        );
    }
}

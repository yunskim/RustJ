use crate::{
    error::{
        ArgumentRole, ArgumentSummary, DiagnosticPhase, DiagnosticValence, Error, ErrorContext,
        Result,
    },
    kernels,
    semantic::{FunctionEntity, FunctionHead, FunctionOperand, FunctionPartOfSpeech, JEntity},
    value::Value,
};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod scope_provenance_tests {
    use super::*;
    use crate::frontend_context::{
        FoundScope, LocalLookupState, NamePolicy, ScopeInstanceId, ScopeSearch,
    };

    fn parse_frame(engine: &mut Engine, source: &str) -> crate::semantic::Program {
        let mut capture = crate::parser_capture::ParseCapture::default();
        let mut host = ModifierFrame {
            parent: EngineParserHost {
                engine,
                pooled: false,
            },
        };
        let program =
            crate::parser::parse_runtime_host(source, &mut host, Some(&mut capture)).unwrap();
        program.frontend.as_ref().unwrap().verify().unwrap();
        program
    }

    #[test]
    fn declared_unbound_local_falls_back_then_local_write_shadows_without_global_commit() {
        let mut engine = Engine::new();
        engine.eval("shared=:10").unwrap();
        let global_version = engine.binding_version("shared");
        let frame = ScopeInstanceId::fresh();
        engine.local_frames.push(LocalFrame {
            instance: frame,
            names: HashMap::new(),
            declared: ["shared".to_owned()].into_iter().collect(),
        });
        let program = parse_frame(&mut engine, "shared=.shared+1");
        let context = program.frontend.unwrap();
        let read = &context.name_uses[0];
        let lookup = read.lookup.as_ref().unwrap();
        assert_eq!(lookup.search, ScopeSearch::CurrentFrameThenGlobal);
        assert_eq!(lookup.frame, Some(frame));
        assert_eq!(lookup.local_state, LocalLookupState::DeclaredUnbound);
        assert_eq!(lookup.found, FoundScope::Global(engine.namespace_instance));
        assert!(context.words.iter().any(|word| word.flags.local_assignment));
        assert_eq!(engine.binding_version("shared"), global_version);
        let later = parse_frame(&mut engine, "shared").frontend.unwrap();
        let lookup = later.name_uses[0].lookup.as_ref().unwrap();
        assert_eq!(lookup.local_state, LocalLookupState::Bound);
        assert_eq!(lookup.found, FoundScope::Local(frame));
        let mut bad = (*later).clone();
        bad.name_uses[0].lookup.as_mut().unwrap().found =
            FoundScope::Global(engine.namespace_instance);
        assert!(bad.verify().is_err());
        engine.local_frames.pop();
        assert_eq!(
            engine.eval("shared").unwrap().unwrap().int_at(0).unwrap(),
            10
        );
    }

    #[test]
    fn frame_instances_distinguish_equal_local_versions_and_implicit_function_substitution() {
        let mut engine = Engine::new();
        let mut observations = Vec::new();
        for _ in 0..2 {
            engine.local_frames.push(LocalFrame {
                instance: ScopeInstanceId::fresh(),
                names: HashMap::new(),
                declared: ["f".to_owned(), "u".to_owned()].into_iter().collect(),
            });
            parse_frame(&mut engine, "f=.+");
            let ordinary = parse_frame(&mut engine, "f").frontend.unwrap();
            assert_eq!(ordinary.name_uses[0].policy, NamePolicy::LateAtCall);
            observations.push(ordinary.name_uses[0].lookup.clone().unwrap());
            parse_frame(&mut engine, "u=.+");
            let implicit = parse_frame(&mut engine, "u").frontend.unwrap();
            assert_eq!(
                implicit.name_uses[0].policy,
                NamePolicy::CaptureAtRead,
                "{:#?}",
                implicit
            );
            assert!(matches!(
                implicit.name_uses[0].lookup.as_ref().unwrap().found,
                FoundScope::Local(_)
            ));
            // u substitutes the supplied entity even when that entity is itself
            // an ordinary late NameRef. The u lookup must not become late.
            parse_frame(&mut engine, "u=.f");
            let implicit_alias = parse_frame(&mut engine, "u").frontend.unwrap();
            assert_eq!(
                implicit_alias.name_uses[0].policy,
                NamePolicy::CaptureAtRead
            );
            assert_eq!(
                implicit_alias.name_uses[0].resolution,
                crate::frontend_context::NameResolution::FunctionValue
            );
            engine.local_frames.pop();
        }
        assert_ne!(observations[0].frame, observations[1].frame);
        assert_eq!(
            observations[0].binding_version,
            observations[1].binding_version
        );
    }
}

pub struct Engine {
    namespace_instance: crate::frontend_context::ScopeInstanceId,
    names: HashMap<String, Binding>,
    pool: crate::pool::OutputPool,
    /// A bounded physical exact-search table, keyed by immutable Arc identity.
    /// Never inferred from a J name string or parser binding version.
    exact_search_cache: crate::index_ops::ExactPrehashCache,
    primitives: crate::primitive::PrimitiveContext,
    definition_depth: usize,
    local_frames: Vec<LocalFrame>,
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
    fn lookup_observation(&self, name: &str) -> Option<crate::frontend_context::LookupObservation> {
        Some(self.engine.lookup_observation(name))
    }
    fn function_name_ranks(&self, name: &str) -> Option<[i64; 3]> {
        match self.engine.visible_binding(name) {
            Some(Binding {
                value: JEntity::Function(function),
                ..
            }) => function.innate_ranks(),
            Some(_) => None,
            None => self
                .engine
                .primitives
                .resolve_extension_binding(name)
                .is_none()
                .then_some([63; 3]),
        }
    }

    fn fork_cap_binding(&self, name: &str) -> Result<Option<(bool, crate::semantic::NameVersion)>> {
        Ok(self.engine.visible_binding(name).map(|binding| (
            matches!(&binding.value, JEntity::Function(function)
                if matches!(function.head, FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Cap))),
            binding.version)))
    }

    fn stacked_modifier(
        &self,
        name: &str,
    ) -> Option<(std::sync::Arc<FunctionEntity>, crate::semantic::NameVersion)> {
        let binding = self.engine.visible_binding(name)?;
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
        self.engine
            .visible_binding(name)
            .map(|binding| binding.version)
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
                .visible_binding(&current)
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

/// Current invocation owns local values separately from the global namespace.
struct LocalFrame {
    instance: crate::frontend_context::ScopeInstanceId,
    names: HashMap<String, Binding>,
    declared: HashSet<String>,
}

struct ModifierFrame<'a> {
    parent: EngineParserHost<'a>,
}
impl crate::parser::RuntimeParserHost for ModifierFrame<'_> {
    fn lookup_observation(&self, name: &str) -> Option<crate::frontend_context::LookupObservation> {
        self.parent.lookup_observation(name)
    }
    fn function_name_ranks(&self, name: &str) -> Option<[i64; 3]> {
        self.parent.function_name_ranks(name)
    }

    fn fork_cap_binding(&self, name: &str) -> Result<Option<(bool, crate::semantic::NameVersion)>> {
        self.parent.fork_cap_binding(name)
    }

    fn enqueue_environment(&self) -> crate::enqueuer::EnqueueEnvironment {
        crate::enqueuer::EnqueueEnvironment::ExplicitDefinition
    }
    fn lookup(&mut self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        self.parent.lookup(name)
    }
    fn stacked_modifier(
        &self,
        name: &str,
    ) -> Option<(std::sync::Arc<FunctionEntity>, crate::semantic::NameVersion)> {
        self.parent.stacked_modifier(name)
    }
    fn operand_function(&self, name: &str) -> Option<std::sync::Arc<FunctionEntity>> {
        if !matches!(name, "u" | "v" | "m" | "n") {
            return None;
        }
        match &self
            .parent
            .engine
            .local_frames
            .last()?
            .names
            .get(name)?
            .value
        {
            JEntity::Function(function) => Some(function.clone()),
            _ => None,
        }
    }
    fn version(&self, name: &str) -> Option<crate::semantic::NameVersion> {
        self.parent.version(name)
    }
    fn gerund_binding(&self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
        self.parent.gerund_binding(name)
    }
    fn apply(&mut self, expression: crate::semantic::Expr) -> Result<Value> {
        self.parent.apply(expression)
    }
    fn resolve_modifier(
        &mut self,
        name: &str,
        expected: FunctionPartOfSpeech,
    ) -> Result<crate::parser::ResolvedModifier> {
        self.parent.resolve_modifier(name, expected)
    }
    fn assign_scoped(&mut self, name: &str, value: JEntity, local: bool) -> Result<JEntity> {
        let engine = &mut self.parent.engine;
        if local {
            let frame = engine.local_frames.last_mut().expect("modifier frame");
            frame.declared.insert(name.to_owned());
            store_binding(&mut frame.names, &mut engine.pool, name.to_owned(), value)
        } else {
            if engine
                .local_frames
                .last()
                .is_some_and(|frame| frame.names.contains_key(name))
            {
                return Err(Error::Domain);
            }
            // Ordinary NameRef is not a reference to this frame's storage.
            // Preserve it for lookup in the eventual execution environment.
            engine.commit_binding(name.to_owned(), value)
        }
    }
    fn apply_definition(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: FunctionOperand,
        right: Option<FunctionOperand>,
    ) -> Result<JEntity> {
        // Ordinary names in operands remain late references. The callee reads
        // its own frame then globals, never the caller's private bindings.
        self.parent.apply_definition(operator, left, right)
    }
}

fn store_binding(
    names: &mut HashMap<String, Binding>,
    pool: &mut crate::pool::OutputPool,
    name: String,
    value: JEntity,
) -> Result<JEntity> {
    let version = crate::semantic::NameVersion(
        names
            .get(&name)
            .map_or(0, |binding| binding.version.0)
            .checked_add(1)
            .ok_or(Error::Limit)?,
    );
    let returned = match value {
        JEntity::Noun(value) => JEntity::Noun(value.into_shared()),
        function => function,
    };
    let stored = match &returned {
        JEntity::Noun(value) => JEntity::Noun(value.clone()),
        JEntity::Function(function) => JEntity::Function(function.clone()),
    };
    if let Some(Binding {
        value: JEntity::Noun(value),
        ..
    }) = names.insert(
        name,
        Binding {
            value: stored,
            version,
        },
    ) {
        pool.retire(value);
    }
    Ok(returned)
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
            namespace_instance: crate::frontend_context::ScopeInstanceId::fresh(),
            names: HashMap::new(),
            pool: crate::pool::OutputPool::new(bytes),
            exact_search_cache: crate::index_ops::ExactPrehashCache::default(),
            primitives: crate::primitive::PrimitiveContext::core(),
            definition_depth: 0,
            local_frames: Vec::new(),
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

    /// Exact scalar search prehash statistics (builds, reuse hits).
    /// A cache hit requires matching immutable storage identity and first/last mode.
    pub fn index_prehash_stats(&self) -> (usize, usize) {
        self.exact_search_cache.stats()
    }

    /// Drop the retained search table; does not affect any J value or binding.
    pub fn clear_index_prehash(&mut self) {
        self.exact_search_cache.clear();
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
            namespace_instance: crate::frontend_context::ScopeInstanceId::fresh(),
            names: HashMap::new(),
            pool: crate::pool::OutputPool::new(64 * 1024 * 1024),
            exact_search_cache: crate::index_ops::ExactPrehashCache::default(),
            primitives,
            definition_depth: 0,
            local_frames: Vec::new(),
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
            let operands = std::iter::once(left)
                .chain(right)
                .map(|operand| match operand {
                    FunctionOperand::Noun { value, span } => FunctionOperand::Noun {
                        value: value.into_shared(),
                        span,
                    },
                    function => function,
                })
                .collect();
            return Ok(JEntity::Function(FunctionEntity::derived(
                operator.head.clone(),
                FunctionPartOfSpeech::Verb,
                operator.span.clone(),
                operands,
            )));
        }
        self.invoke_definition_body(operator, left, right, None, pooled)
    }

    fn invoke_definition_body(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: FunctionOperand,
        right: Option<FunctionOperand>,
        arguments: Option<(Option<Value>, Value)>,
        pooled: bool,
    ) -> Result<JEntity> {
        let FunctionHead::ExplicitDefinition(code) = &operator.head else {
            return Err(Error::Domain);
        };
        let verb_call = arguments.is_some();
        let dyadic = arguments
            .as_ref()
            .map_or(right.is_some(), |(x, _)| x.is_some());
        let (section, controls) = if dyadic {
            (&code.dyad, &code.dyad_controls)
        } else {
            (&code.monad, &code.monad_controls)
        };
        if section.is_empty() {
            return Err(Error::Valence);
        }
        if controls
            .iter()
            .any(|node| node.kind != crate::definition_flow::ControlKind::Body)
        {
            return Err(Error::Unsupported("explicit modifier control flow".into()));
        }
        // Reject unsupported framing before any statement has side effects.
        for sentence in &code.sentences[section.clone()] {
            if !matches!(
                crate::parser::frame_definition_input(&code.body[sentence.span.clone()])?,
                crate::parser::InputFrame::Sentence
            ) {
                return Err(Error::Unsupported(
                    "nested explicit modifier definition scope".into(),
                ));
            }
        }
        // Each invocation currently nests the shared parser. Keep a conservative
        // Windows stack bound until the general executor uses explicit frames.
        const MAX_MODIFIER_INVOCATION_DEPTH: usize = 8;
        if self.definition_depth >= MAX_MODIFIER_INVOCATION_DEPTH {
            return Err(Error::Limit);
        }
        let mut local = LocalFrame {
            instance: crate::frontend_context::ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: ["u", "m", "x", "y"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };
        if right.is_some() {
            local.declared.extend(["v".to_owned(), "n".to_owned()]);
        }
        let name_plan = if dyadic {
            &code.name_plan.dyad
        } else {
            &code.name_plan.monad
        };
        local
            .declared
            .extend(name_plan.local_declarations.iter().cloned());
        for (name, alias, operand) in [("u", "m", Some(left)), ("v", "n", right)] {
            if let Some(operand) = operand {
                let value = match operand {
                    FunctionOperand::Noun { value, .. } => {
                        store_binding(
                            &mut local.names,
                            &mut self.pool,
                            alias.to_owned(),
                            JEntity::Noun(value.clone()),
                        )?;
                        JEntity::Noun(value)
                    }
                    FunctionOperand::Function(function) => JEntity::Function(function),
                };
                store_binding(&mut local.names, &mut self.pool, name.to_owned(), value)?;
            }
        }
        if let Some((x, y)) = arguments {
            store_binding(
                &mut local.names,
                &mut self.pool,
                "y".into(),
                JEntity::Noun(y),
            )?;
            if let Some(x) = x {
                store_binding(
                    &mut local.names,
                    &mut self.pool,
                    "x".into(),
                    JEntity::Noun(x),
                )?;
            }
        }
        self.definition_depth += 1;
        self.local_frames.push(local);
        let result = (|| {
            let mut frame = ModifierFrame {
                parent: EngineParserHost {
                    engine: self,
                    pooled,
                },
            };
            let mut last = None;
            for (position, sentence) in code.sentences[section.clone()].iter().enumerate() {
                for word in &sentence.words {
                    let name = &code.body[word.span.clone()];
                    if word.flags.lookup_name
                        && matches!(name, "u" | "v" | "m" | "n" | "x" | "y")
                        && !frame
                            .parent
                            .engine
                            .local_frames
                            .last()
                            .expect("modifier frame")
                            .names
                            .contains_key(name)
                    {
                        return Err(Error::Unsupported(
                            "undefined explicit operand alias".into(),
                        ));
                    }
                }
                let program = crate::parser::parse_runtime_host(
                    &code.body[sentence.span.clone()],
                    &mut frame,
                    None,
                )?;
                if let Some(expression) = program.expression {
                    let assigned = program.assignment.is_some();
                    let value = match expression.kind {
                        crate::semantic::ExprKind::Literal(value) => JEntity::Noun(value),
                        crate::semantic::ExprKind::VerbValue(verb) => {
                            JEntity::Function(verb.entity)
                        }
                        crate::semantic::ExprKind::ModifierValue(function) => {
                            JEntity::Function(function)
                        }
                        _ => JEntity::Noun(crate::parser::RuntimeParserHost::apply(
                            &mut frame, expression,
                        )?),
                    };
                    if !assigned
                        && matches!(value, JEntity::Function(_))
                        && code.sentences[section.start + position + 1..section.end]
                            .iter()
                            .any(|next| !next.words.is_empty())
                    {
                        return Err(Error::NounResult);
                    }
                    last = Some(value);
                }
            }
            let value =
                last.ok_or_else(|| Error::Unsupported("empty explicit modifier result".into()))?;
            if verb_call && matches!(value, JEntity::Function(_)) {
                return Err(Error::NounResult);
            }
            // cx.c fixes only the first implicit locative on each branch.
            // Replacement operands and ordinary names remain untouched.
            match value {
                JEntity::Function(function) => Ok(JEntity::Function(
                    frame.parent.engine.fix_implicit_return(&function, 0)?,
                )),
                noun => Ok(noun),
            }
        })();
        let departing = self.local_frames.pop().expect("modifier frame");
        for binding in departing.names.into_values() {
            if let JEntity::Noun(value) = binding.value {
                self.pool.retire(value);
            }
        }
        self.definition_depth -= 1;
        result
    }

    fn fix_implicit_return(
        &self,
        function: &std::sync::Arc<FunctionEntity>,
        depth: usize,
    ) -> Result<std::sync::Arc<FunctionEntity>> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }
        use crate::primitive::PrimitiveId;
        let operand = match function.head {
            FunctionHead::PrimitiveVerb(PrimitiveId::OperandU) => Some("u"),
            FunctionHead::PrimitiveVerb(PrimitiveId::OperandV) => Some("v"),
            _ => None,
        };
        if let Some(name) = operand {
            let binding = self
                .local_frames
                .last()
                .and_then(|frame| frame.names.get(name))
                .ok_or_else(|| {
                    Error::Unsupported("returning an unbound implicit locative".into())
                })?;
            let JEntity::Function(target) = &binding.value else {
                return Err(Error::Domain);
            };
            if target.result_pos != FunctionPartOfSpeech::Verb {
                return Err(Error::Domain);
            }
            // Do not recursively fix inside the replacement: that belongs to
            // the caller's operand scope, not this departing frame.
            return Ok(target.clone());
        }
        if function.operands.is_empty() && function.decoded_gerund.is_none() {
            return Ok(function.clone());
        }
        let mut changed = false;
        let mut fix = |child: &std::sync::Arc<FunctionEntity>| -> Result<_> {
            let fixed = self.fix_implicit_return(child, depth + 1)?;
            changed |= !std::sync::Arc::ptr_eq(child, &fixed);
            Ok(fixed)
        };
        let fixed = function
            .operands
            .iter()
            .map(|operand| match operand {
                FunctionOperand::Function(child) => fix(child).map(Some),
                FunctionOperand::Noun { .. } => Ok(None),
            })
            .collect::<Result<Vec<_>>>()?;
        // Decoded gerunds are constructor auxiliaries, not source edges.
        // Fixing them requires operator-specific AR reconstruction (af.c);
        // ordinary noun operands must remain noun snapshots.
        let decoded = function.decoded_gerund.clone();
        if !changed {
            return Ok(function.clone());
        }
        let operands = function
            .operands
            .iter()
            .zip(fixed)
            .map(|(operand, fixed)| match operand {
                FunctionOperand::Function(_) => {
                    FunctionOperand::Function(fixed.expect("fixed child"))
                }
                FunctionOperand::Noun { value, span } => FunctionOperand::Noun {
                    value: value.clone(),
                    span: span.clone(),
                },
            })
            .collect();
        Ok(FunctionEntity::with_decoded_gerund(
            FunctionEntity::derived(
                function.head.clone(),
                function.result_pos,
                function.span.clone(),
                operands,
            ),
            decoded,
        ))
    }

    /// Resolve only ordinary aliases here; body execution must wait for arguments.
    fn explicit_operator(
        &self,
        function: &std::sync::Arc<FunctionEntity>,
    ) -> Result<Option<std::sync::Arc<FunctionEntity>>> {
        if !matches!(
            function.head,
            FunctionHead::NameRef(_) | FunctionHead::ExplicitDefinition(_)
        ) {
            return Ok(None);
        }
        let mut current = function.clone();
        for _ in 0..=crate::semantic::MAX_EXPR_DEPTH {
            if let FunctionHead::NameRef(name) = &current.head {
                // Ordinary lookup failures keep the established resolver's
                // name/error context; this probe only recognizes operator calls.
                let Some(binding) = self.visible_binding(name) else {
                    return Ok(None);
                };
                let JEntity::Function(target) = &binding.value else {
                    return Ok(None);
                };
                if target.result_pos != FunctionPartOfSpeech::Verb {
                    return Ok(None);
                }
                current = target.clone();
                continue;
            }
            return Ok(
                matches!(&current.head, FunctionHead::ExplicitDefinition(code)
                if code.operator_definition && !current.operands.is_empty())
                .then_some(current),
            );
        }
        Err(Error::Limit)
    }

    /// Recognize a direct implicit call without changing ordinary lookup errors.
    fn implicit_operand(
        &self,
        function: &std::sync::Arc<FunctionEntity>,
    ) -> Result<Option<std::sync::Arc<FunctionEntity>>> {
        use crate::primitive::PrimitiveId;
        if !matches!(
            function.head,
            FunctionHead::NameRef(_)
                | FunctionHead::PrimitiveVerb(PrimitiveId::OperandU | PrimitiveId::OperandV)
        ) {
            return Ok(None);
        }
        let mut current = function.clone();
        for _ in 0..=crate::semantic::MAX_EXPR_DEPTH {
            match &current.head {
                FunctionHead::NameRef(name) => {
                    let Some(Binding {
                        value: JEntity::Function(target),
                        ..
                    }) = self.visible_binding(name)
                    else {
                        return Ok(None);
                    };
                    if target.result_pos != FunctionPartOfSpeech::Verb {
                        return Ok(None);
                    }
                    current = target.clone();
                }
                FunctionHead::PrimitiveVerb(PrimitiveId::OperandU | PrimitiveId::OperandV) => {
                    let name = if matches!(
                        current.head,
                        FunctionHead::PrimitiveVerb(PrimitiveId::OperandU)
                    ) {
                        "u"
                    } else {
                        "v"
                    };
                    let binding = self
                        .local_frames
                        .last()
                        .and_then(|frame| frame.names.get(name))
                        .ok_or_else(|| {
                            Error::Value(name.into()).with_context(
                                ErrorContext::phase(DiagnosticPhase::Runtime)
                                    .with_current_name(if name == "u" { "u." } else { "v." }),
                            )
                        })?;
                    let JEntity::Function(target) = &binding.value else {
                        return Err(Error::Domain);
                    };
                    if target.result_pos != FunctionPartOfSpeech::Verb {
                        return Err(Error::Domain);
                    }
                    return Ok(Some(target.clone()));
                }
                _ => return Ok(None),
            }
        }
        Err(Error::Limit)
    }

    fn call_entity(
        &mut self,
        function: std::sync::Arc<FunctionEntity>,
        x: Option<Value>,
        y: Value,
        pooled: bool,
        depth: usize,
    ) -> Result<Value> {
        use crate::semantic::{Expr, ExprKind, Verb, VerbTarget};
        let span = function.span.clone();
        let verb = Verb {
            span: span.clone(),
            target: VerbTarget::Derived,
            entity: function,
        };
        let right = Box::new(Expr {
            origin: None,
            span: span.clone(),
            kind: ExprKind::Literal(y),
        });
        let kind = if let Some(x) = x {
            ExprKind::Dyad {
                verb,
                left: Box::new(Expr {
                    origin: None,
                    span: span.clone(),
                    kind: ExprKind::Literal(x),
                }),
                right,
            }
        } else {
            ExprKind::Monad {
                verb,
                argument: right,
            }
        };
        self.interpret_ir(
            Expr {
                origin: None,
                span,
                kind,
            },
            pooled,
            depth + 1,
        )
    }

    fn call_implicit_operand(
        &mut self,
        function: std::sync::Arc<FunctionEntity>,
        x: Option<Value>,
        y: Value,
        pooled: bool,
        depth: usize,
    ) -> Result<Value> {
        let suspended = self.local_frames.pop().expect("implicit operand frame");
        let result = self.call_entity(function, x, y, pooled, depth);
        self.local_frames.push(suspended);
        result
    }

    /// Resolve only a primitive identity/prototype witness; never execute a
    /// user definition to guess facts or suppress its observable effects.
    fn primitive_witness(
        &mut self,
        function: &std::sync::Arc<FunctionEntity>,
        depth: usize,
    ) -> Result<Option<crate::primitive::PrimitiveId>> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }
        if let Some(target) = self.implicit_operand(function)? {
            let suspended = self.local_frames.pop().expect("implicit witness frame");
            let result = self.primitive_witness(&target, depth + 1);
            self.local_frames.push(suspended);
            return result;
        }
        match &function.head {
            FunctionHead::PrimitiveVerb(id) => Ok(Some(*id)),
            FunctionHead::NameRef(name) => {
                let binding = self
                    .visible_binding(name)
                    .ok_or_else(|| Error::Value(name.clone()))?;
                let JEntity::Function(target) = &binding.value else {
                    return Err(Error::Domain);
                };
                if target.result_pos != FunctionPartOfSpeech::Verb {
                    return Err(Error::Domain);
                }
                let target = target.clone();
                self.primitive_witness(&target, depth + 1)
            }
            _ => Ok(None),
        }
    }

    /// Admit a zero-frame fill-cell only for a resolved primitive or a
    /// structurally nested Rank of such a primitive. Never execute user
    /// definitions or guess through unresolved derived operations.
    fn rank_fill_is_value_only(
        &mut self,
        function: &std::sync::Arc<FunctionEntity>,
        depth: usize,
    ) -> Result<bool> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }
        if self.primitive_witness(function, depth)?.is_some() {
            return Ok(true);
        }
        if !matches!(
            &function.head,
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
        ) || function.requested_ranks().is_none()
        {
            return Ok(false);
        }
        let [FunctionOperand::Function(child), _] = function.operands.as_slice() else {
            return Ok(false);
        };
        self.rank_fill_is_value_only(child, depth + 1)
    }

    fn call_composite(
        &mut self,
        function: std::sync::Arc<FunctionEntity>,
        x: Option<Value>,
        y: Value,
        pooled: bool,
        depth: usize,
    ) -> Result<Value> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }
        match &function.head {
            FunctionHead::NameRef(name) => {
                let Some(Binding {
                    value: JEntity::Function(target),
                    ..
                }) = self.visible_binding(name)
                else {
                    return Err(Error::Unsupported("composite name resolution".into()));
                };
                self.call_composite(target.clone(), x, y, pooled, depth + 1)
            }
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
                if x.is_some() {
                    return Err(Error::Unsupported("dyadic runtime insert".into()));
                }
                let [FunctionOperand::Function(operand)] = function.operands.as_slice() else {
                    return Err(Error::Unsupported("runtime gerund insert".into()));
                };
                if !y.is_sparse() && y.shape().first() == Some(&0) {
                    use crate::primitive::PrimitiveId;
                    if let Some(
                        id @ (PrimitiveId::Add
                        | PrimitiveId::Subtract
                        | PrimitiveId::Multiply
                        | PrimitiveId::Divide),
                    ) = self.primitive_witness(operand, depth)?
                    {
                        return kernels::reduce(id.spelling(), y);
                    }
                }
                crate::logical_executor::apply_reduction(y, |x, y| {
                    self.call_entity(operand.clone(), x, y, pooled, depth)
                })
            }
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
                let [FunctionOperand::Function(operand), _] = function.operands.as_slice() else {
                    return Err(Error::Unsupported("runtime noun-left rank".into()));
                };
                let ranks = function.requested_ranks().ok_or_else(|| {
                    Error::Unsupported("rank construction has no innate-rank witness".into())
                })?;
                // Pure ravel's empty-frame result follows only from logical
                // cell shape. Unknown/explicit verbs retain the prototype boundary.
                if x.is_none() && !y.is_sparse() {
                    let rank = crate::logical_executor::cell_rank(y.shape().len(), ranks[0]);
                    let frame_rank = y.shape().len() - rank;
                    if y.shape()[..frame_rank].contains(&0)
                        && self.primitive_witness(operand, depth)?
                            == Some(crate::primitive::PrimitiveId::Ravel)
                    {
                        return kernels::ranked(",", false, ranks[0], y);
                    }
                }
                // A concrete primitive identity permits the current
                // value-only fill-cell evaluation. Unknown/user definitions
                // retain the explicit effect/prototype boundary.
                let primitive_fill = self.rank_fill_is_value_only(operand, depth)?;
                let primitive = self.primitive_witness(operand, depth)?;
                let atomic_add = primitive == Some(crate::primitive::PrimitiveId::Add);
                let primitive_catenate = primitive == Some(crate::primitive::PrimitiveId::Ravel);
                crate::logical_executor::apply_ranked(
                    ranks,
                    x,
                    y,
                    primitive_fill,
                    atomic_add,
                    primitive_catenate,
                    |x, y| self.call_entity(operand.clone(), x, y, pooled, depth),
                )
            }
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop) => {
                let [
                    FunctionOperand::Function(outer),
                    FunctionOperand::Function(inner),
                ] = function.operands.as_slice()
                else {
                    return Err(Error::Domain);
                };
                let result = self.call_entity(inner.clone(), x, y, pooled, depth)?;
                self.call_entity(outer.clone(), None, result, pooled, depth)
            }
            FunctionHead::Hook => {
                let [FunctionOperand::Function(f), FunctionOperand::Function(g)] =
                    function.operands.as_slice()
                else {
                    return Err(Error::Domain);
                };
                let y = y.into_shared();
                let gy = self.call_entity(g.clone(), None, y.clone(), pooled, depth)?;
                self.call_entity(f.clone(), Some(x.unwrap_or(y)), gy, pooled, depth)
            }
            FunctionHead::Fork
                if function.fork_semantics == Some(crate::semantic::ForkSemantics::Capped) =>
            {
                let [
                    _,
                    FunctionOperand::Function(g),
                    FunctionOperand::Function(h),
                ] = function.operands.as_slice()
                else {
                    return Err(Error::Domain);
                };
                let hy = self.call_entity(h.clone(), x, y, pooled, depth + 1)?;
                self.call_entity(g.clone(), None, hy, pooled, depth + 1)
            }
            FunctionHead::Fork => {
                let [
                    first,
                    FunctionOperand::Function(g),
                    FunctionOperand::Function(h),
                ] = function.operands.as_slice()
                else {
                    return Err(Error::Domain);
                };
                match first {
                    FunctionOperand::Noun { value, .. } => {
                        // j.h NVV: evaluate only h; f is the constructor's
                        // frozen noun snapshot, not a later name lookup.
                        let hy = self.call_entity(h.clone(), x, y, pooled, depth)?;
                        self.call_entity(g.clone(), Some(value.clone()), hy, pooled, depth)
                    }
                    FunctionOperand::Function(f) => {
                        let y = y.into_shared();
                        let x = x.map(Value::into_shared);
                        let hy =
                            self.call_entity(h.clone(), x.clone(), y.clone(), pooled, depth)?;
                        let fy = self.call_entity(f.clone(), x, y, pooled, depth)?;
                        self.call_entity(g.clone(), Some(fy), hy, pooled, depth)
                    }
                }
            }
            _ => Err(Error::Unsupported("runtime semantic composition".into())),
        }
    }

    fn call_explicit_operator(
        &mut self,
        function: std::sync::Arc<FunctionEntity>,
        x: Option<Value>,
        y: Value,
        pooled: bool,
    ) -> Result<Value> {
        let copy_operand = |operand: &FunctionOperand| match operand {
            FunctionOperand::Function(function) => FunctionOperand::Function(function.clone()),
            // Deferred construction freezes noun storage before later calls.
            FunctionOperand::Noun { value, span } => FunctionOperand::Noun {
                value: value.clone(),
                span: span.clone(),
            },
        };
        let [left, rest @ ..] = function.operands.as_slice() else {
            return Err(Error::Domain);
        };
        if rest.len() > 1 {
            return Err(Error::Domain);
        }
        let left = copy_operand(left);
        let right = rest.first().map(copy_operand);
        let result = self.invoke_definition_body(function, left, right, Some((x, y)), pooled);
        match result {
            Ok(JEntity::Noun(value)) => Ok(value),
            Ok(JEntity::Function(_)) => Err(Error::NounResult),
            // Body coordinates do not belong to the caller source. A separate
            // diagnostic source frame is required before preserving body spans.
            Err(Error::Context { error, mut context }) => {
                context.span = None;
                context.blame_word_index = None;
                Err(error.with_context(*context))
            }
            Err(error) => Err(error),
        }
    }

    fn parser_name_binding(&self, name: &str) -> Option<crate::parser::ParserNameBinding> {
        if let Some(binding) = self.visible_binding(name) {
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
            if function.result_pos == FunctionPartOfSpeech::Verb {
                return Some(crate::parser::ParserNameBinding::KnownVerb {
                    function: function.clone(),
                    version: *version,
                });
            }
            if !function.is_known_modifier() {
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
        let jsource_opportunities = j_graph.jsource_opportunities();
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
            jsource_opportunities,
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
        store_binding(&mut self.names, &mut self.pool, name, value)
    }

    fn visible_binding(&self, name: &str) -> Option<&Binding> {
        self.local_frames
            .last()
            .and_then(|frame| frame.names.get(name))
            .or_else(|| self.names.get(name))
    }

    fn lookup_observation(&self, name: &str) -> crate::frontend_context::LookupObservation {
        use crate::frontend_context::{
            FoundScope, LocalLookupState, LookupObservation, ScopeSearch,
        };
        let frame = self.local_frames.last();
        let local_state = match frame {
            None => LocalLookupState::NoFrame,
            Some(frame) if frame.names.contains_key(name) => LocalLookupState::Bound,
            Some(frame) if frame.declared.contains(name) => LocalLookupState::DeclaredUnbound,
            Some(_) => LocalLookupState::Absent,
        };
        let found = match frame {
            Some(frame) if frame.names.contains_key(name) => FoundScope::Local(frame.instance),
            _ if self.names.contains_key(name) => FoundScope::Global(self.namespace_instance),
            _ if self.primitives.resolve_extension_binding(name).is_some() => FoundScope::Extension,
            _ => FoundScope::Missing,
        };
        LookupObservation {
            engine: self.namespace_instance,
            frame: frame.map(|frame| frame.instance),
            search: if frame.is_some() {
                ScopeSearch::CurrentFrameThenGlobal
            } else {
                ScopeSearch::GlobalOnly
            },
            local_state,
            found,
            binding_version: self.visible_binding(name).map(|binding| binding.version),
        }
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

    fn resolve_function_entity(
        &self,
        function: &FunctionEntity,
        depth: usize,
    ) -> Result<ResolvedVerb> {
        if depth > crate::semantic::MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }

        match &function.head {
            FunctionHead::PrimitiveVerb(
                crate::primitive::PrimitiveId::OperandU | crate::primitive::PrimitiveId::OperandV,
            ) => Err(Error::Unsupported(
                "implicit-locative call requires caller-scope execution".into(),
            )),
            FunctionHead::PrimitiveVerb(id) => Ok(ResolvedVerb {
                id: *id,
                reduce: false,
                rank: None,
            }),
            FunctionHead::NameRef(name) => {
                let binding = self.visible_binding(name).ok_or_else(|| {
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
                let [FunctionOperand::Function(operand), _] = function.operands.as_slice() else {
                    return Err(Error::Unsupported("malformed rank semantic entity".into()));
                };
                let mut resolved = self.resolve_function_entity(operand, depth + 1)?;
                if resolved.rank.is_some() {
                    return Err(Error::Unsupported(
                        "runtime subset cannot flatten nested rank modifiers".into(),
                    ));
                }
                resolved.rank = Some(function.requested_ranks().ok_or_else(|| {
                    Error::Unsupported("rank construction has no innate-rank witness".into())
                })?);
                Ok(resolved)
            }
            FunctionHead::VocabularyPrimitive(_)
            | FunctionHead::PrimitiveAdverb(_)
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
                Expr::ReadName(name) => match self.visible_binding(&name) {
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
                    if let Some(function) = self.implicit_operand(&verb.entity)? {
                        return self
                            .call_implicit_operand(function, None, y, pooled, depth)
                            .map_err(|error| error.at(verb_span));
                    }
                    if let Some(function) = self.explicit_operator(&verb.entity)? {
                        return self
                            .call_explicit_operator(function, None, y, pooled)
                            .map_err(|error| error.at(verb_span));
                    }
                    let verb = match self.resolve_function_entity(&verb.entity, 0) {
                        Ok(verb) => verb,
                        Err(error) if error.kind() == "unsupported" => {
                            return self
                                .call_composite(verb.entity, None, y, pooled, depth)
                                .map_err(|error| error.at(verb_span));
                        }
                        Err(error) => return Err(error),
                    };
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
                    if let Some(function) = self.implicit_operand(&verb.entity)? {
                        return self
                            .call_implicit_operand(function, Some(x), y, pooled, depth)
                            .map_err(|error| error.at(verb_span));
                    }
                    if let Some(function) = self.explicit_operator(&verb.entity)? {
                        return self
                            .call_explicit_operator(function, Some(x), y, pooled)
                            .map_err(|error| error.at(verb_span));
                    }
                    let verb = match self.resolve_function_entity(&verb.entity, 0) {
                        Ok(verb) => verb,
                        Err(error) if error.kind() == "unsupported" => {
                            return self
                                .call_composite(verb.entity, Some(x), y, pooled, depth)
                                .map_err(|error| error.at(verb_span));
                        }
                        Err(error) => return Err(error),
                    };
                    let operation = operation_label(&verb);
                    let call = if let Some(rank) = verb.rank {
                        kernels::ranked_dyad_ranks(verb.id.spelling(), rank[1], rank[2], x, y)
                    } else if !pooled
                        && matches!(
                            verb.id,
                            crate::primitive::PrimitiveId::IndexOf
                                | crate::primitive::PrimitiveId::Steps
                                | crate::primitive::PrimitiveId::Member
                        )
                    {
                        // The semantic-reference interpreter must not route
                        // through kernels::dyad -> index_ops::lookup and its
                        // physical planner. Keep ordered scalar/cell search
                        // independent of direct/reverse/prepared hash.
                        match verb.id {
                            crate::primitive::PrimitiveId::IndexOf => {
                                crate::search_reference::index_of(&x, &y, false)
                            }
                            crate::primitive::PrimitiveId::Steps => {
                                crate::search_reference::index_of(&x, &y, true)
                            }
                            crate::primitive::PrimitiveId::Member => {
                                crate::search_reference::member(&x, &y)
                            }
                            _ => unreachable!(),
                        }
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
                            crate::primitive::PrimitiveId::IndexOf
                                if matches!(
                                    x.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) && matches!(
                                    y.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) =>
                            {
                                crate::index_ops::index_of_cached(
                                    x,
                                    y,
                                    false,
                                    &mut self.exact_search_cache,
                                )
                            }
                            crate::primitive::PrimitiveId::Steps
                                if matches!(
                                    x.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) && matches!(
                                    y.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) =>
                            {
                                crate::index_ops::index_of_cached(
                                    x,
                                    y,
                                    true,
                                    &mut self.exact_search_cache,
                                )
                            }
                            crate::primitive::PrimitiveId::Member
                                if matches!(
                                    x.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) && matches!(
                                    y.data(),
                                    crate::value::Data::Int(_) | crate::value::Data::Bool(_)
                                ) =>
                            {
                                crate::index_ops::member_cached(x, y, &mut self.exact_search_cache)
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
        let Some(ParserNameBinding::KnownVerb { function, .. }) =
            engine.parser_analysis_binding("target")
        else {
            panic!()
        };
        assert_eq!(function.result_pos, FunctionPartOfSpeech::Verb);
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

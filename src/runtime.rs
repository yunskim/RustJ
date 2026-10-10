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
    use crate::parser::RuntimeParserHost;

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
        assert!(std::sync::Arc::ptr_eq(
            program.frontend.as_ref().unwrap(),
            capture.frontend.as_ref().unwrap()
        ));
        program.frontend.as_ref().unwrap().verify().unwrap();
        program
    }

    #[test]
    fn assignment_capture_uses_target_table_instead_of_read_fallback() {
        use crate::parser_capture::{CaptureEvent, ParseCapture};
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        let global = engine.binding_version("a");
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: HashSet::new(),
        });
        for expected_local in [None, Some(crate::semantic::NameVersion(1))] {
            let mut capture = ParseCapture::default();
            let mut host = ModifierFrame {
                parent: EngineParserHost {
                    engine: &mut engine,
                    pooled: false,
                },
            };
            assert_eq!(host.version("a"), expected_local.or(global));
            crate::parser::parse_runtime_host("a=.a+1", &mut host, Some(&mut capture)).unwrap();
            let (previous, version) = capture
                .events
                .iter()
                .find_map(|event| match event {
                    CaptureEvent::Commit {
                        previous, version, ..
                    } => Some((*previous, *version)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(previous, expected_local);
            assert_eq!(version, host.assignment_version("a", true).unwrap());
            capture.verify().unwrap();
        }
        assert_eq!(engine.binding_version("a"), global);
        let mut capture = ParseCapture::default();
        let mut host = ModifierFrame {
            parent: EngineParserHost {
                engine: &mut engine,
                pooled: false,
            },
        };
        crate::parser::parse_runtime_host("fresh=:11", &mut host, Some(&mut capture)).unwrap();
        assert!(
            capture
                .events
                .iter()
                .any(|event| matches!(event, CaptureEvent::Commit { previous: None, .. }))
        );
        assert!(
            crate::parser::parse_runtime_host(
                "a=.1 2+1 2 3",
                &mut host,
                Some(&mut ParseCapture::default())
            )
            .is_err()
        );
        assert_eq!(
            host.assignment_version("a", true),
            Some(crate::semantic::NameVersion(2))
        );
    }

    #[test]
    fn base_noun_capture_in_frame_records_bypass_and_checks_lexical_form() {
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: HashSet::new(),
        });
        parse_frame(&mut engine, "a=.9");
        let program = parse_frame(&mut engine, "a__");
        let context = program.frontend.as_ref().unwrap();
        let observation = context.name_uses[0].lookup.as_ref().unwrap();
        assert_eq!(observation.search, ScopeSearch::BaseLocaleOnly);
        assert_eq!(observation.local_state, LocalLookupState::Bypassed);
        let mut invalid = (**context).clone();
        invalid.words[0].flags.name_form = crate::enqueuer::NameForm::Simple;
        assert!(invalid.verify().is_err());
        let mut invalid = (**context).clone();
        invalid.name_uses[0].lookup.as_mut().unwrap().local_state = LocalLookupState::Bound;
        assert!(invalid.verify().is_err());
        parse_frame(&mut engine, "a__=.11");
        let JEntity::Noun(local) = &engine.local_frames.last().unwrap().names["a"].value else {
            panic!()
        };
        assert_eq!(local.int_at(0).unwrap(), 9);
        let JEntity::Noun(base) = &engine.names["a"].value else {
            panic!()
        };
        assert_eq!(base.int_at(0).unwrap(), 11);
    }

    #[test]
    fn named_locale_frame_capture_and_creation_respect_errors() {
        let mut engine = Engine::new();
        engine.eval("a=:1").unwrap();
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: HashSet::new(),
        });
        parse_frame(&mut engine, "a=.9");
        parse_frame(&mut engine, "a_probe_=.7");
        let program = parse_frame(&mut engine, "a_probe_");
        let context = program.frontend.as_ref().unwrap();
        let read = context.name_uses[0].lookup.as_ref().unwrap();
        let scope = engine.named_locales["probe"].instance;
        assert_eq!(read.search, ScopeSearch::DirectLocaleOnly(scope));
        assert_eq!(read.found, FoundScope::Locale(scope));
        assert_eq!(read.local_state, LocalLookupState::Bypassed);
        assert_ne!(scope, engine.namespace_instance);
        assert_eq!(
            engine.eval("missing_newplace_").unwrap_err().kind(),
            "unsupported"
        );
        assert!(engine.named_locales["newplace"].names.is_empty());
        assert_eq!(
            engine.eval("a_bad_=:1 2+1 2 3").unwrap_err().kind(),
            "length error"
        );
        assert!(!engine.named_locales.contains_key("bad"));
        let JEntity::Noun(local) = &engine.local_frames.last().unwrap().names["a"].value else {
            panic!()
        };
        assert_eq!(local.int_at(0).unwrap(), 9);
    }

    #[test]
    fn z_abandon_observation_accepts_actual_local_miss_frames() {
        use crate::parser_capture::{CaptureEvent, ParseCapture};
        for declared in [false, true] {
            let mut engine = Engine::new();
            engine.eval("a_z_=:7").unwrap();
            let frame = ScopeInstanceId::fresh();
            engine.local_frames.push(LocalFrame {
                instance: frame,
                names: HashMap::new(),
                declared: if declared {
                    HashSet::from(["a".to_owned()])
                } else {
                    HashSet::new()
                },
            });
            let mut capture = ParseCapture::default();
            let mut host = ModifierFrame {
                parent: EngineParserHost {
                    engine: &mut engine,
                    pooled: false,
                },
            };
            crate::parser::parse_runtime_host("a_:+0", &mut host, Some(&mut capture)).unwrap();
            capture.verify().unwrap();
            assert!(capture.events.iter().any(|event| matches!(event,
                CaptureEvent::Abandon { lookup, deleted: true, .. }
                    if lookup.frame == Some(frame) && lookup.local_state == if declared {
                        LocalLookupState::DeclaredUnbound
                    } else { LocalLookupState::Absent })));
            assert!(engine.direct_binding("a", "z").is_none());
        }
    }

    #[test]
    fn first_local_z_read_capture_retains_frame_and_own_commit_version() {
        use crate::parser_capture::{CaptureEvent, ParseCapture};
        let mut engine = Engine::new();
        engine.eval("a_z_=:7").unwrap();
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: HashSet::from(["a".to_owned()]),
        });
        let mut capture = ParseCapture::default();
        let mut host = ModifierFrame {
            parent: EngineParserHost {
                engine: &mut engine,
                pooled: false,
            },
        };
        crate::parser::parse_runtime_host("a=.a+1", &mut host, Some(&mut capture)).unwrap();
        capture.verify().unwrap();
        let context = capture.frontend.as_ref().unwrap();
        let obs = context.name_uses[0].lookup.as_ref().unwrap();
        assert!(matches!(obs.search, ScopeSearch::SimpleDefaultZ { .. }));
        assert_eq!(obs.local_state, LocalLookupState::DeclaredUnbound);
        assert!(obs.frame.is_some());
        assert!(
            capture
                .events
                .iter()
                .any(|event| matches!(event, CaptureEvent::Commit { previous: None, .. }))
        );
        let guard = crate::frontend_context::SimpleNameGuard::from_name_use(
            context,
            crate::frontend_context::NameUseId(0),
        )
        .unwrap();
        assert_eq!(
            engine.check_name_guard(&guard),
            crate::frontend_context::NameGuardCheck::LookupChanged
        );
    }

    fn guard_for(engine: &mut Engine, name: &str) -> crate::frontend_context::SimpleNameGuard {
        let program = parse_frame(engine, name);
        crate::frontend_context::SimpleNameGuard::from_name_use(
            program.frontend.as_ref().unwrap(),
            crate::frontend_context::NameUseId(0),
        )
        .unwrap()
    }

    #[test]
    fn alias_guard_tracks_local_fallback_and_shadowing_in_explicit_invocation_frame() {
        use crate::{frontend_context::NameUseId, name_guards::AliasGuardCheck};
        let mut engine = Engine::new();
        engine.eval("f=:+").unwrap();
        engine.eval("g=:f").unwrap();
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: ["f".to_owned()].into_iter().collect(),
        });
        let program = parse_frame(&mut engine, "g");
        let guard = engine
            .prepare_alias_call_guard(program.frontend.as_ref().unwrap(), NameUseId(0))
            .unwrap();
        assert_eq!(
            guard.reads()[1].observation().local_state,
            LocalLookupState::DeclaredUnbound
        );
        parse_frame(&mut engine, "f=.-");
        assert_eq!(
            engine.check_name_guard(guard.root()),
            crate::frontend_context::NameGuardCheck::ValidAtCheck
        );
        assert_eq!(
            engine.check_alias_call_guard(&guard),
            AliasGuardCheck::Invalidated {
                read: 1,
                reason: crate::frontend_context::NameGuardCheck::LookupChanged,
            }
        );
        let program = parse_frame(&mut engine, "g");
        let local = engine
            .prepare_alias_call_guard(program.frontend.as_ref().unwrap(), NameUseId(0))
            .unwrap();
        assert_eq!(
            local.reads()[1].observation().found,
            FoundScope::Local(engine.local_frames.last().unwrap().instance)
        );
        let value = engine
            .validate_alias_call_guard(&local)
            .unwrap()
            .apply_monad(Value::scalar(2))
            .unwrap();
        assert_eq!(value.int_at(0).unwrap(), -2);
        engine.local_frames.pop();
        assert_eq!(
            engine.check_alias_call_guard(&local),
            AliasGuardCheck::Invalidated {
                read: 0,
                reason: crate::frontend_context::NameGuardCheck::FrameChanged,
            }
        );
        assert_eq!(engine.eval("g 2").unwrap().unwrap().int_at(0).unwrap(), 2);
        use crate::name_guards::{AliasCall, AliasCallAttempt};
        let AliasCallAttempt::Miss(miss) = engine.try_alias_call(AliasCall::new(
            std::sync::Arc::new(local),
            None,
            Value::scalar(2),
        )) else {
            panic!()
        };
        let AliasCallAttempt::Miss(rejected) = engine.resume_alias_call(miss) else {
            panic!()
        };
        assert_eq!(rejected.call.right().int_at(0).unwrap(), 2);
    }

    #[test]
    fn simple_guard_rejects_rebinding_and_other_engines_but_not_unrelated_writes() {
        use crate::frontend_context::NameGuardCheck::*;
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        let guard = guard_for(&mut engine, "a");
        assert_eq!(guard.name(), "a");
        assert_eq!(engine.check_name_guard(&guard), ValidAtCheck);
        engine.eval("other=:8").unwrap();
        assert_eq!(engine.check_name_guard(&guard), ValidAtCheck);
        assert!(engine.eval("a=:1 2+1 2 3").is_err());
        assert_eq!(engine.check_name_guard(&guard), ValidAtCheck);
        engine.eval("a=:7").unwrap();
        assert_eq!(engine.check_name_guard(&guard), LookupChanged);
        let mut other = Engine::new();
        other.eval("a=:7").unwrap();
        assert_eq!(other.check_name_guard(&guard), EngineChanged);
    }

    #[test]
    fn simple_guard_tracks_unbound_fallback_shadowing_and_frame_lifetime() {
        use crate::frontend_context::NameGuardCheck::*;
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        let global = guard_for(&mut engine, "a");
        engine.local_frames.push(LocalFrame {
            instance: ScopeInstanceId::fresh(),
            names: HashMap::new(),
            declared: ["a".to_owned()].into_iter().collect(),
        });
        assert_eq!(engine.check_name_guard(&global), FrameChanged);
        let fallback = guard_for(&mut engine, "a");
        assert_eq!(engine.check_name_guard(&fallback), ValidAtCheck);
        engine.eval("a=:8").unwrap();
        assert_eq!(engine.check_name_guard(&fallback), LookupChanged);
        let fallback = guard_for(&mut engine, "a");
        parse_frame(&mut engine, "a=.9");
        assert_eq!(engine.check_name_guard(&fallback), LookupChanged);
        let local = guard_for(&mut engine, "a");
        engine.eval("other=:10").unwrap();
        assert_eq!(engine.check_name_guard(&local), ValidAtCheck);
        engine.local_frames.pop();
        assert_eq!(engine.check_name_guard(&local), FrameChanged);
        assert_eq!(engine.check_name_guard(&global), LookupChanged);
    }

    #[test]
    fn binding_generation_rejects_remove_recreate_aba_even_at_equal_version_and_pos() {
        use crate::frontend_context::NameGuardCheck::*;
        let mut engine = Engine::new();
        engine.eval("f=:+").unwrap();
        let guard = guard_for(&mut engine, "f");
        let old = engine.names.remove("f").unwrap();
        // Simulate future expunge/recreation without claiming support for 4!:55.
        engine.eval("f=:+").unwrap();
        assert_eq!(old.version, engine.names["f"].version);
        assert_ne!(old.generation, engine.names["f"].generation);
        assert_eq!(engine.check_name_guard(&guard), LookupChanged);
        engine.eval("f=:3").unwrap();
        assert_eq!(engine.check_name_guard(&guard), LookupChanged);
    }

    #[test]
    fn guard_admission_requires_runtime_bound_simple_name_and_valid_origin() {
        use crate::frontend_context::{NameUseId, SimpleNameGuard};
        let diagnostic = crate::parser::parse_frontend("a").unwrap();
        assert!(
            SimpleNameGuard::from_name_use(diagnostic.frontend.as_ref().unwrap(), NameUseId(0))
                .is_err()
        );
        let mut engine = Engine::new();
        let missing = parse_frame(&mut engine, "missing");
        assert!(
            SimpleNameGuard::from_name_use(missing.frontend.as_ref().unwrap(), NameUseId(0))
                .is_err()
        );
        engine.eval("a=:7").unwrap();
        let program = parse_frame(&mut engine, "a");
        let context = program.frontend.unwrap();
        let guard = SimpleNameGuard::from_name_use(&context, NameUseId(0)).unwrap();
        assert_eq!(guard.origin(), (context.unit, NameUseId(0)));
        assert!(SimpleNameGuard::from_name_use(&context, NameUseId(1)).is_err());
        let mut bad = (*context).clone();
        bad.name_uses[0].lookup.as_mut().unwrap().binding_generation = None;
        assert!(bad.verify().is_err());
        for spelling in ["a_base_", "a__loc", "a::"] {
            let mut locative = (*context).clone();
            locative.words[0].name = Some(spelling.into());
            assert!(SimpleNameGuard::from_name_use(&locative, NameUseId(0)).is_err());
        }
    }

    #[test]
    fn guard_miss_does_not_refresh_captured_noun_or_freeze_late_function() {
        use crate::frontend_context::NameGuardCheck::*;
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        let program = parse_frame(&mut engine, "a");
        let guard = crate::frontend_context::SimpleNameGuard::from_name_use(
            program.frontend.as_ref().unwrap(),
            crate::frontend_context::NameUseId(0),
        )
        .unwrap();
        engine.eval("a=:8").unwrap();
        assert_eq!(engine.check_name_guard(&guard), LookupChanged);
        let crate::semantic::ExprKind::Literal(value) = program.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(value.int_at(0).unwrap(), 7);
        engine.eval("f=:+").unwrap();
        let late = guard_for(&mut engine, "f");
        engine.eval("g=:f").unwrap();
        let alias = guard_for(&mut engine, "g");
        engine.eval("f=:*").unwrap();
        assert_eq!(engine.check_name_guard(&late), LookupChanged);
        // A guard for g does not cover its transitive late target f. A future
        // specialization must guard every semantic read it actually freezes.
        assert_eq!(engine.check_name_guard(&alias), ValidAtCheck);
        assert_eq!(engine.eval("g _2").unwrap().unwrap().int_at(0).unwrap(), -1);
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
    /// User named locales are independent symbol tables, never locative keys.
    named_locales: HashMap<String, NamedLocale>,
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

/// Only validated empty-direct-locale NAMEs enter this runtime boundary.
/// This selects the Engine's base namespace; it never searches local frames.
fn base_locative_key(name: &str) -> Option<&str> {
    name.strip_suffix("__")
}

struct NamedLocale {
    instance: crate::frontend_context::ScopeInstanceId,
    names: HashMap<String, Binding>,
}

/// Validated direct NAME: separate simple symbol and named locale spelling.
fn named_direct_address(name: &str) -> Option<(&str, &str)> {
    let text = name.strip_suffix('_')?;
    if name.ends_with("__") {
        return None;
    }
    text.rsplit_once('_')
}

// Only a single ordinary holder is admitted by this bounded noun read.
// General chained/debug resolution remains closed.
fn indirect_holder(name: &str) -> Option<&str> {
    let (_, holder) = name.split_once("__")?;
    (holder
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && !holder.contains("__")
        && !holder.ends_with('_'))
    .then_some(holder)
}

struct EngineParserHost<'a> {
    engine: &'a mut Engine,
    pooled: bool,
}
impl crate::parser::RuntimeParserHost for EngineParserHost<'_> {
    fn take_name(&mut self, name: &str, single_word: bool) -> Result<(JEntity, bool)> {
        self.engine.take_binding(name, single_word)
    }
    fn lookup_observation(&self, name: &str) -> Option<crate::frontend_context::LookupObservation> {
        if indirect_holder(name).is_some() {
            let (key, locale) = self.engine.indirect_named_address(name)?;
            if !self.engine.named_locales.contains_key(locale)
                || !matches!(
                    self.engine.direct_read_binding(key, locale),
                    Some(Binding {
                        value: JEntity::Noun(_),
                        ..
                    })
                )
            {
                return None;
            }
            return Some(self.engine.lookup_observation(name));
        }
        if let Some((_, locale)) = named_direct_address(name)
            && locale != "base"
            && !self.engine.named_locales.contains_key(locale)
        {
            // Normal lookup creates the selected locale. Do not invent its ID before that.
            return None;
        }
        Some(self.engine.lookup_observation(name))
    }
    fn function_name_ranks(&self, name: &str) -> Option<[i64; 3]> {
        if base_locative_key(name).is_some()
            || named_direct_address(name).is_some()
            || indirect_holder(name).is_some()
        {
            return None;
        }
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
    fn supports_base_locative_nouns(&self) -> bool {
        true
    }
    fn supports_named_direct_locative_nouns(&self) -> bool {
        true
    }
    fn supports_indirect_noun_reads(&self) -> bool {
        true
    }
    fn lookup(&mut self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
        if let Some(holder) = indirect_holder(name) {
            let binding = match self.engine.visible_binding(holder) {
                Some(binding) => binding,
                None if self.engine.direct_binding(holder, "z").is_some()
                    || self
                        .engine
                        .primitives
                        .resolve_extension_binding(holder)
                        .is_some() =>
                {
                    return Err(Error::Unsupported(
                        "indirect function holder resolution".into(),
                    ));
                }
                None => return Err(Error::Value(holder.to_owned())),
            };
            let JEntity::Noun(value) = &binding.value else {
                return Err(Error::Unsupported(
                    "indirect function holder resolution".into(),
                ));
            };
            if !value.shape().is_empty() {
                return Err(Error::Rank);
            }
            // Numbered locales cannot be created here; negative debug holders
            // remain a separate execution-context contract.
            let numbered = |value: &Value| match &value.data {
                crate::value::Data::Int(numbers) if value.shape().is_empty() => Some(numbers[0]),
                crate::value::Data::Bool(numbers) if value.shape().is_empty() => {
                    Some(i64::from(numbers[0]))
                }
                _ => None,
            };
            if let Some(number) = numbered(value) {
                return Err(if number >= 0 {
                    Error::Locale
                } else {
                    Error::Unsupported("indirect debug locale resolution".into())
                });
            }
            let crate::value::Data::Boxed(boxes) = &value.data else {
                return Err(Error::Domain);
            };
            let contents = &boxes[0];
            if let Some(number) = numbered(contents) {
                return Err(if number >= 0 {
                    Error::Locale
                } else {
                    Error::Unsupported("indirect debug locale resolution".into())
                });
            }
            if contents.shape().len() > 1 {
                return Err(Error::Rank);
            }
            if contents.is_empty() {
                return Err(Error::Length);
            }
            let crate::value::Data::Char(chars) = &contents.data else {
                return Err(Error::Domain);
            };
            if !chars.first().is_some_and(u8::is_ascii_alphabetic)
                || !chars.iter().all(u8::is_ascii_alphanumeric)
            {
                return Err(Error::IllFormedName);
            }
            let Some((key, locale)) = self.engine.indirect_named_address(name) else {
                return Err(Error::Unsupported(
                    "indirect noun namespace resolution".into(),
                ));
            };
            let (key, locale) = (key.to_owned(), locale.to_owned());
            self.engine.ensure_named_locale(&locale)?;
            return match self.engine.direct_read_binding(&key, &locale) {
                Some(Binding {
                    value: JEntity::Noun(value),
                    ..
                }) => Ok(Some(crate::parser::ParserNameBinding::Noun(value.clone()))),
                _ => Err(Error::Unsupported(
                    "indirect function/path-future reference".into(),
                )),
            };
        }
        if name.contains("__") && !name.ends_with('_') {
            return Err(Error::Unsupported(
                "chained/debug indirect namespace resolution".into(),
            ));
        }
        if let Some((key, locale)) = named_direct_address(name) {
            self.engine.ensure_named_locale(locale)?;
            return match self.engine.direct_read_binding(key, locale) {
                Some(Binding {
                    value: JEntity::Noun(value),
                    ..
                }) => Ok(Some(crate::parser::ParserNameBinding::Noun(value.clone()))),
                Some(_) => Err(Error::Unsupported(
                    "direct-locative function reference".into(),
                )),
                None => Err(Error::Unsupported(
                    "direct-locative path/future reference".into(),
                )),
            };
        }
        if let Some(key) = base_locative_key(name) {
            // sn.c/sl.c: an empty direct locale selects base, bypassing the
            // invocation-local table. Nouns snapshot at parser stack entry.
            return match self.engine.direct_read_binding(key, "base") {
                Some(Binding {
                    value: JEntity::Noun(value),
                    ..
                }) => Ok(Some(crate::parser::ParserNameBinding::Noun(value.clone()))),
                Some(_) => Err(Error::Unsupported(
                    "base-locative function reference".into(),
                )),
                None => Err(Error::Unsupported(
                    "unbound base-locative future reference".into(),
                )),
            };
        }
        Ok(self.engine.parser_name_binding(name))
    }
    fn gerund_binding(&self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
        Ok(self.engine.parser_name_binding(name))
    }
    fn version(&self, name: &str) -> Option<crate::semantic::NameVersion> {
        if let Some((key, locale)) = self.engine.indirect_named_address(name) {
            return self
                .engine
                .direct_read_binding(key, locale)
                .map(|binding| binding.version);
        }
        if let Some((key, locale)) = named_direct_address(name) {
            return self
                .engine
                .direct_read_binding(key, locale)
                .map(|binding| binding.version);
        }
        if let Some(key) = base_locative_key(name) {
            return self
                .engine
                .direct_read_binding(key, "base")
                .map(|binding| binding.version);
        }
        self.engine
            .visible_binding(name)
            .map(|binding| binding.version)
    }
    fn assignment_version(&self, name: &str, _local: bool) -> Option<crate::semantic::NameVersion> {
        if let Some((key, locale)) = named_direct_address(name) {
            return self
                .engine
                .direct_binding(key, locale)
                .map(|binding| binding.version);
        }
        self.engine
            .names
            .get(base_locative_key(name).unwrap_or(name))
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
        self.engine.commit_runtime_binding(name, value)
    }
}

/// Current invocation owns local values separately from the global namespace.
struct LocalFrame {
    instance: crate::frontend_context::ScopeInstanceId,
    names: HashMap<String, Binding>,
    declared: HashSet<String>,
}

/// Invocation-local control state, never part of logical array identity.
struct DefinitionForLoop {
    start: usize,
    do_index: usize,
    exit: usize,
    names: Option<(String, String)>,
    iterator: Option<Value>,
    count: Option<usize>,
    next: usize,
    owns_index: bool,
}

impl DefinitionForLoop {
    fn release(&self, frame: &mut LocalFrame) {
        if !self.owns_index {
            return;
        }
        let Some((_, index)) = &self.names else {
            return;
        };
        if let Some(binding) = frame.names.get_mut(index) {
            binding.read_only = false;
        }
    }
}

struct ModifierFrame<'a> {
    parent: EngineParserHost<'a>,
}
impl crate::parser::RuntimeParserHost for ModifierFrame<'_> {
    fn take_name(&mut self, name: &str, single_word: bool) -> Result<(JEntity, bool)> {
        self.parent.take_name(name, single_word)
    }
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
    fn supports_base_locative_nouns(&self) -> bool {
        true
    }
    fn supports_named_direct_locative_nouns(&self) -> bool {
        true
    }
    fn supports_indirect_noun_reads(&self) -> bool {
        true
    }
    fn lookup(&mut self, name: &str) -> Result<Option<crate::parser::ParserNameBinding>> {
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
    fn assignment_version(&self, name: &str, local: bool) -> Option<crate::semantic::NameVersion> {
        if local && base_locative_key(name).is_none() && named_direct_address(name).is_none() {
            return self
                .parent
                .engine
                .local_frames
                .last()?
                .names
                .get(name)
                .map(|binding| binding.version);
        }
        self.parent.assignment_version(name, local)
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
        if base_locative_key(name).is_some() || named_direct_address(name).is_some() {
            return engine.commit_runtime_binding(name, value);
        }
        if local {
            let frame = engine.local_frames.last_mut().expect("modifier frame");
            if frame
                .names
                .get(name)
                .is_some_and(|binding| binding.read_only)
            {
                return Err(Error::ReadOnly);
            }
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
            generation: crate::frontend_context::BindingGeneration::fresh(),
            read_only: false,
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
    generation: crate::frontend_context::BindingGeneration,
    read_only: bool,
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
            named_locales: HashMap::new(),
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
            named_locales: HashMap::new(),
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
        self.invoke_definition_body(operator, Some(left), right, None, pooled)
    }

    fn invoke_definition_body(
        &mut self,
        operator: std::sync::Arc<FunctionEntity>,
        left: Option<FunctionOperand>,
        right: Option<FunctionOperand>,
        arguments: Option<(Option<Value>, Value)>,
        pooled: bool,
    ) -> Result<JEntity> {
        let FunctionHead::ExplicitDefinition(code) = &operator.head else {
            return Err(Error::Domain);
        };
        let verb_call = arguments.is_some();
        let operand_call = left.is_some();
        let dyadic = arguments
            .as_ref()
            .map_or(right.is_some(), |(x, _)| x.is_some());
        let (section, controls) = if dyadic {
            (&code.dyad, &code.dyad_controls)
        } else {
            (&code.monad, &code.monad_controls)
        };
        let admission = |error, span| {
            code.diagnostic_error(
                error,
                crate::error::DiagnosticFrameKind::DefinitionAdmission,
                span,
                None,
            )
        };
        if section.is_empty() {
            return Err(admission(Error::Valence, None));
        }
        use crate::definition_control::ControlWord as W;
        use crate::definition_flow::{ControlJump, ControlKind as K};
        if let Some(node) = controls.iter().find(|node| {
            node.analysis_barrier
                || !matches!(
                    node.kind,
                    K::Body
                        | K::Test
                        | K::DoFor
                        | K::BreakFor
                        | K::Word(
                            W::If
                                | W::Do
                                | W::Else
                                | W::ElseIf
                                | W::End
                                | W::Return
                                | W::While
                                | W::Whilst
                                | W::Break
                                | W::Continue
                                | W::Try
                                | W::Catch
                                | W::CatchD
                                | W::For
                        )
                )
        }) {
            return Err(admission(
                Error::Unsupported("definition control flow outside executable subset".into()),
                Some(node.span.clone()),
            ));
        }
        // Reject unsupported framing before any statement has side effects.
        for sentence in &code.sentences[section.clone()] {
            if matches!(
                crate::parser::frame_definition_input(&code.body[sentence.span.clone()])
                    .map_err(|error| admission(error, Some(sentence.span.clone())))?,
                crate::parser::InputFrame::NeedMore
            ) {
                return Err(admission(
                    Error::Unsupported("nested explicit modifier definition scope".into()),
                    Some(sentence.span.clone()),
                ));
            }
        }
        // Each invocation currently nests the shared parser. Keep a conservative
        // Windows stack bound until the general executor uses explicit frames.
        const MAX_MODIFIER_INVOCATION_DEPTH: usize = 8;
        if self.definition_depth >= MAX_MODIFIER_INVOCATION_DEPTH {
            return Err(admission(Error::Limit, None));
        }
        let local = (|| -> Result<LocalFrame> {
            let mut local = LocalFrame {
                instance: crate::frontend_context::ScopeInstanceId::fresh(),
                names: HashMap::new(),
                declared: ["x", "y"].into_iter().map(str::to_owned).collect(),
            };
            if operand_call {
                local.declared.extend(["u".to_owned(), "m".to_owned()]);
            }
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
            for (name, alias, operand) in [("u", "m", left), ("v", "n", right)] {
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
            Ok(local)
        })()
        .map_err(|error| admission(error, None))?;
        let body_origin = code.origin.body(code.body.clone(), code.source_map.clone());
        self.definition_depth += 1;
        self.local_frames.push(local);
        let result = (|| {
            let mut frame = ModifierFrame {
                parent: EngineParserHost {
                    engine: self,
                    pooled,
                },
            };
            // cx.c initializes z to the Boolean empty matrix (mtm).
            let empty_result = || {
                Value::new(
                    vec![0, 0],
                    crate::value::Data::Bool(crate::storage::CpuStorage::new(Vec::new())),
                )
                .map(JEntity::Noun)
            };
            let mut last = Some(empty_result()?);
            let mut last_result_span = None;
            let mut test = None;
            let mut pc = 0;
            // Only currently protected try bodies catch errors. Branching out,
            // entering a handler, and recursion must not retain stale handlers.
            let mut handlers: Vec<(usize, usize, usize)> = Vec::new();
            let mut loops: Vec<DefinitionForLoop> = Vec::new();
            while let Some(node) = controls.get(pc) {
                handlers.retain(|&(start, end, _)| start < pc && pc < end);
                while loops
                    .last()
                    .is_some_and(|state| pc <= state.start || pc >= state.exit)
                {
                    loops.pop().expect("exiting loop").release(
                        frame
                            .parent
                            .engine
                            .local_frames
                            .last_mut()
                            .expect("definition frame"),
                    );
                }
                let step = (|| -> Result<Option<usize>> {
                    let jump = || match node.go {
                        ControlJump::Index(target) => Ok(target),
                        _ => Err(Error::Control),
                    };
                    match node.kind {
                        K::Word(W::Return) => return Ok(None),
                        K::Word(W::For) => {
                            let end = jump()?;
                            let do_index = match controls[end].go {
                                ControlJump::Index(index) if controls[index].kind == K::DoFor => {
                                    index
                                }
                                _ => return Err(Error::Control),
                            };
                            let spelling = &code.body[node.span.clone()];
                            let names = spelling
                                .strip_prefix("for_")
                                .and_then(|s| s.strip_suffix('.'))
                                .map(|name| (name.to_owned(), format!("{name}_index")));
                            if let Some((item, index)) = &names {
                                if item.len() > 249 {
                                    return Err(Error::IllFormedName);
                                }
                                let local = frame
                                    .parent
                                    .engine
                                    .local_frames
                                    .last_mut()
                                    .expect("definition frame");
                                local.declared.extend([item.clone(), index.clone()]);
                            }
                            loops.push(DefinitionForLoop {
                                start: pc,
                                do_index,
                                exit: end + 1,
                                names,
                                iterator: None,
                                count: None,
                                next: 0,
                                owns_index: false,
                            });
                            return Ok(Some(pc + 1));
                        }
                        K::DoFor => {
                            let state = loops.last_mut().ok_or(Error::Control)?;
                            if state.do_index != pc {
                                return Err(Error::Control);
                            }
                            let engine = &mut frame.parent.engine;
                            let local = engine.local_frames.last_mut().expect("definition frame");
                            if state.count.is_none() {
                                let value = match test.take() {
                                    Some(JEntity::Noun(value)) => value,
                                    Some(JEntity::Function(_)) => return Err(Error::NounResult),
                                    None => return Err(Error::Control),
                                };
                                if let Some((_, index)) = &state.names {
                                    if value.is_sparse() {
                                        return Err(Error::Unsupported(
                                            "sparse named for iterator".into(),
                                        ));
                                    }
                                    if local
                                        .names
                                        .get(index)
                                        .is_some_and(|binding| binding.read_only)
                                    {
                                        return Err(Error::ReadOnly);
                                    }
                                }
                                state.count = Some(value.shape().first().copied().unwrap_or(1));
                                if state.names.is_some() {
                                    state.iterator = Some(value.into_shared());
                                }
                            }
                            let count = state.count.expect("initialized loop");
                            if let Some((item, index)) = &state.names {
                                let iteration =
                                    i64::try_from(state.next).map_err(|_| Error::Limit)?;
                                store_binding(
                                    &mut local.names,
                                    &mut engine.pool,
                                    index.clone(),
                                    JEntity::Noun(Value::scalar(iteration)),
                                )?;
                                local.names.get_mut(index).expect("index binding").read_only = true;
                                state.owns_index = true;
                                let value = if state.next < count {
                                    let iterator = state.iterator.as_ref().expect("named iterator");
                                    iterator
                                        .view()
                                        .cell(iterator.shape().len().saturating_sub(1), state.next)?
                                        .to_owned()?
                                } else {
                                    Value::new(
                                        vec![0],
                                        crate::value::Data::Bool(crate::storage::CpuStorage::new(
                                            Vec::new(),
                                        )),
                                    )?
                                };
                                store_binding(
                                    &mut local.names,
                                    &mut engine.pool,
                                    item.clone(),
                                    JEntity::Noun(value),
                                )?;
                            }
                            if state.next < count {
                                state.next += 1;
                                return Ok(Some(pc + 1));
                            }
                            return Ok(Some(jump()?));
                        }
                        K::BreakFor => return Ok(Some(jump()?)),
                        K::Word(W::Try) => {
                            let first = jump()?;
                            let mut handler = first;
                            while !matches!(controls[handler].kind, K::Word(W::Catch | W::CatchD)) {
                                handler = match controls[handler].go {
                                    ControlJump::Index(target) if target < controls.len() => target,
                                    _ => return Err(Error::Control),
                                };
                            }
                            handlers.push((pc, first, handler + 1));
                            return Ok(Some(pc + 1));
                        }
                        K::Word(W::Do) => {
                            // cx.c CDO: empty/missing tests and nonnumeric nouns are
                            // true; numeric tests inspect the first atom only.
                            let truth = match test.take() {
                                None => true,
                                Some(JEntity::Function(_)) => return Err(Error::NounResult),
                                Some(JEntity::Noun(value)) => {
                                    if value.is_sparse() {
                                        return Err(Error::Unsupported(
                                            "sparse definition condition".into(),
                                        ));
                                    }
                                    value.is_empty()
                                        || match value.data() {
                                            crate::value::Data::Bool(_)
                                            | crate::value::Data::Int(_)
                                            | crate::value::Data::Float(_) => {
                                                value.float_at(0)? != 0.0
                                            }
                                            _ => true,
                                        }
                                }
                            };
                            return Ok(Some(if truth { pc + 1 } else { jump()? }));
                        }
                        K::Word(
                            W::Else
                            | W::ElseIf
                            | W::End
                            | W::Whilst
                            | W::Break
                            | W::Continue
                            | W::Catch
                            | W::CatchD,
                        ) => {
                            return Ok(Some(jump()?));
                        }
                        K::Word(W::If | W::While) => {
                            return Ok(Some(pc + 1));
                        }
                        K::Body | K::Test => {}
                        _ => unreachable!("preflight executable controls"),
                    }
                    let sentence = &code.sentences[section.clone()]
                        .iter()
                        .find(|sentence| sentence.line == node.line)
                        .expect("control physical sentence");
                    for word in &sentence.words[node.words.clone()] {
                        let name = &code.body[word.span.clone()];
                        if operand_call
                            && word.flags.lookup_name
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
                    let program = crate::parser::parse_runtime_source(
                        body_origin.slice(node.span.clone())?,
                        &mut frame,
                        None,
                    )?;
                    let assigned = program.has_assignment();
                    if let Some(expression) = program.expression {
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
                            && node.kind == K::Body
                            && controls[pc + 1..].iter().any(|next| next.kind == K::Body)
                        {
                            return Err(Error::NounResult);
                        }
                        if node.kind == K::Test {
                            test = Some(value);
                        } else {
                            last = Some(value);
                            last_result_span = Some(node.span.clone());
                        }
                    }
                    Ok(Some(pc + 1))
                })();
                match step {
                    Ok(Some(next)) => pc = next,
                    Ok(None) => break,
                    Err(error) => {
                        // Statement-parser spans are fragment-relative. Preserve
                        // original definition coordinates before caller relocation.
                        let mut context = error.context().cloned().unwrap_or_default();
                        let relative = context.span.clone().unwrap_or(0..node.span.len());
                        let body_span =
                            node.span.start + relative.start..node.span.start + relative.end;
                        if let Some(span) = code.source_map.original_span(body_span) {
                            context
                                .source_frames
                                .push(crate::error::DiagnosticSourceFrame {
                                    kind: if context.source_frames.is_empty() {
                                        crate::error::DiagnosticFrameKind::DefinitionBody
                                    } else {
                                        crate::error::DiagnosticFrameKind::DefinitionCall
                                    },
                                    origin: code.origin.clone(),
                                    source: code.source.clone(),
                                    definition_span: code.source_span.clone(),
                                    span,
                                    blame_word_index: context.blame_word_index,
                                });
                        }
                        let error = error.into_unlocated().with_context(context);
                        // Capability misses, verifier defects and backend failures
                        // never become successful values through a J catch.
                        if !error.is_j_catchable() {
                            return Err(error);
                        }
                        // cx.c forinitnames/forinit use BZ/BASSERT and leave the
                        // definition directly. Only CHECKNOUN routes a for-test
                        // non-noun through its surrounding catch handler.
                        if node.kind == K::Word(W::For)
                            || (node.kind == K::DoFor && !matches!(error.root(), Error::NounResult))
                        {
                            return Err(error);
                        }
                        let Some((_, _, handler)) = handlers.pop() else {
                            return Err(error);
                        };
                        pc = handler;
                        last = Some(empty_result()?);
                        last_result_span = Some(node.span.clone());
                        test = None;
                    }
                }
            }
            let returning = |error| {
                code.diagnostic_error(
                    error,
                    crate::error::DiagnosticFrameKind::DefinitionReturn,
                    last_result_span.clone(),
                    None,
                )
            };
            let value = last.ok_or_else(|| {
                returning(Error::Unsupported("empty explicit modifier result".into()))
            })?;
            if verb_call && matches!(value, JEntity::Function(_)) {
                return Err(returning(Error::NounResult));
            }
            // cx.c fixes only the first implicit locative on each branch.
            // Replacement operands and ordinary names remain untouched.
            match value {
                JEntity::Function(function) => Ok(JEntity::Function(
                    frame
                        .parent
                        .engine
                        .fix_implicit_return(&function, 0)
                        .map_err(returning)?,
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
                if current.result_pos == FunctionPartOfSpeech::Verb
                    && (code.result_pos == FunctionPartOfSpeech::Verb
                        || (code.operator_definition && !current.operands.is_empty())))
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
                if x.is_none() {
                    if let Some(result) =
                        crate::logical_executor::exact_empty_rank_reduction(operand, ranks[0], &y)
                    {
                        return result;
                    }
                }
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
        if function.operands.len() > 2 {
            return Err(Error::Domain);
        }
        let left = function.operands.first().map(copy_operand);
        let right = function.operands.get(1).map(copy_operand);
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
            ..
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
    /// Return the parser transport before binding/admission. Catalog class and
    /// version observations are assumptions, not runtime binding guards.
    pub fn parse_frontend(
        &self,
        source: &str,
    ) -> std::result::Result<crate::semantic::Program, crate::frontend_context::FrontendFailure>
    {
        crate::parser::parse_frontend_with_lookup(
            source,
            Some(&|name| self.parser_analysis_binding(name)),
        )
    }

    /// Inspect bindings without execution or mutation. Versions are Engine-local.
    /// Stable machine API: diagnostic wrappers are stripped before return.
    pub fn prepare_semantic(&self, source: &str) -> Result<crate::semantic::BoundProgram> {
        self.prepare_semantic_diagnostic(source)
            .map_err(Error::into_unlocated)
    }

    /// Lower without executing kernels, reading noun payloads or deleting names.
    /// Unlike BoundProgram, this bounded route carries ordered NAME effects.
    pub fn prepare_name_effects(&self, source: &str) -> Result<crate::name_effect_ir::Plan> {
        let program = self.parse_frontend(source).map_err(|failure| {
            // A catalog lookup failure is an admission failure, not an
            // executed J error that may overtake an earlier runtime action.
            if failure.error.kind() == "value error" {
                let mut error = Error::Unsupported("ordered NAME requires dynamic parsing".into());
                if let Some(context) = failure.error.context() {
                    error = error.with_context(context.clone());
                }
                error.in_phase(DiagnosticPhase::SemanticAnalysis)
            } else {
                failure.error
            }
        })?;
        crate::frontend_handoff::VerifiedFrontend::from_program(program)?.lower_name_effects()
    }

    /// Execute verified semantic operations once. No parser replay or fallback
    /// occurs after effects. Binding transitions are observations, not guards.
    pub fn execute_name_effects(
        &mut self,
        plan: &crate::name_effect_ir::Plan,
    ) -> crate::name_effect_ir::Execution {
        self.execute_name_effects_route(plan, None)
    }

    pub fn prepare_name_arrays(
        &self,
        source: &str,
    ) -> Result<crate::name_array_regions::ArrayPlan> {
        crate::name_array_regions::ArrayPlan::from_effects(self.prepare_name_effects(source)?)
    }

    pub fn execute_name_arrays(
        &mut self,
        plan: &crate::name_array_regions::ArrayPlan,
    ) -> crate::name_effect_ir::Execution {
        self.execute_name_effects_route(plan.effects(), Some(plan))
    }

    fn execute_name_effects_route(
        &mut self,
        plan: &crate::name_effect_ir::Plan,
        arrays: Option<&crate::name_array_regions::ArrayPlan>,
    ) -> crate::name_effect_ir::Execution {
        use crate::name_effect_ir::{EffectToken, Execution, NameObservation, Operation, ValueId};
        fn consume(values: &mut [Option<JEntity>], uses: &mut [usize], id: ValueId) -> JEntity {
            uses[id.0] -= 1;
            if uses[id.0] == 0 {
                values[id.0].take().expect("verified ready value")
            } else {
                match values[id.0].as_ref().expect("verified ready value") {
                    JEntity::Noun(value) => JEntity::Noun(value.clone()),
                    JEntity::Function(function) => JEntity::Function(function.clone()),
                }
            }
        }
        let mut completed = EffectToken(0);
        let mut names = Vec::new();
        let result = (|| -> Result<Option<Value>> {
            plan.verify()?;
            if let Some(arrays) = arrays {
                arrays.verify()?;
            }
            if !self.local_frames.is_empty() {
                return Err(Error::Unsupported("ordered NAME definition frame".into()));
            }
            // Only POS is a parse-specialization precondition. Do not prefetch
            // payloads or report missing-name errors ahead of the effect chain.
            for step in plan.steps() {
                if let Operation::Read { name, expected } | Operation::Take { name, expected, .. } =
                    &step.operation
                {
                    let observation = self.lookup_observation(name);
                    if observation
                        .binding_class
                        .is_some_and(|class| class != *expected)
                        || matches!(
                            observation.found,
                            crate::frontend_context::FoundScope::Extension
                        )
                        || (matches!(step.operation, Operation::Read { .. })
                            && observation.binding_class.is_none())
                    {
                        return Err(Error::Unsupported(
                            "ordered NAME POS precondition changed".into(),
                        ));
                    }
                }
            }
            let mut values: Vec<Option<JEntity>> = (0..plan.value_count()).map(|_| None).collect();
            let mut uses = vec![0; plan.value_count()];
            for step in plan.steps() {
                for input in step.operation.inputs() {
                    uses[input.0] += 1;
                }
            }
            uses[plan.result().0] += 1;
            let mut index = 0;
            while index < plan.steps().len() {
                if let Some(batch) = arrays.and_then(|arrays| arrays.batch_at_step(index)) {
                    // Replace all parent uses inside this batch with one import.
                    // A parent value with later uses is shared; otherwise it moves.
                    let mut inputs: Vec<_> = batch
                        .inputs()
                        .iter()
                        .map(|(id, count)| {
                            uses[id.0] -= count - 1;
                            let JEntity::Noun(value) = consume(&mut values, &mut uses, *id) else {
                                unreachable!("verified array batch input")
                            };
                            value
                        })
                        .collect();
                    // Only immutable literal payloads may be supplied ahead of
                    // their zero-operation transport checkpoint. No lookup,
                    // constructor or computation is performed here.
                    inputs.extend(
                        batch
                            .constants()
                            .iter()
                            .map(|(_, node)| plan.literal(*node).clone()),
                    );
                    let exports: Vec<_> = batch.outputs().iter().map(|(_, value)| *value).collect();
                    let progress =
                        crate::logical_executor::execute_outputs(batch.logical(), inputs, &exports);
                    match progress.result {
                        Ok(results) => {
                            // Internal uses are satisfied by batch SSA, not parent slots.
                            for step in &plan.steps()[batch.steps()] {
                                for id in step.operation.inputs() {
                                    if !batch.inputs().iter().any(|(input, _)| *input == id) {
                                        uses[id.0] -= 1;
                                    }
                                }
                            }
                            for ((id, _), value) in batch.outputs().iter().zip(results) {
                                values[id.0] = Some(JEntity::Noun(if uses[id.0] > 1 {
                                    value.into_shared()
                                } else {
                                    value
                                }));
                            }
                            completed = batch
                                .checkpoints()
                                .last()
                                .expect("nonempty verified batch")
                                .success;
                            index = batch.steps().end;
                            continue;
                        }
                        Err(error) => {
                            let failed = batch
                                .checkpoints()
                                .iter()
                                .find(|point| point.operations.end > progress.completed_operations)
                                .expect("verified batch failure checkpoint");
                            completed = failed.entry;
                            let step = &plan.steps()[failed.step];
                            return Err(error.at(step.span.clone()).blamed_on_word(step.blame.0));
                        }
                    }
                }
                let step = &plan.steps()[index];
                let observation = match &step.operation {
                    Operation::Read { name, .. }
                    | Operation::Take { name, .. }
                    | Operation::Commit { name, .. } => Some((name, self.lookup_observation(name))),
                    _ => None,
                };
                let mut deleted = false;
                let value = (|| -> Result<Option<JEntity>> {
                    Ok(match &step.operation {
                        Operation::Literal(node) => {
                            Some(JEntity::Noun(plan.literal(*node).clone()))
                        }
                        Operation::Function(node) => {
                            Some(JEntity::Function(plan.function(*node).clone()))
                        }
                        Operation::Read { name, .. } => Some(match self.visible_binding(name) {
                            Some(Binding {
                                value: JEntity::Noun(value),
                                ..
                            }) => JEntity::Noun(value.clone()),
                            Some(_) => return Err(Error::Domain),
                            None => return Err(Error::Value(name.clone())),
                        }),
                        Operation::Take {
                            name, single_word, ..
                        } => {
                            let (entity, removed) = self.take_binding(name, *single_word)?;
                            deleted = removed;
                            Some(entity)
                        }
                        Operation::Apply {
                            primitive,
                            function,
                            left,
                            right,
                        } => {
                            let JEntity::Noun(y) = consume(&mut values, &mut uses, *right) else {
                                unreachable!("verified noun operand")
                            };
                            let literal = |value| crate::semantic::Expr {
                                origin: None,
                                span: step.span.clone(),
                                kind: crate::semantic::ExprKind::Literal(value),
                            };
                            let verb = crate::semantic::Verb {
                                span: step.span.clone(),
                                target: crate::semantic::VerbTarget::Primitive(*primitive),
                                entity: plan.function(*function).clone(),
                            };
                            let kind = if let Some(left) = left {
                                let JEntity::Noun(x) = consume(&mut values, &mut uses, *left)
                                else {
                                    unreachable!("verified noun operand")
                                };
                                crate::semantic::ExprKind::Dyad {
                                    verb,
                                    left: Box::new(literal(x)),
                                    right: Box::new(literal(y)),
                                }
                            } else {
                                crate::semantic::ExprKind::Monad {
                                    verb,
                                    argument: Box::new(literal(y)),
                                }
                            };
                            Some(JEntity::Noun(self.interpret_ir(
                                crate::semantic::Expr {
                                    origin: None,
                                    span: step.span.clone(),
                                    kind,
                                },
                                true,
                                0,
                            )?))
                        }
                        Operation::Commit { name, value } => {
                            let value = consume(&mut values, &mut uses, *value);
                            self.commit_binding(name.clone(), value)?;
                            None
                        }
                    })
                })();
                if let Some((name, before)) = observation {
                    names.push(NameObservation {
                        step: index,
                        before,
                        after: self.lookup_observation(name),
                        deleted,
                    });
                }
                let value = value
                    .map_err(|error| error.at(step.span.clone()).blamed_on_word(step.blame.0))?;
                if let (Some(output), Some(value)) = (step.output, value) {
                    if uses[output.0] > 0 {
                        values[output.0] = Some(match value {
                            JEntity::Noun(value) if uses[output.0] > 1 => {
                                JEntity::Noun(value.into_shared())
                            }
                            value => value,
                        });
                    }
                }
                completed = step.after;
                index += 1;
            }
            if plan.program().has_assignment() {
                return Ok(None);
            }
            match consume(&mut values, &mut uses, plan.result()) {
                JEntity::Noun(value) => Ok(Some(value)),
                JEntity::Function(_) => Err(Error::Unsupported("function result display".into())),
            }
        })();
        Execution {
            result,
            completed,
            names,
        }
    }

    /// Inspect one stage without executing J kernels, definitions or writes.
    /// Accepted representations still require downstream admission and guards.
    pub fn admit_frontend(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::semantic::Program> {
        crate::admission::Admission::frontend(self.parse_frontend(source))
    }
    pub fn admit_frontend_handoff(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::frontend_handoff::VerifiedFrontend> {
        crate::admission::Admission::parsed(
            crate::admission::Stage::FrontendHandoff,
            self.parse_frontend(source),
            crate::frontend_handoff::VerifiedFrontend::from_program,
        )
    }
    pub fn admit_semantic(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::semantic::BoundProgram> {
        crate::admission::Admission::inspected(
            crate::admission::Stage::SemanticBinding,
            self.prepare_semantic_diagnostic(source),
        )
    }
    pub fn admit_j_graph(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::j_graph_ir::Plan> {
        crate::admission::Admission::inspected(
            crate::admission::Stage::JGraph,
            self.analyze_j_graph_diagnostic(source),
        )
    }
    pub fn admit_logical(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::compilation::CompilationAnalysis> {
        crate::admission::Admission::inspected(
            crate::admission::Stage::Logical,
            self.analyze_compilation_diagnostic(source),
        )
    }
    pub fn admit_name_effects(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::name_effect_ir::Plan> {
        crate::admission::Admission::inspected(
            crate::admission::Stage::NameEffects,
            self.prepare_name_effects(source),
        )
    }
    pub fn admit_name_arrays(
        &self,
        source: &str,
    ) -> crate::admission::Admission<crate::name_array_regions::ArrayPlan> {
        crate::admission::Admission::inspected(
            crate::admission::Stage::NameArrays,
            self.prepare_name_arrays(source),
        )
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

    // sl.c initializes a named locale with a z path. This slice supports its
    // own noun tables plus the bounded default z noun path; path mutation and numbered locales stay closed.
    fn ensure_named_locale(&mut self, locale: &str) -> Result<()> {
        if !locale
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        {
            return Err(Error::Unsupported("numbered locale namespace".into()));
        }
        if locale != "base" {
            self.named_locales
                .entry(locale.to_owned())
                .or_insert_with(|| NamedLocale {
                    instance: crate::frontend_context::ScopeInstanceId::fresh(),
                    names: HashMap::new(),
                });
        }
        Ok(())
    }
    // s.c::locindirect: scalar box containing an atomic/list literal locale name.
    // Base aliases and numbered/debug/chained resolution remain separate gates.
    fn indirect_named_address<'a>(&'a self, name: &'a str) -> Option<(&'a str, &'a str)> {
        let holder = indirect_holder(name)?;
        let (key, _) = name.split_once("__")?;
        let JEntity::Noun(value) = &self.visible_binding(holder)?.value else {
            return None;
        };
        if !value.shape().is_empty() {
            return None;
        }
        let crate::value::Data::Boxed(boxes) = &value.data else {
            return None;
        };
        let contents = &boxes[0];
        if contents.shape().len() > 1 {
            return None;
        }
        let crate::value::Data::Char(chars) = &contents.data else {
            return None;
        };
        if !chars.first().is_some_and(u8::is_ascii_alphabetic)
            || !chars.iter().all(u8::is_ascii_alphanumeric)
        {
            return None;
        }
        let locale = std::str::from_utf8(chars).ok()?;
        (locale != "base").then_some((key, locale))
    }

    fn direct_binding(&self, key: &str, locale: &str) -> Option<&Binding> {
        if locale == "base" {
            self.names.get(key)
        } else {
            self.named_locales.get(locale)?.names.get(key)
        }
    }

    /// A bounded named-locale noun path. Writes continue to use direct_binding.
    fn direct_read_binding(&self, key: &str, locale: &str) -> Option<&Binding> {
        self.direct_binding(key, locale).or_else(|| {
            if locale == "z" {
                None
            } else {
                self.direct_binding(key, "z")
            }
        })
    }

    fn commit_runtime_binding(&mut self, name: &str, value: JEntity) -> Result<JEntity> {
        if let Some((key, locale)) = named_direct_address(name) {
            if !matches!(value, JEntity::Noun(_)) {
                return Err(Error::Unsupported(
                    "direct-locative function assignment".into(),
                ));
            }
            self.ensure_named_locale(locale)?;
            if locale == "base" {
                return self.commit_binding(key.to_owned(), value);
            }
            return store_binding(
                &mut self
                    .named_locales
                    .get_mut(locale)
                    .expect("created locale")
                    .names,
                &mut self.pool,
                key.to_owned(),
                value,
            );
        }
        if let Some(key) = base_locative_key(name) {
            if !matches!(value, JEntity::Noun(_)) {
                return Err(Error::Unsupported(
                    "base-locative function assignment".into(),
                ));
            }
            return self.commit_binding(key.to_owned(), value);
        }
        self.commit_binding(name.to_owned(), value)
    }

    fn commit_binding(&mut self, name: String, value: JEntity) -> Result<JEntity> {
        store_binding(&mut self.names, &mut self.pool, name, value)
    }

    fn visible_binding(&self, name: &str) -> Option<&Binding> {
        self.local_frames
            .last()
            .and_then(|frame| frame.names.get(name))
            .or_else(|| self.names.get(name))
            .or_else(|| {
                self.direct_binding(name, "z")
                    .filter(|binding| matches!(binding.value, JEntity::Noun(_)))
            })
    }

    fn take_binding(&mut self, name: &str, single_word: bool) -> Result<(JEntity, bool)> {
        let base_key = base_locative_key(name).or_else(|| {
            named_direct_address(name).and_then(|(key, locale)| (locale == "base").then_some(key))
        });
        if let Some(key) = base_key {
            return match self.names.get(key) {
                Some(Binding {
                    value: JEntity::Noun(_),
                    ..
                }) => Ok((self.names.remove(key).expect("found base noun").value, true)),
                Some(_) => Err(Error::Unsupported("base-locative function abandon".into())),
                None => {
                    if self
                        .direct_binding(key, "z")
                        .is_some_and(|binding| matches!(binding.value, JEntity::Noun(_)))
                    {
                        return Ok((
                            self.named_locales
                                .get_mut("z")
                                .expect("found z")
                                .names
                                .remove(key)
                                .expect("found z noun")
                                .value,
                            true,
                        ));
                    }
                    Err(Error::Unsupported(
                        "base-locative path/future abandon".into(),
                    ))
                }
            };
        }
        if let Some((key, locale)) = named_direct_address(name) {
            let own = self.named_locales.get_mut(locale);
            return match own.and_then(|locale| locale.names.get(key)) {
                Some(Binding {
                    value: JEntity::Noun(_),
                    ..
                }) => Ok((
                    self.named_locales
                        .get_mut(locale)
                        .expect("found named locale")
                        .names
                        .remove(key)
                        .expect("found named noun")
                        .value,
                    true,
                )),
                Some(_) => Err(Error::Unsupported("named-locative function abandon".into())),
                None => {
                    if locale != "z"
                        && self
                            .direct_binding(key, "z")
                            .is_some_and(|binding| matches!(binding.value, JEntity::Noun(_)))
                    {
                        return Ok((
                            self.named_locales
                                .get_mut("z")
                                .expect("found z")
                                .names
                                .remove(key)
                                .expect("found z noun")
                                .value,
                            true,
                        ));
                    }
                    Err(Error::Unsupported(
                        "named-locative path/future abandon".into(),
                    ))
                }
            };
        }
        // Enqueue retains address form independently of abandon policy. Do not
        // reinterpret unsupported locatives as flat ordinary-name keys.
        if name.ends_with('_') || name.contains("__") {
            return Err(Error::Unsupported(
                "locative abandon namespace resolution".into(),
            ));
        }
        if let Some(frame) = self.local_frames.last_mut()
            && let Some(binding) = frame.names.get(name)
        {
            // p.c finlocal1 bypasses nameundco for a single local word.
            if single_word {
                let value = match &binding.value {
                    JEntity::Noun(value) => JEntity::Noun(value.clone()),
                    JEntity::Function(function) => JEntity::Function(function.clone()),
                };
                return Ok((value, false));
            }
            if binding.read_only {
                return Err(Error::Unsupported("abandon read-only loop binding".into()));
            }
            return Ok((frame.names.remove(name).expect("found local").value, true));
        }
        if let Some(binding) = self.names.remove(name) {
            return Ok((binding.value, true));
        }
        if let Some(locale) = self.named_locales.get_mut("z")
            && let Some(binding) = locale.names.remove(name)
        {
            return Ok((binding.value, true));
        }
        if self.primitives.resolve_extension_binding(name).is_some() {
            return Err(Error::Unsupported(
                "abandon extension registry binding".into(),
            ));
        }
        Err(Error::Value(name.to_owned()))
    }

    fn lookup_observation(&self, name: &str) -> crate::frontend_context::LookupObservation {
        use crate::frontend_context::{
            FoundScope, LocalLookupState, LookupObservation, ScopeSearch,
        };
        if let Some((key, locale)) = self.indirect_named_address(name)
            && let Some(start) = self.named_locales.get(locale).map(|locale| locale.instance)
            && let Some(binding) = self.direct_read_binding(key, locale)
        {
            let holder_name = indirect_holder(name).expect("validated indirect holder");
            let holder = self.lookup_observation(holder_name);
            let z = self
                .direct_binding(key, locale)
                .is_none()
                .then(|| self.named_locales["z"].instance);
            return LookupObservation {
                engine: self.namespace_instance,
                frame: self.local_frames.last().map(|frame| frame.instance),
                search: ScopeSearch::IndirectNamedNoun {
                    start,
                    z,
                    holder_found: holder.found,
                    holder_version: holder.binding_version.expect("noun holder"),
                    holder_generation: holder.binding_generation.expect("noun holder"),
                    holder_local_state: holder.local_state,
                },
                local_state: if self.local_frames.is_empty() {
                    LocalLookupState::NoFrame
                } else {
                    LocalLookupState::Bypassed
                },
                found: FoundScope::Locale(z.unwrap_or(start)),
                binding_version: Some(binding.version),
                binding_generation: Some(binding.generation),
                binding_class: Some(crate::parser::ParseClass::Noun),
            };
        }
        if let Some((key, locale)) = named_direct_address(name) {
            let start = if locale == "base" {
                Some(self.namespace_instance)
            } else {
                self.named_locales.get(locale).map(|locale| locale.instance)
            };
            let own = self.direct_binding(key, locale);
            let binding = self.direct_read_binding(key, locale);
            let z = (own.is_none() && binding.is_some()).then(|| self.named_locales["z"].instance);
            return LookupObservation {
                engine: self.namespace_instance,
                frame: self.local_frames.last().map(|frame| frame.instance),
                search: if let Some(z) = z {
                    if locale == "base" {
                        ScopeSearch::BaseDefaultZ { z }
                    } else {
                        ScopeSearch::NamedDefaultZ {
                            start: start.expect("successful lookup created locale"),
                            z,
                        }
                    }
                } else {
                    ScopeSearch::DirectLocaleOnly(start.expect("successful lookup created locale"))
                },
                local_state: if self.local_frames.is_empty() {
                    LocalLookupState::NoFrame
                } else {
                    LocalLookupState::Bypassed
                },
                found: if binding.is_none() {
                    FoundScope::Missing
                } else if locale == "base" && z.is_none() {
                    FoundScope::Global(self.namespace_instance)
                } else {
                    FoundScope::Locale(z.unwrap_or_else(|| start.unwrap()))
                },
                binding_version: binding.map(|binding| binding.version),
                binding_generation: binding.map(|binding| binding.generation),
                binding_class: binding.map(|binding| match &binding.value {
                    JEntity::Noun(_) => crate::parser::ParseClass::Noun,
                    JEntity::Function(function) => function.result_pos.into(),
                }),
            };
        }
        if let Some(key) = base_locative_key(name) {
            let own = self.names.get(key);
            let binding = self.direct_read_binding(key, "base");
            let z = (own.is_none() && binding.is_some()).then(|| self.named_locales["z"].instance);
            return LookupObservation {
                engine: self.namespace_instance,
                frame: self.local_frames.last().map(|frame| frame.instance),
                search: z.map_or(ScopeSearch::BaseLocaleOnly, |z| ScopeSearch::BaseDefaultZ {
                    z,
                }),
                local_state: if self.local_frames.is_empty() {
                    LocalLookupState::NoFrame
                } else {
                    LocalLookupState::Bypassed
                },
                found: if let Some(z) = z {
                    FoundScope::Locale(z)
                } else if binding.is_some() {
                    FoundScope::Global(self.namespace_instance)
                } else {
                    FoundScope::Missing
                },
                binding_version: binding.map(|binding| binding.version),
                binding_generation: binding.map(|binding| binding.generation),
                binding_class: binding.map(|binding| match &binding.value {
                    JEntity::Noun(_) => crate::parser::ParseClass::Noun,
                    JEntity::Function(function) => function.result_pos.into(),
                }),
            };
        }
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
            _ if self.direct_binding(name, "z").is_some() => {
                FoundScope::Locale(self.named_locales["z"].instance)
            }
            _ if self.primitives.resolve_extension_binding(name).is_some() => FoundScope::Extension,
            _ => FoundScope::Missing,
        };
        LookupObservation {
            engine: self.namespace_instance,
            frame: frame.map(|frame| frame.instance),
            search: if let FoundScope::Locale(z) = found {
                ScopeSearch::SimpleDefaultZ { z }
            } else if frame.is_some() {
                ScopeSearch::CurrentFrameThenGlobal
            } else {
                ScopeSearch::GlobalOnly
            },
            local_state,
            found,
            binding_version: self.visible_binding(name).map(|binding| binding.version),
            binding_generation: self.visible_binding(name).map(|binding| binding.generation),
            binding_class: self
                .visible_binding(name)
                .map(|binding| match &binding.value {
                    JEntity::Noun(_) => crate::parser::ParseClass::Noun,
                    JEntity::Function(function) => function.result_pos.into(),
                }),
        }
    }

    /// Recheck the full supported simple-name search. This does not execute a
    /// specialized plan or authorize skipping any future semantic lookup.
    pub fn check_name_guard(
        &self,
        guard: &crate::frontend_context::SimpleNameGuard,
    ) -> crate::frontend_context::NameGuardCheck {
        self.check_lookup_observation(&guard.name, &guard.expected)
    }

    /// Recheck a bounded direct/base noun lookup, including path hit and frame.
    pub fn check_locative_noun_guard(
        &self,
        guard: &crate::frontend_context::LocativeNounGuard,
    ) -> crate::frontend_context::NameGuardCheck {
        self.check_lookup_observation(&guard.name, &guard.expected)
    }

    fn check_lookup_observation(
        &self,
        name: &str,
        expected: &crate::frontend_context::LookupObservation,
    ) -> crate::frontend_context::NameGuardCheck {
        use crate::frontend_context::NameGuardCheck;
        if self.namespace_instance != expected.engine {
            return NameGuardCheck::EngineChanged;
        }
        if self.local_frames.last().map(|frame| frame.instance) != expected.frame {
            return NameGuardCheck::FrameChanged;
        }
        if self.lookup_observation(name) == *expected {
            NameGuardCheck::ValidAtCheck
        } else {
            NameGuardCheck::LookupChanged
        }
    }

    /// Inspect only ordinary call-target aliases. Never walk derived operands
    /// speculatively or execute a definition to discover its eventual target.
    pub fn prepare_alias_call_guard(
        &self,
        context: &std::sync::Arc<crate::frontend_context::FrontendContext>,
        id: crate::frontend_context::NameUseId,
    ) -> std::result::Result<
        crate::name_guards::AliasCallGuard,
        crate::name_guards::AliasGuardAdmission,
    > {
        use crate::frontend_context::{NameGuardCheck, NamePolicy, SimpleNameGuard};
        use crate::name_guards::{AliasCallGuard, AliasGuardAdmission as Failure, AliasRead};
        use crate::primitive::PrimitiveId;
        let root = SimpleNameGuard::from_name_use(context, id).map_err(Failure::InvalidOrigin)?;
        if context.name_uses[id.0].policy != NamePolicy::LateAtCall {
            return Err(Failure::UnsupportedTarget);
        }
        let checked = self.check_name_guard(&root);
        if checked != NameGuardCheck::ValidAtCheck {
            return Err(Failure::RootChanged(checked));
        }
        let mut name = root.name().to_owned();
        let mut reads = Vec::new();
        let mut seen = HashSet::new();
        for _ in 0..=crate::semantic::MAX_EXPR_DEPTH {
            // Each alias has the same supported simple-name search recipe.
            if !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || name.ends_with('_')
                || name.contains("__")
            {
                return Err(Failure::UnsupportedTarget);
            }
            let binding = self
                .visible_binding(&name)
                .ok_or_else(|| Failure::UnboundTarget(name.clone()))?;
            let JEntity::Function(target) = &binding.value else {
                return Err(Failure::WrongPartOfSpeech(name));
            };
            if target.result_pos != FunctionPartOfSpeech::Verb {
                return Err(Failure::WrongPartOfSpeech(name));
            }
            if !seen.insert(binding.generation) {
                return Err(Failure::Cycle(name));
            }
            reads.push(AliasRead {
                name: name.clone(),
                observation: self.lookup_observation(&name),
                target: target.clone(),
            });
            if !target.operands.is_empty() {
                return Err(Failure::UnsupportedTarget);
            }
            match &target.head {
                FunctionHead::NameRef(next) => name = next.clone(),
                FunctionHead::PrimitiveVerb(
                    id @ (PrimitiveId::Add
                    | PrimitiveId::Subtract
                    | PrimitiveId::Multiply
                    | PrimitiveId::Divide),
                ) => {
                    return Ok(AliasCallGuard {
                        context: context.clone(),
                        root,
                        reads,
                        primitive: *id,
                    });
                }
                _ => return Err(Failure::UnsupportedTarget),
            }
        }
        Err(Failure::DepthLimit)
    }

    pub fn check_alias_call_guard(
        &self,
        guard: &crate::name_guards::AliasCallGuard,
    ) -> crate::name_guards::AliasGuardCheck {
        use crate::{frontend_context::NameGuardCheck, name_guards::AliasGuardCheck};
        if guard.verify().is_err() {
            return AliasGuardCheck::InvalidRecipe;
        }
        for (read, dependency) in guard.reads.iter().enumerate() {
            let reason = self.check_lookup_observation(&dependency.name, &dependency.observation);
            if reason != NameGuardCheck::ValidAtCheck {
                return AliasGuardCheck::Invalidated { read, reason };
            }
        }
        AliasGuardCheck::ValidAtCheck
    }

    /// Validate after argument evaluation, immediately before the pure call.
    /// A miss returns without invoking kernels or replaying any prior effects.
    pub fn validate_alias_call_guard<'a>(
        &'a self,
        guard: &'a crate::name_guards::AliasCallGuard,
    ) -> std::result::Result<
        crate::name_guards::ValidatedAliasTarget<'a>,
        crate::name_guards::AliasGuardCheck,
    > {
        let check = self.check_alias_call_guard(guard);
        if check != crate::name_guards::AliasGuardCheck::ValidAtCheck {
            return Err(check);
        }
        Ok(crate::name_guards::ValidatedAliasTarget {
            _engine: self,
            guard,
        })
    }

    /// Try one pure guarded call with already evaluated arguments. A miss moves
    /// the unchanged call back to the caller; it is never a language error.
    pub fn try_alias_call(
        &self,
        call: crate::name_guards::AliasCall,
    ) -> crate::name_guards::AliasCallAttempt {
        use crate::name_guards::{AliasCallAttempt, AliasCallMiss};
        match self.validate_alias_call_guard(&call.guard) {
            Ok(lease) => AliasCallAttempt::Executed(match call.x {
                Some(x) => lease.apply_dyad(x, call.y),
                None => lease.apply_monad(call.y),
            }),
            Err(check) => AliasCallAttempt::Miss(AliasCallMiss { check, call }),
        }
    }

    /// Explicit semantic call at the same call-ready boundary. Recheck engine
    /// and frame, then use the original NameRef and retained values through the
    /// existing Rust caller. No source parsing or argument/effect replay occurs.
    /// Wrong-engine/frame or damaged recipes return ownership without calling.
    pub fn resume_alias_call(
        &mut self,
        miss: crate::name_guards::AliasCallMiss,
    ) -> crate::name_guards::AliasCallAttempt {
        use crate::{
            frontend_context::NameGuardCheck,
            name_guards::{AliasCallAttempt, AliasCallMiss, AliasGuardCheck},
        };
        let call = miss.call;
        let check = self.check_alias_call_guard(&call.guard);
        let rejected = matches!(
            check,
            AliasGuardCheck::InvalidRecipe
                | AliasGuardCheck::Invalidated {
                    reason: NameGuardCheck::EngineChanged | NameGuardCheck::FrameChanged,
                    ..
                }
        );
        if rejected {
            return AliasCallAttempt::Miss(AliasCallMiss { check, call });
        }
        let function = call
            .guard
            .call_function()
            .expect("verified original callee")
            .clone();
        AliasCallAttempt::Executed(self.call_entity(function, call.x, call.y, true, 0))
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
        self.eval_source_captured(crate::source::SourceUnit::new("<input>", source).origin())
    }

    pub fn eval_source_captured(
        &mut self,
        origin: crate::source::SourceOrigin,
    ) -> CapturedEvaluation {
        let mut capture = crate::parser_capture::ParseCapture::default();
        let result = self.eval_origin(origin, true, Some(&mut capture));
        if let Err(error) = &result {
            capture.failure = Some(crate::parser_capture::CaptureFailure {
                kind: error.kind().into(),
                context: error.context().cloned(),
            });
        }
        CapturedEvaluation { result, capture }
    }

    /// Evaluate a checked fragment of an immutable named input revision.
    /// Definition provenance survives later calls and input redefinition.
    pub fn eval_source_diagnostic(
        &mut self,
        origin: crate::source::SourceOrigin,
        semantic_reference: bool,
    ) -> Result<Option<Value>> {
        self.eval_origin(origin, !semantic_reference, None)
    }

    fn eval_program(
        &mut self,
        source: &str,
        pooled: bool,
        capture: Option<&mut crate::parser_capture::ParseCapture>,
    ) -> Result<Option<Value>> {
        self.eval_origin(
            crate::source::SourceUnit::new("<input>", source).origin(),
            pooled,
            capture,
        )
    }

    fn eval_origin(
        &mut self,
        origin: crate::source::SourceOrigin,
        pooled: bool,
        capture: Option<&mut crate::parser_capture::ParseCapture>,
    ) -> Result<Option<Value>> {
        let program = crate::parser::parse_runtime_source(
            origin,
            &mut EngineParserHost {
                engine: self,
                pooled,
            },
            capture,
        )?;
        let assigned = program.has_assignment();
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
        if assigned {
            // Runtime row 7 already committed the value. Even a later parser
            // exit error must not roll back that J-visible assignment.
            Ok(None)
        } else {
            match value {
                JEntity::Noun(value) => Ok(Some(value)),
                JEntity::Function(function) => {
                    // A bare unresolved ordinary NAME is a delayed nameref
                    // during parsing, but C reports its missing binding when
                    // the sentence result is requested. Do not call the verb.
                    if let FunctionHead::NameRef(name) = &function.head
                        && self.visible_binding(name).is_none()
                        && self.primitives.resolve_extension_binding(name).is_none()
                    {
                        return Err(Error::Value(name.clone()).at(function.span.clone()));
                    }
                    Err(Error::Unsupported(
                        if function.result_pos == FunctionPartOfSpeech::Verb {
                            "verb result display"
                        } else {
                            "modifier result display"
                        }
                        .into(),
                    ))
                }
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
            | FunctionHead::TakeName { .. }
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
                Expr::TakeName { .. } => Err(Error::Unsupported(
                    "deferred abandon requires ordered semantic execution".into(),
                )),
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

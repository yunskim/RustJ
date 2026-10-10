use rustj::{
    Engine,
    frontend_context::{FoundScope, NameUseId, ScopeSearch, SimpleNameGuard},
};
fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}
#[test]
fn named_noun_tables_are_independent_and_share_base_aliases() {
    let mut e = Engine::new();
    for source in ["a=:1", "a_probe_=:2", "a_other_=.3"] {
        e.eval(source).unwrap();
    }
    scalar(&mut e, "a", 1);
    scalar(&mut e, "a_probe_", 2);
    scalar(&mut e, "a_other_", 3);
    assert!(e.binding_version("a_probe_").is_none());
    e.eval("a_base_=.4").unwrap();
    scalar(&mut e, "a", 4);
    scalar(&mut e, "a__", 4);
    scalar(&mut e, "a_probe_", 2);
}
#[test]
fn named_nouns_preserve_array_snapshots_and_lookup_order() {
    let mut e = Engine::new();
    e.eval("a_probe_=:i.4").unwrap();
    e.eval("saved=:a_probe_").unwrap();
    e.eval("a_probe_=:a_probe_+10").unwrap();
    assert_eq!(
        e.eval("saved").unwrap().unwrap().json(),
        e.eval("i.4").unwrap().unwrap().json()
    );
    e.eval("a_probe_=:1").unwrap();
    scalar(&mut e, "a_probe_+(a_probe_=:2)", 4);
    scalar(&mut e, "(a_probe_=:3)+a_probe_", 5);
    assert_eq!(
        e.eval("missing_probe_+(a_probe_=:4)").unwrap_err().kind(),
        "unsupported"
    );
    scalar(&mut e, "a_probe_", 4);
    assert_eq!(
        e.eval("a_probe_=:1 2+1 2 3").unwrap_err().kind(),
        "length error"
    );
    scalar(&mut e, "a_probe_", 4);
}
#[test]
fn named_nouns_bypass_definition_local_shadowing() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a=:1").unwrap();
        e.eval("a_probe_=:7").unwrap();
        e.eval("f=:3 : 0\na=.9\na_probe_=.11\na+a_probe_\n)")
            .unwrap();
        let result = if semantic {
            e.eval_semantic_reference("f 0")
        } else {
            e.eval("f 0")
        };
        assert_eq!(result.unwrap().unwrap().int_at(0).unwrap(), 20);
        scalar(&mut e, "a", 1);
        scalar(&mut e, "a_probe_", 11);
    }
}
#[test]
fn capture_separates_locale_identity_from_spelling_and_binding_version() {
    let mut e = Engine::new();
    let mut identities = Vec::new();
    for locale in ["probe", "other"] {
        e.eval(&format!("a_{locale}_=:7")).unwrap();
        let r = e.eval_captured(&format!("a_{locale}_+1"));
        r.result.unwrap();
        r.capture.verify().unwrap();
        assert!(rustj::j_graph_ir::Plan::from_capture(&r.capture).is_err());
        let c = r.capture.frontend.as_ref().unwrap();
        let read = &c.name_uses[0];
        let obs = read.lookup.as_ref().unwrap();
        let ScopeSearch::DirectLocaleOnly(start) = obs.search else {
            panic!()
        };
        assert_eq!(obs.found, FoundScope::Locale(start));
        identities.push(start);
        assert!(SimpleNameGuard::from_name_use(c, NameUseId(0)).is_err());
        let mut bad = (**c).clone();
        bad.name_uses[0].lookup.as_mut().unwrap().found = FoundScope::Global(obs.engine);
        assert!(bad.verify().is_err());
        let r = e.eval_captured(&format!("a_{locale}_=:8"));
        r.result.unwrap();
        r.capture.verify().unwrap();
    }
    assert_ne!(identities[0], identities[1]);
}
#[test]
fn unsupported_paths_functions_and_computed_locatives_do_not_mutate_bindings() {
    let mut e = Engine::new();
    e.eval("a_probe_=:7").unwrap();
    for source in [
        "a_probe_=:+",
        "a_0_=:9",
        "a__holder",
        "'a_probe_'=:9",
        "a_probe__:9",
        "a_probe__:",
    ] {
        assert_eq!(
            e.eval(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
        scalar(&mut e, "a_probe_", 7);
    }
    assert_eq!(
        e.prepare_semantic("a_probe_").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn direct_locale_witness_cannot_swap_base_and_named_namespace_roles() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    e.eval("a_probe_=:7").unwrap();
    let capture = e.eval_captured("a_probe_");
    let ScopeSearch::DirectLocaleOnly(named) = capture.capture.frontend.as_ref().unwrap().name_uses
        [0]
    .lookup
    .as_ref()
    .unwrap()
    .search
    else {
        panic!()
    };
    for source in ["a_probe_", "a_base_"] {
        let r = e.eval_captured(source);
        r.result.unwrap();
        r.capture.verify().unwrap();
        let mut bad = (**r.capture.frontend.as_ref().unwrap()).clone();
        let read = bad.name_uses[0].lookup.as_mut().unwrap();
        if source == "a_probe_" {
            read.search = ScopeSearch::DirectLocaleOnly(read.engine);
            read.found = FoundScope::Global(read.engine);
        } else {
            let forged = named;
            read.search = ScopeSearch::DirectLocaleOnly(forged);
            read.found = FoundScope::Locale(forged);
        }
        assert!(bad.verify().is_err(), "forged namespace role: {source}");
    }
}

#[test]
fn locative_witness_cannot_masquerade_as_simple_name_search() {
    use rustj::frontend_context::LocalLookupState;
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    e.eval("a_probe_=:7").unwrap();
    let mut other = Engine::new();
    other.eval("a=:7").unwrap();
    let frame_capture = other.eval_captured("a");
    let frame = frame_capture.capture.frontend.as_ref().unwrap().name_uses[0]
        .lookup
        .as_ref()
        .unwrap()
        .engine;
    for source in ["a__", "a_base_", "a_probe_"] {
        let r = e.eval_captured(source);
        r.result.unwrap();
        r.capture.verify().unwrap();
        for search in [ScopeSearch::GlobalOnly, ScopeSearch::CurrentFrameThenGlobal] {
            let mut bad = (**r.capture.frontend.as_ref().unwrap()).clone();
            let lookup = bad.name_uses[0].lookup.as_mut().unwrap();
            lookup.search = search;
            (lookup.frame, lookup.local_state) = if search == ScopeSearch::GlobalOnly {
                (None, LocalLookupState::NoFrame)
            } else {
                (Some(frame), LocalLookupState::Absent)
            };
            lookup.found = FoundScope::Global(lookup.engine);
            assert!(bad.verify().is_err(), "forged simple search: {source}");
        }
    }
}

#[test]
fn z_own_nouns_keep_scope_versions_snapshots_and_failed_writes() {
    let mut e = Engine::new();
    e.eval("a=:1").unwrap();
    e.eval("a_probe_=:2").unwrap();
    e.eval("a_z_=:i.4").unwrap();
    let first = e.eval_captured("a_z_");
    first.result.unwrap();
    first.capture.verify().unwrap();
    let read = &first.capture.frontend.as_ref().unwrap().name_uses[0];
    let obs = read.lookup.as_ref().unwrap();
    let ScopeSearch::DirectLocaleOnly(z) = obs.search else {
        panic!()
    };
    assert_eq!(obs.found, FoundScope::Locale(z));
    assert_ne!(z, obs.engine);
    assert!(
        SimpleNameGuard::from_name_use(first.capture.frontend.as_ref().unwrap(), NameUseId(0))
            .is_err()
    );
    assert!(rustj::j_graph_ir::Plan::from_capture(&first.capture).is_err());
    e.eval("saved=:a_z_").unwrap();
    e.eval("a_z_=.a_z_+10").unwrap();
    assert_eq!(
        e.eval("saved").unwrap().unwrap().json(),
        e.eval("i.4").unwrap().unwrap().json()
    );
    e.eval("a_z_=:7").unwrap();
    let before = e.eval_captured("a_z_");
    e.eval("a_z_=:1 2+1 2 3").unwrap_err();
    e.eval("a_z_=:+").unwrap_err();
    let after = e.eval_captured("a_z_");
    after.result.unwrap();
    after.capture.verify().unwrap();
    assert_eq!(
        before.capture.frontend.as_ref().unwrap().name_uses[0].lookup,
        after.capture.frontend.as_ref().unwrap().name_uses[0].lookup
    );
    scalar(&mut e, "a_z_", 7);
    scalar(&mut e, "a", 1);
    scalar(&mut e, "a_probe_", 2);
    assert_eq!(e.eval("missing_z_").unwrap_err().kind(), "unsupported");
    assert_eq!(e.eval("missing_probe_").unwrap_err().kind(), "unsupported");
}

#[test]
fn z_own_nouns_bypass_local_shadowing_in_both_execution_paths() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a=:1").unwrap();
        e.eval("a_z_=:7").unwrap();
        e.eval("f=:3 : 0\na=.9\na_z_=.11\na+a_z_\n)").unwrap();
        let r = if semantic {
            e.eval_semantic_reference("f 0")
        } else {
            e.eval("f 0")
        };
        assert_eq!(r.unwrap().unwrap().int_at(0).unwrap(), 20);
        scalar(&mut e, "a", 1);
        scalar(&mut e, "a_z_", 11);
    }
}

#[test]
fn named_default_z_reads_separate_found_scope_and_assignment_target() {
    let mut e = Engine::new();
    e.eval("a_z_=:7").unwrap();
    e.eval("a=:1").unwrap();
    let r = e.eval_captured("a_probe_=:a_probe_+1");
    r.result.unwrap();
    r.capture.verify().unwrap();
    let previous = r
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            rustj::parser_capture::CaptureEvent::Commit { previous, .. } => Some(*previous),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        previous, None,
        "first named write must not inherit the z version"
    );
    let c = r.capture.frontend.as_ref().unwrap();
    let read = c.name_uses.iter().find(|n| n.lookup.is_some()).unwrap();
    let obs = read.lookup.as_ref().unwrap();
    let ScopeSearch::NamedDefaultZ { start, z } = obs.search else {
        panic!()
    };
    assert_ne!(start, z);
    assert_eq!(obs.found, FoundScope::Locale(z));
    assert!(SimpleNameGuard::from_name_use(c, NameUseId(0)).is_err());
    assert!(rustj::j_graph_ir::Plan::from_capture(&r.capture).is_err());
    scalar(&mut e, "a_probe_", 8);
    scalar(&mut e, "a_z_", 7);
    scalar(&mut e, "a", 1);
    scalar(&mut e, "a_other_", 7);
    for found in [FoundScope::Locale(start), FoundScope::Global(obs.engine)] {
        let mut bad = (**c).clone();
        bad.name_uses[0].lookup.as_mut().unwrap().found = found;
        assert!(bad.verify().is_err());
    }
}

#[test]
fn named_z_path_preserves_snapshot_local_bypass_and_failed_target_write() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a_z_=:i.4").unwrap();
        e.eval("saved=:a_probe_").unwrap();
        e.eval("a_z_=:a_z_+10").unwrap();
        assert_eq!(
            e.eval("saved").unwrap().unwrap().json(),
            e.eval("i.4").unwrap().unwrap().json()
        );
        e.eval("a_z_=:7").unwrap();
        assert_eq!(
            e.eval("a_probe_=:1 2+1 2 3").unwrap_err().kind(),
            "length error"
        );
        scalar(&mut e, "a_probe_", 7);
        e.eval("f=:3 : 0\na=.9\na_probe_=.a_probe_+1\na+a_probe_\n)")
            .unwrap();
        let r = if semantic {
            e.eval_semantic_reference("f 0")
        } else {
            e.eval("f 0")
        };
        assert_eq!(r.unwrap().unwrap().int_at(0).unwrap(), 17);
        scalar(&mut e, "a_probe_", 8);
        scalar(&mut e, "a_z_", 7);
    }
}

#[test]
fn explicit_base_z_reads_and_writes_preserve_the_target_table() {
    for source in ["a__", "a_base_"] {
        let mut e = Engine::new();
        e.eval("a_z_=:7").unwrap();
        let r = e.eval_captured(&format!("{source}=:{source}+1"));
        r.result.unwrap();
        r.capture.verify().unwrap();
        let previous = r
            .capture
            .events
            .iter()
            .find_map(|event| match event {
                rustj::parser_capture::CaptureEvent::Commit { previous, .. } => Some(*previous),
                _ => None,
            })
            .unwrap();
        assert_eq!(previous, None);
        let c = r.capture.frontend.as_ref().unwrap();
        let obs = c.name_uses[0].lookup.as_ref().unwrap();
        let ScopeSearch::BaseDefaultZ { z } = obs.search else {
            panic!()
        };
        assert_eq!(obs.found, FoundScope::Locale(z));
        assert_ne!(z, obs.engine);
        assert!(SimpleNameGuard::from_name_use(c, NameUseId(0)).is_err());
        assert!(rustj::j_graph_ir::Plan::from_capture(&r.capture).is_err());
        let mut bad = (**c).clone();
        bad.name_uses[0].lookup.as_mut().unwrap().found = FoundScope::Global(obs.engine);
        assert!(bad.verify().is_err());
        scalar(&mut e, "a", 8);
        scalar(&mut e, "a__", 8);
        scalar(&mut e, "a_base_", 8);
        scalar(&mut e, "a_probe_", 7);
        scalar(&mut e, "a_z_", 7);
    }
}

#[test]
fn base_z_snapshots_failed_writes_and_local_bypass_match_both_paths() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a_z_=:i.4").unwrap();
        e.eval("saved=:a__").unwrap();
        e.eval("a_z_=:a_z_+10").unwrap();
        assert_eq!(
            e.eval("saved").unwrap().unwrap().json(),
            e.eval("i.4").unwrap().unwrap().json()
        );
        e.eval("a_z_=:7").unwrap();
        assert_eq!(
            e.eval("a_base_=:1 2+1 2 3").unwrap_err().kind(),
            "length error"
        );
        scalar(&mut e, "a__", 7);
        e.eval("f=:3 : 0\na=.9\na__=.a_base_+1\na+a__\n)").unwrap();
        let r = if semantic {
            e.eval_semantic_reference("f 0")
        } else {
            e.eval("f 0")
        };
        assert_eq!(r.unwrap().unwrap().int_at(0).unwrap(), 17);
        scalar(&mut e, "a", 8);
        scalar(&mut e, "a_z_", 7);
    }
}

#[test]
fn ordinary_z_read_hits_do_not_become_global_assignment_versions_or_guards() {
    let mut e = Engine::new();
    e.eval("a_z_=:7").unwrap();
    let r = e.eval_captured("a=:a+1");
    r.result.unwrap();
    r.capture.verify().unwrap();
    assert!(r.capture.events.iter().any(|event| matches!(
        event,
        rustj::parser_capture::CaptureEvent::Commit { previous: None, .. }
    )));
    let c = r.capture.frontend.as_ref().unwrap();
    let obs = c.name_uses[0].lookup.as_ref().unwrap();
    let ScopeSearch::SimpleDefaultZ { z } = obs.search else {
        panic!()
    };
    assert_eq!(obs.found, FoundScope::Locale(z));
    assert!(SimpleNameGuard::from_name_use(c, NameUseId(0)).is_err());
    let mut bad = (**c).clone();
    bad.name_uses[0].lookup.as_mut().unwrap().found = FoundScope::Global(obs.engine);
    assert!(bad.verify().is_err());
    scalar(&mut e, "a", 8);
    scalar(&mut e, "a__", 8);
    scalar(&mut e, "a_base_", 8);
    scalar(&mut e, "a_probe_", 7);
    scalar(&mut e, "a_z_", 7);
}

#[test]
fn ordinary_z_snapshot_and_local_first_write_preserve_z_in_both_paths() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a_z_=:i.4").unwrap();
        e.eval("saved=:a").unwrap();
        e.eval("a_z_=:a_z_+10").unwrap();
        assert_eq!(
            e.eval("saved").unwrap().unwrap().json(),
            e.eval("i.4").unwrap().unwrap().json()
        );
        e.eval("a_z_=:7").unwrap();
        assert_eq!(e.eval("a=:1 2+1 2 3").unwrap_err().kind(), "length error");
        scalar(&mut e, "a", 7);
        e.eval("f=:3 : 0\na=.a+1\na+a__\n)").unwrap();
        let r = if semantic {
            e.eval_semantic_reference("f 0")
        } else {
            e.eval("f 0")
        };
        assert_eq!(r.unwrap().unwrap().int_at(0).unwrap(), 15);
        scalar(&mut e, "a", 7);
        scalar(&mut e, "a_z_", 7);
        assert!(e.binding_version("a").is_none());
        scalar(&mut e, "a_:", 7);
        assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
    }
}

#[test]
fn simple_z_abandon_deletes_found_table_and_retains_snapshot_and_error_order() {
    for semantic in [false, true] {
        let mut e = Engine::new();
        e.eval("a_z_=:i.3").unwrap();
        e.eval("saved=:a").unwrap();
        let run = |e: &mut Engine, source: &str| {
            if semantic {
                e.eval_semantic_reference(source)
            } else {
                e.eval(source)
            }
        };
        assert_eq!(
            run(&mut e, "a_:+1 2+1 2 3").unwrap_err().kind(),
            "length error"
        );
        assert_eq!(e.eval("a_z_").unwrap().unwrap().shape(), &[3]);
        assert_eq!(run(&mut e, "1 2+a_:").unwrap_err().kind(), "length error");
        assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
        assert_eq!(e.eval("saved").unwrap().unwrap().shape(), &[3]);
        e.eval("a_z_=:7").unwrap();
        e.eval("a=:9").unwrap();
        assert_eq!(run(&mut e, "a_:").unwrap().unwrap().int_at(0).unwrap(), 9);
        scalar(&mut e, "a", 7);
        e.eval("f=:3 : 'a_:'").unwrap();
        assert_eq!(run(&mut e, "f 0").unwrap().unwrap().int_at(0).unwrap(), 7);
        assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
    }
}

#[test]
fn simple_z_abandon_capture_identifies_deleted_locale() {
    let mut e = Engine::new();
    e.eval("a_z_=:7").unwrap();
    let r = e.eval_captured("a_:+a");
    assert_eq!(r.result.unwrap().unwrap().int_at(0).unwrap(), 14);
    r.capture.verify().unwrap();
    let event = r
        .capture
        .events
        .iter()
        .find(|event| matches!(event, rustj::parser_capture::CaptureEvent::Abandon { .. }))
        .unwrap();
    let rustj::parser_capture::CaptureEvent::Abandon {
        lookup, deleted, ..
    } = event
    else {
        unreachable!()
    };
    assert!(*deleted);
    let ScopeSearch::SimpleDefaultZ { z } = lookup.search else {
        panic!()
    };
    assert_eq!(lookup.found, FoundScope::Locale(z));
    assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
    let mut bad = r.capture.clone();
    if let rustj::parser_capture::CaptureEvent::Abandon { lookup, .. } = bad
        .events
        .iter_mut()
        .find(|event| matches!(event, rustj::parser_capture::CaptureEvent::Abandon { .. }))
        .unwrap()
    {
        lookup.found = FoundScope::Locale(lookup.engine);
    }
    assert!(bad.verify().is_err());
}

#[test]
fn z_abandon_observation_rejects_frame_alias_and_wrong_search_state() {
    use rustj::frontend_context::LocalLookupState;
    {
        let mut e = Engine::new();
        e.eval("a_z_=:7").unwrap();
        let r = e.eval_captured("a_:+a");
        r.result.unwrap().unwrap();
        r.capture.verify().unwrap();
        // Abandon events carry their own pre-action observation. Mutating that
        // observation must be rejected even when frontend NAME uses are valid.
        for mutation in 0..4 {
            let mut bad = r.capture.clone();
            let rustj::parser_capture::CaptureEvent::Abandon { lookup, .. } = bad
                .events
                .iter_mut()
                .find(|event| matches!(event, rustj::parser_capture::CaptureEvent::Abandon { .. }))
                .unwrap()
            else {
                unreachable!()
            };
            let ScopeSearch::SimpleDefaultZ { z } = lookup.search else {
                panic!()
            };
            match mutation {
                0 => lookup.frame = Some(lookup.engine),
                1 => lookup.frame = Some(z),
                2 => lookup.local_state = LocalLookupState::Bypassed,
                3 => lookup.local_state = LocalLookupState::DeclaredUnbound,
                _ => unreachable!(),
            }
            assert!(
                bad.verify().is_err(),
                "mutation {mutation} unexpectedly passed"
            );
        }
    }
}

#[test]
fn locative_abandon_enqueue_does_not_admit_unimplemented_deletion() {
    for semantic in [false, true] {
        for source in ["a_probe__:", "a_z__:", "a__holder_:"] {
            let mut e = Engine::new();
            e.eval("a=:9").unwrap();
            e.eval("a_probe_=:11").unwrap();
            e.eval("a_z_=:7").unwrap();
            let result = if semantic {
                e.eval_semantic_reference(source)
            } else {
                e.eval(source)
            };
            assert_eq!(result.unwrap_err().kind(), "unsupported");
            scalar(&mut e, "a", 9);
            scalar(&mut e, "a_probe_", 11);
            scalar(&mut e, "a_z_", 7);
        }
    }
}

#[test]
fn explicit_base_own_abandon_preserves_snapshots_and_first_error_order() {
    for semantic in [false, true] {
        for name in ["a___:", "a_base__:"] {
            let mut e = Engine::new();
            e.eval("a=:i.3").unwrap();
            e.eval("saved=:a").unwrap();
            e.eval("a_z_=:7").unwrap();
            let run = |e: &mut Engine, source: &str| {
                if semantic {
                    e.eval_semantic_reference(source)
                } else {
                    e.eval(source)
                }
            };
            assert_eq!(
                run(&mut e, &format!("{name}+1 2+1 2 3"))
                    .unwrap_err()
                    .kind(),
                "length error"
            );
            assert!(e.binding_version("a").is_some());
            assert_eq!(
                run(&mut e, &format!("1 2+{name}")).unwrap_err().kind(),
                "length error"
            );
            assert!(e.binding_version("a").is_none());
            assert_eq!(e.eval("saved").unwrap().unwrap().shape(), &[3]);
            scalar(&mut e, "a", 7);
            assert_eq!(run(&mut e, name).unwrap_err().kind(), "unsupported");
            scalar(&mut e, "a_z_", 7);
            e.eval("a=:+").unwrap();
            assert_eq!(run(&mut e, name).unwrap_err().kind(), "unsupported");
            scalar(&mut e, "a 3", 3);
        }
    }
}

#[test]
fn explicit_base_own_abandon_capture_and_local_bypass_select_base() {
    for (name, address) in [("a___:", "a__"), ("a_base__:", "a_base_")] {
        let mut e = Engine::new();
        e.eval("a=:9").unwrap();
        let r = e.eval_captured(&format!("{name}+0"));
        assert_eq!(r.result.unwrap().unwrap().int_at(0).unwrap(), 9);
        r.capture.verify().unwrap();
        assert!(r.capture.events.iter().any(|event| matches!(event,
            rustj::parser_capture::CaptureEvent::Abandon { name, lookup, deleted: true, .. }
            if name == address && lookup.found == FoundScope::Global(lookup.engine))));
        assert!(rustj::j_graph_ir::Plan::from_capture(&r.capture).is_err());
        assert!(e.binding_version("a").is_none());
        for semantic in [false, true] {
            e.eval("a=:9").unwrap();
            e.eval("a_z_=:7").unwrap();
            e.eval(&format!("f=:3 : 0\na=.11\n{name}+a\n)")).unwrap();
            let r = if semantic {
                e.eval_semantic_reference("f 0")
            } else {
                e.eval("f 0")
            };
            assert_eq!(r.unwrap().unwrap().int_at(0).unwrap(), 20);
            assert!(e.binding_version("a").is_none());
            scalar(&mut e, "a_z_", 7);
        }
    }
}

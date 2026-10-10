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

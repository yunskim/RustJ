use rustj::{Engine, parser_capture::CaptureEvent};
fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}

#[test]
fn straight_line_local_nouns_and_last_assignment_return_values_without_leaking() {
    let mut engine = Engine::new();
    engine.eval("smt=:90").unwrap();
    let version = engine.binding_version("smt");
    for definition in [
        "smadv=:1 : 0\nsmt=.u+1\nsmt*2\n)",
        "smadv=:{{ smt=.u+1\nsmt*2 }}",
    ] {
        engine.eval(definition).unwrap();
        scalar(&mut engine, "4 smadv", 10);
        scalar(&mut engine, "7 smadv", 16);
        scalar(&mut engine, "smt", 90);
        assert_eq!(engine.binding_version("smt"), version);
    }
    engine.eval("smlast=:1 : 'smt=.u+2'").unwrap();
    scalar(&mut engine, "5 smlast", 7);
    scalar(&mut engine, "smt", 90);
    engine.eval("smconj=:2 : 0\nsmt=.m+n\nsmt+1\n)").unwrap();
    scalar(&mut engine, "3 smconj 4", 8);
    engine.eval("smfresh=:1 : 0\nsmt=.smt+u\nsmt\n)").unwrap();
    scalar(&mut engine, "2 smfresh", 92);
    scalar(&mut engine, "3 smfresh", 93);
}

#[test]
fn local_function_names_execute_late_and_remain_names_after_frame_exit() {
    let mut engine = Engine::new();
    engine.eval("smf=:99").unwrap();
    let version = engine.binding_version("smf");
    engine.eval("smcall=:1 : 0\nsmf=.u\nsmf 7\n)").unwrap();
    scalar(&mut engine, "-smcall", -7);
    engine
        .eval("smescape=:1 : 0\nsmf=.u\nsmg=.smf/\nsmf=.-\nsmg\n)")
        .unwrap();
    engine.eval("smresult=:+smescape").unwrap();
    assert_eq!(
        engine.eval("smresult 1 2 3").unwrap_err().kind(),
        "value error"
    );
    engine.eval("smg=:*").unwrap();
    scalar(&mut engine, "smresult 7", 1);
    assert_eq!(engine.binding_version("smf"), version);
    scalar(&mut engine, "smf", 99);
    engine.eval("smreturned=:1 : 'smf=.u'").unwrap();
    engine.eval("smresult=:+smreturned").unwrap();
    scalar(&mut engine, "smresult 7", 7);
    engine.eval("smadvlocal=:1 : 0\nsma=./\nu sma\n)").unwrap();
    engine.eval("smresult=:+smadvlocal").unwrap();
    scalar(&mut engine, "smresult i.4", 6);
    assert!(engine.binding_version("smg").is_some());
    assert_eq!(engine.binding_version("sma"), None);
}

#[test]
fn global_effects_survive_later_errors_but_failed_rhs_and_outer_targets_do_not_commit() {
    let mut engine = Engine::new();
    for definition in [
        "smcount=:0",
        "smt=:88",
        "smkeep=:+",
        "smfail=:1 : 0\nsmcount=:smcount+1\nsmt=.u+1\n1 2+1 2 3\n)",
        "smrhs=:1 : 'smcount=:1 2+1 2 3'",
        "smgood=:1 : '7'",
    ] {
        engine.eval(definition).unwrap();
    }
    let keep = engine.binding_version("smkeep");
    let version = engine.binding_version("smcount").unwrap();
    let report = engine.eval_captured("smkeep=:4 smfail");
    assert_eq!(report.result.unwrap_err().kind(), "length error");
    report.capture.verify().unwrap();
    assert!(
        report
            .capture
            .events
            .iter()
            .any(|event| matches!(event, CaptureEvent::ExplicitModifierApply { .. }))
    );
    assert_eq!(engine.binding_version("smkeep"), keep);
    assert_eq!(engine.binding_version("smcount").unwrap().0, version.0 + 1);
    scalar(&mut engine, "smcount", 1);
    scalar(&mut engine, "smt", 88);
    let version = engine.binding_version("smcount");
    assert_eq!(engine.eval("+smrhs").unwrap_err().kind(), "length error");
    assert_eq!(engine.binding_version("smcount"), version);
    scalar(&mut engine, "+smgood", 7);
}

#[test]
fn public_assignment_to_declared_local_fails_and_nonnoun_intermediate_stops_execution() {
    let mut engine = Engine::new();
    engine.eval("smcount=:0").unwrap();
    for definition in [
        "smcollision=:1 : 0\nsmt=:1\nsmt=.2\nsmt\n)",
        "smcollision2=:1 : 0\nsmt=.1\nsmt=:2\nsmt\n)",
        "smnonnoun=:1 : 0\n+\nsmcount=:9\n7\n)",
    ] {
        engine.eval(definition).unwrap();
    }
    scalar(&mut engine, "+smcollision", 2);
    scalar(&mut engine, "smt", 1);
    assert_eq!(
        engine.eval("+smcollision2").unwrap_err().kind(),
        "domain error"
    );
    scalar(&mut engine, "smt", 1);
    assert_eq!(
        engine.eval("+smnonnoun").unwrap_err().kind(),
        "noun result was required"
    );
    scalar(&mut engine, "smcount", 0);
}

#[test]
fn returned_array_keeps_its_storage_after_local_frame_and_pool_cleanup() {
    let mut engine = Engine::new();
    engine.eval("smarray=:i.65536").unwrap();
    engine.eval("smcopy=:1 : 'smprivate=.u'").unwrap();
    let original = engine.eval("smarray").unwrap().unwrap();
    let returned = engine.eval("smarray smcopy").unwrap().unwrap();
    let rustj::Data::Int(before) = original.data() else {
        panic!()
    };
    let rustj::Data::Int(after) = returned.data() else {
        panic!()
    };
    assert_eq!(before.as_ptr(), after.as_ptr());
    engine.eval("smarray=:0").unwrap();
    engine.clear_output_cache();
    assert_eq!(returned.int_at(65535).unwrap(), 65535);
    assert_eq!(engine.binding_version("smprivate"), None);
}

#[test]
fn ordinary_function_names_can_cross_frames_without_capturing_caller_locals() {
    let mut engine = Engine::new();
    for source in [
        "smf=:+",
        "sminner=:1 : 'u 7'",
        "smreturn=:1 : 'u'",
        "smcross=:1 : 0\nsmf=.u\nsmf sminner\n)",
        "smescape=:1 : 0\nsmf=.u\nsmf smreturn\n)",
    ] {
        engine.eval(source).unwrap();
    }
    let version = engine.binding_version("smf");
    // The inner invocation sees global +, not the caller's private -.
    scalar(&mut engine, "-smcross", 7);
    engine.eval("smres=:-smescape").unwrap();
    scalar(&mut engine, "smres 7", 7);
    assert_eq!(engine.binding_version("smf"), version);
    engine.eval("smf=:-").unwrap();
    scalar(&mut engine, "smres 7", -7);
    engine.eval("smf=:1").unwrap();
    assert_eq!(engine.eval("smres 7").unwrap_err().kind(), "domain error");
}

#[test]
fn published_functions_keep_ordinary_names_and_committed_effects_after_failure() {
    let mut engine = Engine::new();
    for source in [
        "smf=:+",
        "smkeep=:+",
        "smpublish=:1 : 0\nsmf=.u\nsmexport=:smf/\n)",
        "smfail=:1 : 0\nsmf=.u\nsmexport=:smf/\n1 2+1 2 3\n)",
    ] {
        engine.eval(source).unwrap();
    }
    let version = engine.binding_version("smf");
    engine.eval("smres=:-smpublish").unwrap();
    scalar(&mut engine, "smexport 1 2 3", 6);
    engine.eval("smf=:-").unwrap();
    scalar(&mut engine, "smexport 1 2 3", 2);
    engine.eval("smf=:+").unwrap();
    let keep = engine.binding_version("smkeep");
    let export = engine.binding_version("smexport");
    let failure = engine.eval_captured("smkeep=:-smfail");
    assert_eq!(failure.result.unwrap_err().kind(), "length error");
    failure.capture.verify().unwrap();
    assert_eq!(engine.binding_version("smkeep"), keep);
    assert_ne!(engine.binding_version("smexport"), export);
    scalar(&mut engine, "smexport 1 2 3", 6);
    scalar(&mut engine, "smf 7", 7);
    assert_ne!(engine.binding_version("smf"), version);
    engine.eval("smf=:1").unwrap();
    assert_eq!(
        engine.eval("smexport 1 2 3").unwrap_err().kind(),
        "domain error"
    );
}

#[test]
fn uninitialized_local_and_operand_collision_preserve_late_name_identity() {
    let mut engine = Engine::new();
    for source in [
        "smf=:+",
        "smself=:1 : 'smf=.smf'",
        "smcollision=:1 : 'smf=.u'",
    ] {
        engine.eval(source).unwrap();
    }
    let version = engine.binding_version("smf");
    engine.eval("smres=:-smself").unwrap();
    engine.eval("smcollisionresult=:smf smcollision").unwrap();
    assert_eq!(engine.binding_version("smf"), version);
    scalar(&mut engine, "smres 7", 7);
    engine.eval("smf=:-").unwrap();
    scalar(&mut engine, "smres 7", -7);
    scalar(&mut engine, "smcollisionresult 7", -7);
    // Noun lookup still snapshots a value, even in the same source form.
    engine.eval("smf=:9").unwrap();
    scalar(&mut engine, "-smself", 9);
    scalar(&mut engine, "smf", 9);
}

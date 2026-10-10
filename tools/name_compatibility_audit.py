"""Bounded C/Rust NAME audit: record incompatibilities without counting them as passes."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from name_system_research import CASES
from oracle import Oracle

# Each fixture gets a fresh Engine and C instance. Every sentence, including
# setup and post-error reads, is compared. This does not observe internal events.
FIXTURES = [(name, setup + [query]) for name, setup, query, _, _ in CASES] + [
    ("array_alias_snapshot", ["a=:i.6", "b=:a", "a=:a+1", "a", "b"]),
    ("reshape_snapshot", ["a=:i.6", "b=:2 3$a", "a=:a+10", "b"]),
    ("failed_assignment_preserves_binding", ["a=:7", "a=:1 2+1 2 3", "a"]),
    ("verb_alias_chain", ["f=:+", "g=:f", "h=:g", "f=:*", "h 3"]),
    ("local_shadow", ["a=:7", "f=:3 : 'a=.9'", "f 0", "a"]),
    ("local_unbound_fallback", ["a=:7", "f=:3 : 'a=.a+1'", "f 0", "a"]),
    ("modifier_local_shadow", ["a=:7", "adv=:1 : 'a=.u+2'", "5 adv", "a"]),
    ("modifier_local_unbound_fallback", ["a=:7", "adv=:1 : 'a=.a+u'", "2 adv", "a"]),
    ("named_adverb_capture", ["a=:/", "f=:+a", "a=:\\", "f 1 2 3"]),
    ("nameless_adverb_alias", ["a=:/", "b=:a", "a=:\\", "+b 1 2 3"]),
    ("conjunction_alias", ["c=:@:", "d=:c", "c=:&", "(+d-) 3"]),
    ("base_locative", ["a=:7", "a__"]),
    ("named_noun_tables", ["a=:1", "a_probe_=:7", "a_other_=.11", "a", "a_probe_", "a_other_"]),
    ("named_noun_base_alias", ["a=:1", "a_base_=.7", "a__", "a", "a_base_"]),
    ("named_noun_snapshot", ["a_probe_=:i.4", "saved=:a_probe_", "a_probe_=:a_probe_+10", "saved", "a_probe_"]),
    ("named_noun_order", ["a_probe_=:1", "a_probe_+(a_probe_=:2)", "(a_probe_=:3)+a_probe_", "a_probe_"]),
    ("named_noun_local_write", ["a=:1", "a_probe_=:7", "f=:3 : 0\na=.9\na_probe_=.11\na+a_probe_\n)", "f 0", "a", "a_probe_"]),
    ("named_noun_failed_write", ["a_probe_=:7", "a_probe_=:1 2+1 2 3", "a_probe_"]),
    ("z_own_noun_table", ["a=:1", "a_probe_=:2", "a_z_=:7", "a_z_", "a", "a_probe_", "a_z_=.11", "a_z_", "a"]),
    ("z_own_failed_assignment", ["a_z_=:7", "a_z_=:1 2+1 2 3", "a_z_"]),
    ("z_own_array_snapshot", ["a_z_=:i.4", "saved=:a_z_", "a_z_=:a_z_+10", "saved", "a_z_"]),
    ("z_own_local_bypass", ["a=:1", "a_z_=:7", "f=:3 : 0\na=.9\na_z_=.11\na+a_z_\n)", "f 0", "a", "a_z_"]),
    ("named_default_z_path", ["pathnoun_z_=:9", "pathnoun_probe_", "pathnoun_probe_=:pathnoun_probe_+1", "pathnoun_probe_", "pathnoun_z_", "pathnoun_other_"]),
    ("named_z_failed_target", ["a_z_=:7", "a_probe_=:1 2+1 2 3", "a_probe_", "a_z_"]),
    ("named_z_snapshot", ["a_z_=:i.4", "saved=:a_probe_", "a_z_=:a_z_+10", "saved", "a_probe_"]),
    ("named_z_local_bypass", ["a_z_=:7", "f=:3 : 0\na=.9\na_probe_=.a_probe_+1\na+a_probe_\n)", "f 0", "a_probe_", "a_z_"]),
    ("base_z_alias_write", ["a_z_=:7", "a__", "a_base_", "a_base_=:a_base_+1", "a", "a__", "a_z_", "a_probe_"]),
    ("base_z_failed_write", ["a_z_=:7", "a__=:1 2+1 2 3", "a__", "a_base_", "a_z_"]),
    ("base_z_array_snapshot", ["a_z_=:i.4", "saved=:a__", "a_z_=:a_z_+10", "saved", "a_base_"]),
    ("base_z_local_bypass", ["a_z_=:7", "f=:3 : 0\na=.9\na__=.a_base_+1\na+a__\n)", "f 0", "a", "a_z_"]),
    ("base_z_abandon_aliases", ["a_z_=:7", "a___:+a_base_", "a+0"]),
    ("base_z_abandon_error_order", ["a_z_=:i.3", "saved=:a__", "a_base__:+1 2+1 2 3", "a", "1 2+a___:", "a+0", "saved"]),
    ("base_z_abandon_local_bypass", ["a_z_=:7", "f=:3 : 0\na=.11\na_base__:+a\n)", "f 0", "a+0"]),
    ("base_own_abandon_error_order", ["a=:i.3", "saved=:a", "a___:+1 2+1 2 3", "a", "1 2+a___:", "a+0", "saved"]),
    ("base_own_abandon_local_bypass", ["a=:9", "a_z_=:7", "f=:3 : 0\na=.11\na_base__:+a\n)", "f 0", "a", "a_z_"]),
    ("base_abandon_enqueue_boundary", ["a=:9", "a_z_=:7", "a___:+0", "a+0", "a_z_+0"]),
    ("base_alias_abandon_enqueue_boundary", ["a=:9", "a_z_=:7", "a_base__:+0", "a+0", "a_z_+0"]),
    ("named_z_abandon_fresh_locale", ["a_z_=:7", "a_fresh__:+0", "a+0"]),
    ("named_z_abandon_error_order", ["a_z_=:i.3", "saved=:a_probe_", "a_probe__:+1 2+1 2 3", "a_z_", "1 2+a_probe__:", "a+0", "saved"]),
    ("named_z_abandon_local_bypass", ["a_z_=:7", "f=:3 : 0\na=.11\na_probe__:+a\n)", "f 0", "a+0"]),
    ("named_own_abandon_error_order", ["a_probe_=:i.3", "saved=:a_probe_", "a_probe__:+1 2+1 2 3", "a_probe_", "1 2+a_probe__:", "saved"]),
    ("named_own_abandon_local_bypass", ["a=:9", "a_probe_=:7", "f=:3 : 0\na=.13\na_probe__:+a\n)", "f 0", "a"]),
    ("z_own_abandon_snapshot", ["a_z_=:i.3", "saved=:a_z_", "a_z__:+0", "a+0", "saved"]),
    ("named_abandon_enqueue_boundary", ["a_probe_=:9", "a_z_=:7", "a_probe__:+0", "a_probe_+0", "a_z_+0"]),
    ("z_abandon_enqueue_boundary", ["a_z_=:7", "a_z__:+0", "a+0"]),
    ("simple_z_abandon", ["a_z_=:7", "a_:+a", "a+0"]),
    ("simple_z_abandon_error_order", ["a_z_=:i.3", "saved=:a", "a_:+1 2+1 2 3", "a_z_", "1 2+a_:", "a+0", "saved"]),
    ("simple_z_abandon_local_fallback", ["a_z_=:7", "f=:3 : 'a_:'", "f 0", "a+0"]),
    ("simple_z_abandon_own_priority", ["a_z_=:7", "a=:9", "a_:", "a", "a_z_"]),
    ("simple_default_z", ["a_z_=:7", "a", "a__", "a_base_", "a_probe_", "a=:a+1", "a", "a_z_", "a_probe_"]),
    ("simple_z_failed_write", ["a_z_=:7", "a=:1 2+1 2 3", "a", "a_z_"]),
    ("simple_z_array_snapshot", ["a_z_=:i.4", "saved=:a", "a_z_=:a_z_+10", "saved", "a"]),
    ("simple_z_local_first_write", ["a_z_=:7", "f=:3 : 0\na=.a+1\na+a__\n)", "f 0", "a", "a_z_"]),
    ("base_default_z_path", ["pathnoun_z_=:9", "pathnoun__"]),
    ("base_noun_write", ["a=:7", "a__=.9", "a", "a__", "fresh__=:11", "fresh"]),
    ("base_noun_snapshot", ["a=:i.4", "saved=:a__", "a__=:a__+10", "saved", "a"]),
    ("base_noun_order", ["a=:1", "a__+(a__=:2)", "(a__=:3)+a__", "a"]),
    ("base_noun_local_shadow", ["a=:7", "f=:3 : '(a=.9)+a__'", "f 0", "a"]),
    ("base_noun_local_write", ["a=:7", "f=:3 : 0\na=.9\na__=.11\na+a__\n)", "f 0", "a"]),
    ("base_noun_failed_write", ["a=:7", "a__=:1 2+1 2 3", "a"]),
    ("indirect_locative_rebind", ["loc=:<'probe'", "f_probe_=:+", "f_other_=:*",
                                  "g=:f__loc", "loc=:<'other'", "g 3"]),
    ("locative_execution_context", ["a=:9", "a_probe_=:7", "f_probe_=:3 : 'a'", "f_probe_ 0"]),
    ("computed_single_assignment", ["'a'=:7", "a"]),
    ("multiple_assignment", ["'a b'=:3 4", "a", "b"]),
    ("undefined_function_call", ["g=:later", "g 3"]),
    ("invalid_name", ["foo_ 1"]),
]

# Prerequisite matrix for indirect noun resolution. Unsupported Rust results
# remain gaps; C values/errors and post-error state are observable contracts.
INDIRECT_NOUN_FIXTURES = [
    ("indirect_noun_commit_holder", ["a_probe_=:7", "holder=:<'probe'", "a__holder=:holder=:<'other'", "a_probe_", "a_other_", "holder"]),
    ("indirect_noun_self_holder", ["holder=:<'base'", "holder__holder=:<'other'", "holder"]),
    ("indirect_noun_local_write", ["a=:13", "holder=:<'base'", "f=:3 : 0\nholder=.<'fresh'\na=.99\na__holder=.21\na+a__holder\n)", "f 0", "a_fresh_", "a", "holder"]),
    ("indirect_noun_invalid_write_order", ["a_probe_=:7", "holder=:7", "a__holder=:1 2+1 2 3", "a__holder=:9", "a_probe_", "holder"]),
    ("indirect_noun_holder_rebind", ["a_probe_=:7", "a_other_=:11", "holder=:<'probe'", "a__holder+0", "holder=:<'other'", "a__holder+0"]),
    ("indirect_noun_z_write", ["a_z_=:7", "holder=:<'probe'", "a__holder+0", "a__holder=:9", "a_probe_+0", "a_z_+0"]),
    ("indirect_noun_snapshot", ["a_probe_=:i.3", "holder=:<'probe'", "saved=:a__holder", "a_probe_=:a_probe_+10", "saved"]),
    ("indirect_noun_failed_rhs", ["a_probe_=:7", "holder=:<'probe'", "a__holder=:1 2+1 2 3", "a_probe_+0"]),
    ("indirect_noun_missing_holder", ["a_probe_=:7", "a__missingholder+0", "a_probe_+0"]),
    ("indirect_noun_numeric_holder", ["a_probe_=:7", "holder=:7", "a__holder+0", "a_probe_+0"]),
    ("indirect_noun_unboxed_holder", ["a_probe_=:7", "holder=:'probe'", "a__holder+0", "a_probe_+0"]),
    ("indirect_noun_first_error", ["holder=:7", "a__holder+(1 2+1 2 3)"]),
]
FIXTURES += INDIRECT_NOUN_FIXTURES
FIXTURES += [
    ("indirect_base_holder_snapshot", ["a=:i.3", "holder=:<'base'", "saved=:a__holder", "a=:a+10", "saved"]),
    ("indirect_base_holder_z_rebind", ["fallback_z_=:11", "fallback_probe_=:9", "holder=:<'base'", "fallback__holder+0", "holder=:<'probe'", "fallback__holder+0"]),
]


# Invalid holder contents follow s.c::locindirect rank/length/type/name order.
INDIRECT_HOLDER_VALIDATION_FIXTURES = [
    ("indirect_holder_" + label,
     ["a_probe_=:7", "holder=:" + holder, "a__holder+(a_probe_=:9)",
      "a_probe_+0", "(a_probe_=:11)+a__holder", "a_probe_+0"])
    for label, holder in [
        ("boolean", "1"), ("float", "1.5"), ("char_atom", "'p'"),
        ("empty", "<''"), ("numeric_list", "<1 2"),
        ("matrix", "<i.2 2"), ("boxed_float", "<2.5"),
        ("invalid_name", "<'bad_name'"), ("boxed_number", "<7"),
    ]
]
FIXTURES += INDIRECT_HOLDER_VALIDATION_FIXTURES
FIXTURES += [
    ("indirect_absent_debug_" + label,
     ["a_probe_=:7", "holder=:" + holder, "a__holder+(a_probe_=:9)",
      "a_probe_+0", "(a_probe_=:11)+a__holder", "a_probe_+0"])
    for label, holder in [("minus_one", "_1"), ("boxed_minus_one", "<_1"),
                          ("minus_two", "_2"), ("boxed_minus_two", "<_2")]
]



# Boundary probes pin observable runtime behavior. They do not test a future
# admission verifier or parser continuation and must never be reported as such.
BOUNDARY_FIXTURES = [
    ("pure_array_region", ["a=:i.3", "b=:a+a*a", "b", "a"]),
    ("shape_unknown_is_not_a_parse_demand", ["n=:4", "i.n"]),
    ("literal_rank_constructor", ['(+/"1) i.2 3']),
    ("computed_rank_constructor", ['(+/"(1+0)) i.2 3']),
    ("assignment_then_name_read", ["a=:1", "a+a=:2", "a"]),
    ("modifier_write_then_left_read", ["a=:1", "adv=:1 : 'a=:u'", "a+2 adv", "a"]),
    ("right_error_precedes_missing_left_name", ["missing+(1 2+1 2 3)"]),
    ("effect_survives_later_error", ["count=:0", "adv=:1 : 'count=:count+u'",
                                   "(1 2+1 2 3)+5 adv", "count"]),
    ("dynamic_source_request", ['". \'1+2\'']),
    ("late_target_pos_change", ["f=:+", "g=:f", "f=:7", "g 3"]),
]

# String target assignment follows p.c::jtis. Post-failure observations matter:
# later invalid names must not roll back earlier successful writes.
ASSIGNMENT_FIXTURES = [
    ("string_single", ["'a'=:7 8", "a"]),
    ("string_spaces", ["' a '=:7", "a"]),
    ("string_multiple", ["'a b'=:3 4", "a", "b"]),
    ("string_scalar_extension", ["'a b'=:7", "a", "b"]),
    ("string_items", ["'a b'=:2 3$i.6", "a", "b"]),
    ("string_zero_atom_items", ["'a b'=:2 0$i.0", "a", "b"]),
    ("string_open_scalar", ["'a b'=:<7 8", "a", "b"]),
    ("string_open_items", ["'a b'=:(<7),<8 9", "a", "b"]),
    ("string_duplicate", ["'a a'=:3 4", "a"]),
    ("string_empty", ["''=:i.0"]),
    ("string_empty_invalid", ["''=:7"]),
    ("string_empty_function", ["''=:+"]),
    ("string_multiple_function", ["'a b'=:+"]),
    ("string_mismatch", ["a=:9", "'a b'=:3 4 5", "a", "b+0"]),
    ("string_partial_invalid", ["'a 1bad'=:3 4", "a", "b+0"]),
    ("string_invalid_first", ["'1bad a'=:3 4", "a+0"]),
    ("string_word_formation", ["'a+b'=:1", "a"]),
    ("string_rhs_failure_first", ["a=:9", "'a 1bad'=:1 2+1 2 3", "a"]),
    ("string_verb_alias", ["op=:+", "'f'=:op", "op=:-", "f 3"]),
    ("string_adverb", ["'adv'=:/", "+adv 1 2 3"]),
    ("string_local_direct", ["a=:99", "f=:{{\n'a b'=.y,y+1\na+b\n}}", "f 3", "a", "b+0"]),
    ("string_local_explicit", ["a=:99", "f=:3 : 0\n'a b'=.y,y+1\na+b\n)", "f 3", "a", "b+0"]),
    ("string_dynamic_local", ["a=:99", "f=:{{\nnames=.'a b'\n(names)=.y,y+1\na+b\n}}", "f 3", "a", "b+0"]),
    ("string_global_collision", ["a=:9", "f=:{{\nb=.1\n'a b'=:3 4\n0\n}}", "f 0", "a", "b+0"]),
    ("string_nonfinal", ["a=:1", "a+('a'=:7)", "a"]),
    ("string_chained", ["'a b'=:c=:3 4", "a", "b", "c"]),
    ("string_readonly_partial", ["f=:{{for_i. i.1 do. try. 'a i_index'=.3 4 catch. a return. end. end.}}", "f 0"]),
    ("string_boxed_rows", ["'a b'=:2 2$<7 8", "a", "b"]),
]

# Both definition spellings must preserve the same NAME timing and scope rules.
SCOPE_FIXTURES = [
    ("explicit_local_fallback", ["t=:10", "add=:1 : 't=.t+u'", "2 add", "3 add", "t"]),
    ("direct_local_fallback", ["t=:10", "add=:{{ t=.t+u }}", "2 add", "3 add", "t"]),
    ("explicit_local_function_escape", ["t=:+", "make=:1 : 0\nt=.u\nt/\n)",
                                        "res=:-make", "res 1 2 3", "t=:-", "res 1 2 3"]),
    ("direct_local_function_escape", ["t=:+", "make=:{{ t=.u\nt/ }}",
                                      "res=:-make", "res 1 2 3", "t=:-", "res 1 2 3"]),
    ("explicit_global_write", ["a=:10", "set=:1 : 'a=:u'", "3 set", "a"]),
    ("direct_global_write", ["a=:10", "set=:{{ a=:u }}", "3 set", "a"]),
    ("explicit_local_global_collision", ["a=:10", "bad=:1 : 0\na=.u\na=:7\na\n)", "3 bad", "a"]),
    ("direct_local_global_collision", ["a=:10", "bad=:{{ a=.u\na=:7\na }}", "3 bad", "a"]),
    ("top_level_local_copula_is_global", ["outside=.7", "outside"]),
    ("callee_does_not_capture_caller_local", ["f=:+", "inner=:1 : 'u 7'",
                                             "outer=:{{ f=.u\nf inner }}", "-outer", "f 7"]),
]

# Only these assignment setup blocks need script input (JDo has no interactive
# block-input callback). Keep original source identical on both sides and never
# script-wrap value queries, whose noun results must remain observable.
DEFINITION_FIXTURES = [
    ("diagnostic_return_direct", ["f=:{{+}}", "f 0"]),
    ("diagnostic_return_explicit", ["f=:3 : '+'", "f 0"]),
    ("diagnostic_return_post_effect", ["count=:0", "saved=:99", "f=:3 : 0\ncount=:count+1\ntry. local=.+ catch. 42 end.\n)", "saved=:f 0", "count", "saved", "local+0"]),
    ("diagnostic_missing_valence", ["f=:4 : 'x+y'", "f 0", "2 f 3"]),
    ("diagnostic_nested_return", ["inner=:3 : '+'", "outer=:{{inner y}}", "outer 0", "inner=:{{y+1}}", "outer 4"]),
    ("verb_direct", ["f=:{{ y+1 }}", "f 41", "f 1 2 3"]),
    ("verb_explicit", ["f=:3 : 'y+1'", "f 41"]),
    ("verb_dyad", ["f=:4 : 'x+y'", "2 f 3", "f 3"]),
    ("verb_sections", ["f=:3 : 0\ny+1\n:\nx+y\n)", "f 4", "2 f 3"]),
    ("verb_local_fallback", ["g=:10", "f=:3 : 'g=.g+y'", "f 2", "f 3", "g"]),
    ("verb_late_global", ["g=:10", "f=:3 : 'g+y'", "g=:20", "f 2"]),
    ("verb_late_function", ["op=:+", "f=:3 : 'op y'", "op=:-", "f 3"]),
    ("verb_snapshot", ["a=:i.4", "f=:3 : 0\nn=.y\ncopy=.n\nn=.n+10\ncopy\n)", "f a", "a"]),
    ("verb_effect", ["count=:0", "f=:3 : 0\ncount=:count+y\ncount\n)", "count", "f 2", "count"]),
    ("verb_frames", ["y=:99", "g=:3 : 'y'", "f=:3 : 'y+g y+1'", "f 2", "y"]),
    ("verb_private_caller", ["a=:10", "g=:3 : 'a+y'", "f=:3 : 0\na=.99\ng y\n)", "f 2", "a"]),
    ("verb_error_cleanup", ["y=:99", "f=:3 : 0\nt=.y\n1 2+1 2 3\n)", "f 2", "y", "t+0"]),
    ("verb_branch_return", ["f=:3 : 0\nif. y>0 do.\n42 return.\nelse.\n7 return.\nend.\n1 2+1 2 3\n)", "f 1", "f _1"]),
    ("verb_elseif", ["f=:3 : 0\nif. y=0 do.\n10\nelseif. y=1 do.\n20\nelse.\n30\nend.\n)", "f 0", "f 1", "f 2"]),
    ("verb_while", ["f=:3 : 0\nn=.y\nwhile. n>0 do.\nn=.n-1\nend.\nn\n)", "f 4", "f 0"]),
    ("verb_whilst", ["f=:3 : 0\nn=.y\nwhilst. n>0 do.\nn=.n-1\nend.\nn\n)", "f 4", "f 0"]),
    ("verb_break_continue", ["f=:3 : 0\nn=.0\ns=.0\nwhile. n<y do.\nn=.n+1\nif. n=2 do. continue. end.\nif. n=4 do. break. end.\ns=.s+n\nend.\ns\n)", "f 8"]),
    ("verb_first_atom_condition", ["f=:3 : 'if. y do. 42 else. 7 end.'", "f 0 1", "f 1 0", "f i.0", "f 'x'"]),
    ("verb_test_preserves_result", ["f=:3 : 0\n42\nif. y do. 7 end.\n)", "f 0", "f 1"]),
    ("verb_recursion", ["f=:3 : 0\nif. y=0 do. 1 return. end.\ny*f y-1\n)", "f 5", "f 3"]),
    ("verb_catch_error", ["f=:3 : 0\ntry.\n1 2+1 2 3\ncatch.\n42\nend.\n)", "f 0"]),
    ("verb_catch_skip", ["f=:3 : 'try. y+1 catch. 1 2+1 2 3 end.'", "f 4"]),
    ("verb_catch_condition", ["f=:3 : 'try. if. + do. 1 end. catch. 42 end.'", "f 0"]),
    ("verb_nested_catch", ["f=:3 : 'try. try. 1 2+1 2 3 catch. 1 2+1 2 3 end. catch. 42 end.'", "f 0"]),
    ("verb_catchd", ["f=:3 : 'try. 1 2+1 2 3 catchd. 42 end.'", "f 0"]),
    ("verb_catch_effect", ["g=:0", "f=:3 : 0\ntry.\ng=:y\n1 2+1 2 3\ncatch.\ng\nend.\n)", "f 7", "g"]),
    ("verb_catch_scope", ["f=:3 : 'try. y+1 catch. 42 end. 1 2+1 2 3'", "f 0"]),
    ("verb_empty_branch", ["f=:3 : 'if. y do. 42 end.'", "f 0"]),
    ("verb_empty_return", ["f=:3 : 'return. 42'", "f 0"]),
    ("verb_unbound_implicit_global_fallback", ["u=:10", "x=:10",
        "f=:3 : 'u+y'", "f 2", "f=:3 : 'x+y'", "f 2"]),
    ("frontend_e2e_demonstration", ["a=:1 2 3", "a+2*3", "g=:10",
        "explicit=:3 : 0\nt=.y+g\nt\n)", "direct=:{{ t=.y+g\nt }}",
        "explicit 2", "direct 2", "g=:20", "explicit 2", "direct 2",
        "pair=:4 : 'x+y'", "2 pair 3", "ddpair=:{{ x+y }}", "2 ddpair 3"]),
]

NESTED_FIXTURES = [
    ("nested_direct", ["outer=:{{\ninner=.{{y+1}}\ninner y\n}}", "outer 4", "inner 4"]),
    ("nested_controls", ["outer=:{{\ninner=.{{\nif. y>0 do. y+1 else. 0 end.\n}}\ninner y\n}}", "outer 4", "outer _2"]),
    ("nested_in_explicit", ["outer=:3 : 0\ninner=.{{\nif. y>0 do. y+1 else. 0 end.\n}}\ninner y\n)", "outer 4", "outer _2"]),
    ("nested_explicit_string", ["outer=:{{\ninner=.3 : 'if. y>0 do. y+1 else. 0 end.'\ninner y\n}}", "outer 4", "outer _2"]),
    ("nested_explicit_block", ["outer=:{{\ninner=.3 : 0\nif. y>0 do. y+1 else. 0 end.\n)\ninner y\n}}", "outer 4", "outer _2"]),
    ("nested_valence_separator", ["outer=:{{\ninner=.{{\ny+1\n:\nx+y\n}}\n2 inner y\n}}", "outer 4"]),
    ("nested_no_capture", ["g=:10", "outer=:{{\ng=.99\ninner=.{{t=.y+g\nt}}\ninner y\n}}", "outer 2", "g=:20", "outer 2", "t+0"]),
    ("nested_escape", ["g=:20", "outer=:{{\nprivate=.99\nescaped=:{{y+g}}\ny\n}}", "outer 0", "escaped 2", "private+0"]),
    ("nested_mode_isolation", ["u=:10", "outer=:{{\ninner=.{{u+y}}\ny\n}}", "outer 2"]),
    ("nested_anonymous", ["f=:{{ {{y+1}} y }}", "f 4"]),
    ("nested_deferred_control_error", ["f=:{{inner=.{{if. y do. y}}\ny}}", "f 1"]),
    ("nested_body_effect_timing", ["count=:0", "f=:{{inner=.{{count=:count+1\ny}}\ninner y\n}}", "count", "f 4", "count"]),
]

FOR_FIXTURES = [
    ("for_sum", ["f=:3 : 's=.0 for_i. i.y do. s=.s+i end. s'", "f 4", "f 0"]),
    ("for_direct", ["f=:{{ s=.0 for_i. i.y do. s=.s+i end. s }}", "f 4"]),
    ("for_scalar", ["f=:3 : 's=.0 for_i. y do. s=.s+i end. s'", "f 7"]),
    ("for_rows", ["f=:3 : 's=.0 0 0 for_i. y do. s=.s+i end. s'", "f i.2 3"]),
    ("for_zero_atoms", ["f=:3 : 's=.0 for_i. y do. s=.s+1 end. s'", "f 2 0$0", "f 0 3$0"]),
    ("for_unnamed", ["f=:3 : 's=.0 for. y do. s=.s+1 end. s'", "f i.4", "f 7", "f i.0"]),
    ("for_final_index", ["f=:3 : 'for_i. y do. end. i_index'", "f i.4", "f i.0"]),
    ("for_final_item", ["f=:3 : 'for_i. y do. end. i'", "f i.4", "f i.0"]),
    ("for_readonly", ["f=:3 : 'for_i. y do. i_index=.7 end.'", "f i.3"]),
    ("for_global_index_write", ["f=:3 : 'for_i. y do. i_index=:7 end.'", "f i.3"]),
    ("for_index_snapshot", ["f=:3 : 'a=.0 for_i. i.2 do. if. i_index=0 do. a=.i_index end. end. a'", "f 0"]),
    ("for_item_snapshot", ["f=:3 : 'a=.0 for_i. y do. if. i_index=0 do. a=.i end. end. a'", "f i.2 3"]),
    ("for_item_reassignment", ["f=:3 : 0\ns=.0\nfor_i. y do.\ns=.s+i\ni=.99\nend.\ns\n)", "f i.4"]),
    ("for_iterator_snapshot", ["f=:3 : 0\ns=.0\na=.y\nfor_i. a do.\na=.99\ns=.s+i\nend.\ns\n)", "f i.4"]),
    ("for_break", ["f=:3 : 'for_i. y do. if. i_index=1 do. break. end. end. i'", "f i.4", "f i.3 2"]),
    ("for_continue", ["f=:3 : 's=.0 for_i. y do. if. i=2 do. continue. end. s=.s+i end. s'", "f i.5"]),
    ("for_nested", ["f=:3 : 's=.0 for_i. i.2 do. for_j. i.3 do. s=.s+i+j end. end. s'", "f 0"]),
    ("for_catch_exit", ["f=:3 : 'try. for_i. y do. 1 2+1 2 3 end. catch. i_index=.7 end. i_index'", "f i.3"]),
    ("for_catch_inside", ["f=:3 : 's=.0 for_i. y do. try. i_index=.7 catch. s=.s+1 end. end. s'", "f i.3"]),
    ("for_reentrant_index", ["f=:3 : 's=.0 for_i. i.2 do. try. for_i. i.2 do. end. catch. s=.s+1 end. try. i_index=.9 catch. s=.s+1 end. end. s'", "f 0"]),
    ("for_repeat_after_break", ["f=:3 : 's=.0 for_i. y do. break. end. for_i. y do. s=.s+i end. s'", "f i.4"]),
    ("for_early_return", ["f=:3 : 'for_i. y do. i return. end.'", "f i.3 2", "f i.0"]),
    ("for_local_global", ["i=:99", "i_index=:88", "f=:3 : 'for_i. y do. end. i_index'", "f i.3", "i", "i_index"]),
]

ABANDON_FIXTURES = [
    ("global_noun", ["a=:7", "a_:", "a+0"]),
    ("right_to_left_success", ["a=:7", "a_:+a", "a+0"]),
    ("right_to_left_error", ["a=:7", "(a+a_:)0", "a+0"]),
    ("earlier_kernel_error", ["a=:7", "a_:+1 2+1 2 3", "a+0"]),
    ("missing", ["missing_:"]),
    ("array_alias", ["a=:i.6", "b=:a", "a_:", "a+0", "b"]),
    ("verb_value", ["f=:+", "g=:f_:", "f=:*", "g 3"]),
    ("adverb_value", ["adv=:/", "sum=:+adv_:", "sum 1 2 3", "adv 0"]),
    ("conjunction_value", ["conj=:@:", "h=:-conj_:+", "h 3", "conj 0"]),
    ("nameless_conjunction_transfer", ["c=:@:", "d=:c_:", "c=:&", "h=:-d+", "h 3"]),
    ("nameless_conjunction_group_transfer", ["c=:@:", "d=:(c_:)", "h=:-d+", "h 3", "c 0"]),
    ("nameless_conjunction_chain_transfer", ["c=:@:", "d=:e=:c_:", "h=:-d+", "h 3", "c 0"]),
    ("nameless_conjunction_group_inline", ["c=:@:", "h=:- (c_:) +", "h 3", "c 0"]),
    ("nameless_conjunction_nested_inline", ["c=:@:", "h=:- (d=:c_:) +", "h=:-d+", "h 3", "c 0"]),
    ("nameless_conjunction_local_inline", ["f=:{{c=.@:\nh=.-c_:+\nh y}}", "f 3"]),
    ("nameless_conjunction_local_transfer_direct", ["c=:7", "f=:{{c=.@:\nd=.c_:\nh=.-d+\nh y}}", "f 3", "c"]),
    ("nameless_conjunction_local_transfer_explicit", ["c=:7", "f=:3 : 0\nc=.@:\nd=.c_:\nh=.-d+\nh y\n)", "f 3", "c"]),
    ("nameless_conjunction_local_bare_direct", ["f=:{{c=.@:\ntry.\nc_:\ncatch.\nh=.-c+\nend.\nh y}}", "f 3"]),
    ("nameless_conjunction_local_bare_explicit", ["f=:3 : 0\nc=.@:\ntry.\nc_:\ncatch.\nh=.-c+\nend.\nh y\n)", "f 3"]),
    ("assignment_target", ["a_:=:9", "a+0"]),
    ("direct_local_bare", ["a=:7", "f=:{{a=.9\na_:\na}}", "f 0", "a+0"]),
    ("explicit_local_bare", ["a=:7", "f=:3 : 0\na=.9\na_:\na\n)", "f 0", "a+0"]),
    ("direct_local_delete", ["a=:7", "f=:{{a=.9\n(a_:)\na}}", "f 0", "a+0"]),
    ("explicit_local_delete", ["a=:7", "f=:3 : 0\na=.9\n(a_:)\na\n)", "f 0", "a+0"]),
    ("unbound_local_global", ["a=:7", "f=:3 : 0\na_:\na=.9\na\n)", "f 0", "a+0"]),
    ("local_assignment_target", ["a=:9", "f=:{{a_:=.3\na}}", "f 0", "a+0"]),
    ("readonly_bare", ["f=:{{for_i. i.1 do. i_index_: end.}}", "f 0"]),
    ("explicit_conjunction_inline", ["c=:2 : 'u@:v'", "h=:-c_:+", "h 3", "c 0"]),
    ("direct_conjunction_inline", ["c=:{{u@:v}}", "h=:-c_:+", "h 3", "c 0"]),
    ("explicit_conjunction_transfer", ["c=:2 : 'u@:v'", "saved=:c_:", "c=:2 : 'u@:u'", "h=:-saved+", "h 3"]),
    ("conjunction_local_direct", ["c=:7", "f=:{{c=.2 : 'u@:v'\nh=.-c_:+\nh y}}", "f 3", "c"]),
    ("conjunction_local_explicit", ["c=:7", "f=:3 : 0\nc=.2 : 'u@:v'\nh=.-c_:+\nh y\n)", "f 3", "c"]),
    ("conjunction_local_bare_caught", ["f=:{{c=.2 : 'u@:v'\ntry.\nc_:\ncatch.\nh=.-c+\nend.\nh y}}", "f 3"]),
    ("conjunction_global_fallback", ["c=:2 : 'u@:v'", "f=:{{h=.-c_:+\nc=.7\nh y}}", "f 3", "c 0"]),
    ("conjunction_failed_constructor", ["c=:2 : 'u+v'", "h=:1 2 c_:1 2 3", "c 0", "h 0"]),
    ("conjunction_caught_constructor", ["c=:2 : 'u+v'", "f=:{{try. h=.1 2 c_:1 2 3 catch. 99 end.}}", "f 0", "c 0"]),
    ("abandoned_cap_fork", ["cap=:[:", "f=:(cap_: + *)", "f 3", "cap 0"]),
]
# General-path deletion of a read-only loop index is excluded: a separate
# j.dll probe faulted inside the DLL. Rust explicitly rejects this boundary.

SCRIPT_SETUPS = {source for name, sources in SCOPE_FIXTURES
                 if name in {"explicit_local_function_escape", "explicit_local_global_collision"}
                 for source in sources if " : 0\n" in source}
SCRIPT_SETUPS.update(source for _, sources in DEFINITION_FIXTURES + FOR_FIXTURES + NESTED_FIXTURES + ASSIGNMENT_FIXTURES + ABANDON_FIXTURES + FIXTURES
                     for source in sources if " : 0\n" in source and not source.startswith("outer=:{{"))


def reference_trace(oracle, sources):
    outcomes, transports = [], []
    for source in sources:
        script = source in SCRIPT_SETUPS
        outcomes.append((oracle.run_script(source) or {"silent": True})
                        if script else oracle.eval(source))
        transports.append("script" if script else "sentence")
    return outcomes, transports


def compare_trace(sources, expected, actual):
    if len(expected) != len(sources) or len(actual) > len(sources):
        raise RuntimeError("Incomplete observation stream")
    differences = [{"index": i, "source": source, "reference": c, "rust": r}
                   for i, (source, c, r) in enumerate(zip(sources, expected, actual)) if c != r]
    if not differences and len(actual) == len(sources):
        return "matched", differences
    if not differences:
        return "observation_gap", differences
    # Later results after failed setup are secondary. Classify by the first
    # divergence, but preserve the complete trace for review.
    first = differences[0]
    if first["rust"] == {"error": "unsupported"}:
        return "unsupported_gap", differences
    return "semantic_mismatch", differences


def rust_trace(binary, route, sources):
    command = [str(binary), "--json"]
    if route == "semantic-reference":
        command.append("--semantic-reference")
    result = subprocess.run(command, input="\n".join(sources) + "\n", text=True,
                            encoding="utf-8", capture_output=True, timeout=30)
    if result.returncode not in (0, 1):
        raise RuntimeError((command, result.returncode, result.stderr))
    return [json.loads(line) for line in result.stdout.splitlines()]


PIN = "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528"


def reference_inputs(assets_root=None, reference_root=None, platform=None):
    """Select actual oracle files without certifying missing variants as passes."""
    platform = sys.platform if platform is None else platform
    if reference_root is None:
        if assets_root is None or platform != "win32":
            raise ValueError("Use --reference-root for Linux/WSL builds or native Windows --assets-root")
        assets = assets_root.resolve()
        libraries = [(name, assets / "target/cj-windows/j64" / name)
                     for name in ["j.dll", "javx2.dll"]]
        for _, library in libraries:
            if not library.is_file():
                raise ValueError(f"Missing reference library: {library}")
        return libraries, {"unrun_reference_variants": [],
                           "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
                           "revision_note": "Recorded asset revisions; DLL hashes identify the actual oracle, not a same-source rebuild"}
    if assets_root is not None or platform != "linux":
        raise ValueError("--reference-root requires Linux/WSL and cannot accompany --assets-root")
    root = reference_root.resolve()
    libraries, manifests = [], {}
    for variant in ["j64", "j64avx2"]:
        manifest = root / f"manifest-{variant}.json"
        library = root / "bin/linux" / variant / "libj.so"
        if not manifest.exists() and not library.exists():
            continue
        if not manifest.is_file() or not library.is_file():
            raise ValueError(f"Incomplete {variant} reference: require manifest and libj.so")
        metadata = json.loads(manifest.read_text(encoding="utf-8"))
        if (metadata.get("commit") != PIN or metadata.get("platform") != "linux"
                or metadata.get("variant") != variant):
            raise ValueError(f"Mismatched pinned reference manifest: {manifest}")
        libraries.append((variant, library))
        manifests[variant] = metadata
    if not libraries:
        raise ValueError("No built Linux reference libraries found")
    return libraries, {"reference_revision": PIN, "reference_manifests": manifests,
                       "unrun_reference_variants": [v for v in ["j64", "j64avx2"] if v not in manifests],
                       "revision_note": "Pinned build manifests select supplied Linux libraries; actual SHA256 identifies each binary. Absent variants are unrun, not passes."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    references = parser.add_mutually_exclusive_group(required=True)
    references.add_argument("--assets-root", type=Path)
    references.add_argument("--reference-root", type=Path,
                            help="Linux .reference directory produced by tools/build_reference.py")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--rust-revision", required=True)
    parser.add_argument("--report", type=Path, required=True)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--boundary-fixtures-only", action="store_true")
    selection.add_argument("--scope-fixtures-only", action="store_true")
    selection.add_argument("--definition-fixtures-only", action="store_true")
    selection.add_argument("--for-fixtures-only", action="store_true")
    selection.add_argument("--nested-fixtures-only", action="store_true")
    selection.add_argument("--assignment-fixtures-only", action="store_true")
    selection.add_argument("--abandon-fixtures-only", action="store_true")
    args = parser.parse_args()
    try:
        libraries, reference_metadata = reference_inputs(args.assets_root, args.reference_root)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    fixtures = (ABANDON_FIXTURES if args.abandon_fixtures_only else
                ASSIGNMENT_FIXTURES if args.assignment_fixtures_only else
                NESTED_FIXTURES if args.nested_fixtures_only else
                FOR_FIXTURES if args.for_fixtures_only else
                DEFINITION_FIXTURES if args.definition_fixtures_only else
                BOUNDARY_FIXTURES if args.boundary_fixtures_only else
                SCOPE_FIXTURES if args.scope_fixtures_only else FIXTURES)
    records = []
    for variant, library in libraries:
        os.environ["J_LIBRARY"] = str(library)
        for name, sources in fixtures:
            oracle = Oracle()
            try:
                reference, transports = reference_trace(oracle, sources)
            finally:
                oracle.close()
            for route in ["direct", "semantic-reference"]:
                actual = rust_trace(args.binary, route, sources)
                status, differences = compare_trace(sources, reference, actual)
                records.append({"case": name, "variant": variant, "route": route,
                                "status": status, "sources": sources, "reference": reference,
                                "reference_transport": transports,
                                "rust": actual, "differences": differences,
                                "unobserved_suffix": sources[len(actual):]})
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    counts = dict(Counter(record["status"] for record in records))
    report = {
        "scope": "Bounded observable sentence/value/error audit; not full NAME, event, Graph/A3 or compiled-route conformance",
        "platform": sys.platform, "rust_revision": args.rust_revision,
        "rust_binary_sha256": sha(args.binary),
        "rust_source_sha256": {path.as_posix(): sha(path) for path in
                               sorted(Path("src").rglob("*.rs")) +
                               [Path("Cargo.toml"), Path("Cargo.lock"),
                                Path("tools/name_compatibility_audit.py"), Path("tools/oracle.py")]},
        "source_revision": "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
        **reference_metadata,
        "reference_sha256": {name: sha(library) for name, library in libraries},
        "fixture_set": ("name-abandon" if args.abandon_fixtures_only else
                        "string-assignment" if args.assignment_fixtures_only else
                        "definition-nested" if args.nested_fixtures_only else
                        "definition-for-loops" if args.for_fixtures_only else
                        "definition-calls" if args.definition_fixtures_only else
                        "semantic-boundaries" if args.boundary_fixtures_only else
                        "name-scopes" if args.scope_fixtures_only else "names"),
        "fixtures": len(fixtures), "observations": len(records), "counts": counts, "records": records,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"fixtures": len(fixtures), "observations": len(records), "counts": counts}))
    # Completing an audit is not a conformance pass. Counts always expose gaps.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

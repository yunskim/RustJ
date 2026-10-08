"""Bounded frontend admission/handoff/error review, not a full conformance gate.

F/H/P/G/L are independent read-only inspections (frontend / handoff / binding / Graph / Logical). R is actual captured execution.
Only R and post-error state are compared with C; a stage rejection is not a J error.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys

from oracle import Oracle

CASES = [
    ("array_expression", ["a=:1 2 3"], "a+2*3", []),
    ("late_verb", ["f=:+", "g=:f", "f=:*"], "g 3", []),
    ("computed_rank", [], '+/"(1+0) i.2 3', []),
    ("nonfinal_assignment", ["a=:1"], "a+a=:2", ["a"]),
    ("chained_assignment", [], "a=:b=:1", ["a", "b"]),
    ("computed_target", [], "'a'=:7", ["a"]),
    ("multiple_target", [], "'a b'=:3 4", ["a", "b"]),
    ("direct_locative", [], "a_base_=:7", ["a"]),
    ("indirect_locative", ["loc=:<'base'", "a=:7"], "a__loc", []),
    ("abandon_name", ["a=:7"], "a_:", ["a"]),
    ("abandon_verb_transfer", ["f=:+"], "g=:f_:", ["f"]),
    ("abandon_modifier_transfer", ["adv=:/"], "saved=:adv_:", ["adv"]),
    ("abandon_explicit_conjunction", ["c=:2 : 'u@:v'"], "h=:-c_:+", ["c"]),
    ("complex_literal", [], "1j2", []),
    ("extended_literal", [], "123x", []),
    ("rational_literal", [], "2r3", []),
    ("overflow_literal", [], "9223372036854775808", []),
    ("core_conjunction", [], "f=:+&2", []),
    ("modifier_value", [], "adv=:/", []),
    ("execute", [], '\". \'1+2\'', []),
    ("direct_definition", [], "f=:{{y+1}}", []),
    ("explicit_definition", [], "f=:3 : 'y+1'", []),
    ("nested_definition", [], "f=:{{\ninner=.{{y+1}}\ninner y\n}}", []),
    ("select_preparse", [], "f=:3 : 'select. y case. 1 do. 10 case. do. 20 end.'", []),
    ("syntax_failure", [], "1+)", []),
    ("domain_failure", [], "'a'+1", []),
    ("first_error", [], "missing+(1 2+1 2 3)", []),
    ("failed_assignment", ["a=:7"], "a=:1 2+1 2 3", ["a"]),
    ("effect_before_error", ["a=:1"], "(1 2+1 2 3)+a=:9", ["a"]),
    ("definition_error_location", ["f=:{{y+1 2 3}}"], "f 1 2", []),
    ("nested_error_location", ["inner=:{{y+1 2 3}}", "outer=:{{inner y}}"], "outer 1 2", []),
    ("caught_error_effect", ["count=:0", "f=:{{\ntry.\ncount=:count+1\n1 2+1 2 3\ncatch.\ncount\nend.\n}}"], "f 0", ["count"]),
    ("unsupported_not_caught", ["f=:{{try. +&2 y catch. 42 end.}}"], "f 3", []),
    ("invalid_control_redefinition", ["f=:42"], "f=:{{if. y do. y}}", ["f"]),
    ("definition_return_explicit", ["f=:3 : '+'"], "f 0", []),
    ("definition_return_direct", ["f=:{{+}}"], "f 0", []),
    ("definition_admission_valence", ["f=:4 : 'x+y'"], "f 0", []),
    ("definition_return_post_effect", ["count=:0", "saved=:99", "f=:3 : 'count=:count+1\ntry. local=.+ catch. 42 end.'"], "saved=:f 0", ["count", "saved"]),
]

NUMERIC_OVERFLOW_CASES = [
    ("int_max", [], "9223372036854775807", []),
    ("int_min", [], "_9223372036854775808", []),
    ("int_exact_above_float_precision", [], "9007199254740993", []),
    ("positive_overflow", [], "9223372036854775808", []),
    ("negative_overflow", [], "_9223372036854775809", []),
    ("leading_zero_overflow", [], "000000009223372036854775808", []),
    ("word_overflow_last", [], "9007199254740993 1 9223372036854775808", []),
    ("word_overflow_first", [], "9223372036854775808 9007199254740993 1", []),
    ("word_overflow_middle", [], "1 _9223372036854775809 9007199254740993", []),
    ("word_no_overflow", [], "9007199254740993 9223372036854775807", []),
    ("huge_positive", [], "9" * 400, []),
    ("huge_negative", [], "_" + "9" * 400, []),
    ("overflow_expression", [], "1+9223372036854775808", []),
    ("overflow_assignment", [], "saved=:9223372036854775808", ["saved"]),
    ("malformed_after_overflow", ["saved=:7"], "saved=:9223372036854775808 1_2", ["saved"]),
    ("malformed_before_overflow", ["saved=:7"], "saved=:1_2 9223372036854775808", ["saved"]),
    ("direct_overflow_body", ["f=:{{9223372036854775808}}"], "f 0", []),
    ("explicit_overflow_body", ["f=:3 : '9223372036854775808'"], "f 0", []),
]

INTEGER_DTYPE_CASES = [
    ("bool_zero", [], "0", []),
    ("bool_one", [], "1", []),
    ("bool_negative_zero_scalar", [], "_0", []),
    ("int_double_zero", [], "00", []),
    ("int_leading_zero_one", [], "01", []),
    ("int_triple_zero", [], "000", []),
    ("int_many_leading_zeros", [], "0001", []),
    ("int_negative_double_zero", [], "_00", []),
    ("int_negative_leading_zero_one", [], "_01", []),
    ("bool_word", [], "0 1", []),
    ("int_signed_atom_last", [], "0 _0", []),
    ("int_signed_atom_first", [], "_0 1", []),
    ("int_double_zero_word", [], "00 1", []),
    ("int_leading_zero_word", [], "0 01", []),
    ("int_assignment", [], "saved=:01", ["saved"]),
    ("int_direct_body", ["f=:{{01}}"], "f 0", []),
    ("int_explicit_body", ["f=:3 : '_00'"], "f 0", []),
    ("int_boxed_payload", [], "<01", []),
]

SCIENTIFIC_CASES = [
    (name, [], source, []) for name, source in [
        ("one", "1e0"), ("zero", "0e0"), ("negative_zero", "_0e0"),
        ("uppercase", "1E0"), ("uppercase_zero", "0E0"), ("uppercase_signed_zero", "_0E0"),
        ("fraction", "1e_1"), ("exact_scaled", "10e_1"),
        ("near_integer", "100000000000001e_14"),
        ("underflow", "1e_9999"), ("negative_underflow", "_1e_9999"),
        ("dot_scalar", "1.0e0"), ("dot_word", "1e0 1.0"),
        ("uppercase_word", "1E0 1"), ("mixed_exponents", "1E0 1e0"),
        ("mixed_signed_zero", "1e0 _0E0"), ("integer_word", "1e0 0 1"),
        ("precision_first", "9007199254740993 1e0"), ("precision_last", "1e0 9007199254740993"),
        ("min_inclusive", "_9223372036854775808e0"),
        ("rounded_min", "_9223372036854775809e0"),
        ("max_rounded_out", "9223372036854775807e0"),
        ("max_exclusive", "9223372036854775808e0"),
        ("largest_inside", "9223372036854774784e0"),
        ("below_min", "_9223372036854777856e0"),
        ("mixed_rounded_min", "_9223372036854775809 1e0"),
        ("integer_overflow_control", "_9223372036854775809 1"),
        ("infinity", "1e9999"), ("nan_word", "_. 1e0"),
        ("positive_infinity_word", "1e0 _"), ("negative_infinity_word", "__ 1e0"),
        ("fraction_first", "1e_1 1e0"), ("fraction_last", "1e0 1e_1"),
        ("expression", "1+1e0"), ("boxed", "<1e0"),
    ]
] + [
    ("assignment", [], "saved=:1e0", ["saved"]),
    ("malformed_exponent", ["saved=:7"], "saved=:1e0 1e_", ["saved"]),
    ("invalid_exact_mix", ["saved=:7"], "saved=:1e0 1x", ["saved"]),
    ("direct_body", ["f=:{{1e0}}"], "f 0", []),
    ("explicit_body", ["f=:3 : '1e0'"], "f 0", []),
]

# Exercise C independently across magnitude, exponent case and whole-word
# masks. Expected dtypes/values come from the oracle, not Rust's conversion.
for mantissa_index, mantissa in enumerate([
        "0", "1", "_1", "10", "9007199254740993", "9223372036854775807", "_9223372036854775809"]):
    for exponent_index, exponent in enumerate(["0", "1", "_1", "18", "_18", "309", "_400"]):
        for marker in ["e", "E"]:
            source = mantissa + marker + exponent
            for suffix_name, suffix in [("scalar", ""), ("real_word", " 1e0"), ("dot_word", " 1.0")]:
                SCIENTIFIC_CASES.append((f"grid_{mantissa_index}_{exponent_index}_{marker}_{suffix_name}",
                                         [], source + suffix, []))


RATIO_CASES = [
    ("ratio_assignment", [], "saved=:1r2.0", ["saved"]),
    ("ratio_direct_local", ["f=:{{local=.1r2.0\nlocal+y}}"], "f 0", []),
    ("ratio_explicit_local", ["f=:3 : 'local=.2r1 1e0\nlocal+y'"], "f 0", []),
    ("ratio_failed_assignment", ["saved=:7"], "saved=:1r2.0 1e_", ["saved"]),
    ("ratio_uppercase_mode", [], "1r2 1E0", []),
    ("ratio_bad_denominator", [], "1r_ 1.0", []),
    ("ratio_extra_separator", [], "1r2r3 1.0", []),
    ("ratio_bad_extended", [], "1r2.0 1x", []),
]
for numerator_index, numerator in enumerate(["0", "_0", "1", "_1", "2", "9007199254740993", "9223372036854775808", "1e_9999", "1e9999"]):
    for denominator_index, denominator in enumerate(["0", "_0", "1", "_2", "3", "1e_9999", "1e9999"]):
        for suffix_name, suffix in [("dot", " 1.0"), ("scientific", " 1e0")]:
            RATIO_CASES.append((f"ratio_{numerator_index}_{denominator_index}_{suffix_name}", [], numerator + "r" + denominator + suffix, []))
for index, source in enumerate(["1r2.0", "1.r2", "1r2.", "0r0.0", "_0r0.0", "0r_0.0", "_0r_0.0", "1r0.0", "1r_0.0", "_1r0.0", "_1r_0.0", "1e9999r1e9999", "2e0r1", "2r1 1E0 1e0"]):
    RATIO_CASES.append((f"ratio_scalar_{index}", [], source, []))


EXTENDED_CASES = [
    ("extended_assignment", [], "saved=:9007199254740993x", ["saved"]),
    ("extended_direct_local", ["saved=:9007199254740993x", "f=:{{local=.saved\nlocal+1}}"], "f 0", ["saved"]),
    ("extended_explicit_local", ["saved=:9007199254740993x", "f=:3 : 'local=.saved\nlocal+1'"], "f 0", ["saved"]),
    ("extended_alias", ["saved=:9007199254740993x", "alias=:saved", "saved=:2x"], "alias", ["saved", "alias"]),
    ("extended_failed_assignment", ["saved=:9007199254740993x"], "saved=:1x 2xx", ["saved"]),
    ("extended_float_literal_invalid", ["saved=:7x"], "saved=:1x 1.0", ["saved"]),
    ("extended_uppercase_invalid", [], "1x 1E0", []),
]
for index, source in enumerate(["0x", "1x", "01x", "_0x", "_01x", "9223372036854775808x", "_9223372036854775809x", "9"*400 + "x"]):
    for context, expression in [("scalar",source),("first",source+" 9007199254740993"),("last","9007199254740993 "+source)]:
        EXTENDED_CASES.append((f"extended_{index}_{context}", [], expression, []))
for index, source in enumerate(["9007199254740993x+1", "1+9007199254740993x", "9223372036854775808x*2", "2*9223372036854775808x", "0-9223372036854775808x", "9007199254740993x-9007199254740992x", "9007199254740993x=9007199254740992x", "9007199254740993x>9007199254740992x", "9007199254740992x<9007199254740993x", "1 2+9007199254740993x", "- _9223372036854775809x", "|_12345678901234567890x", "*_12345678901234567890x", "+1x", "$1x 2x", "#1x 2x", "$1x", ",1x", "|.1x 2x", "|:2 2$1x 2x 3x 4x", "1{1x 9007199254740993x", "4{.1x 2x", "_4{.1x 2x", "1}.1x 2x", "1|.1x 2x 3x", "0$1x", "2 0$1x", "1x+0$2x", "> <1x", "<1x 2x", "1 2x+1 2 3x", "5{1x 2x"]):
    EXTENDED_CASES.append((f"extended_operation_{index}", [], source, []))


RATIONAL_CASES = [
    ("rational_assignment", [], "saved=:9007199254740993r2", ["saved"]),
    ("rational_alias", ["saved=:9007199254740993r2", "alias=:saved", "saved=:1r0"], "alias", ["saved", "alias"]),
    ("rational_direct_local", ["saved=:2r3", "f=:{{local=.saved\nlocal}}"], "f 0", ["saved"]),
    ("rational_explicit_local", ["saved=:2r3", "f=:3 : 'local=.saved\nlocal'"], "f 0", ["saved"]),
    ("rational_failed_assignment", ["saved=:2r3"], "saved=:1r2 2rr3", ["saved"]),
    ("rational_uppercase_invalid", ["saved=:2r3"], "saved=:1r2 1E0", ["saved"]),
]
RATIONAL_REFERENCE_EXCLUSIONS = []
for n_index, numerator in enumerate(['0','_0','1','_1','2','_6','9007199254740993','9'*200]):
    for d_index, denominator in enumerate(['0','_0','1','_1','2','_4','_','__']):
        for context, source in [('scalar',numerator+'r'+denominator),('first',numerator+'r'+denominator+' 9007199254740993 1x'),('last','9007199254740993 1x '+numerator+'r'+denominator)]:
            if n_index == 7 and denominator in {'0', '_0'}:
                # Pinned j.dll traps in vq.c on this large-numerator infinity.
                # Exclude from both variants, record separately, never as a pass.
                RATIONAL_REFERENCE_EXCLUSIONS.append(source)
                continue
            RATIONAL_CASES.append((f'rational_{n_index}_{d_index}_{context}',[],source,[]))
for index, source in enumerate(['_r','__r','_r0','__r0','_r_0','__r_0','_r_3','__r_3','1x _','1x __','2r3 1','2r3 0 1','_ 2r3','__ 2r3','+2r3', '$2r3', '#2r3', ',2r3', '|.2r3 3r4', '|:2 2$2r3 3r4', '1{2r3 3r4', '4{.2r3 3r4', '_4{.2r3 3r4', '1}.2r3 3r4', '1|.2r3 3r4', '0$2r3', '2 0$2r3', '> <2r3', '<2r3 3r4', '5{2r3 3r4', '2r3x', '_r_', '2rr3']):
    RATIONAL_CASES.append((f'rational_special_{index}',[],source,[]))

# N3c: Cartesian non-finite arithmetic and exact comparisons. Outcomes come
# from both C engines; independent Rust expectations separately cover precision.
for a_index, a in enumerate(['0r1','1r2','_1r2','1r0','_1r0','9007199254740993r2','2r3']):
    for b_index, b in enumerate(['0r1','1r2','_1r2','1r0','_1r0','9007199254740993r2','2r3']):
        for op_index, op in enumerate(['+','-','*','%','=','<','>']):
            RATIONAL_CASES.append((f'qop_{a_index}_{b_index}_{op_index}',[],a+op+b,[]))
    for op_index, op in enumerate(['-','|','*','%']):
        RATIONAL_CASES.append((f'qunary_{a_index}_{op_index}',[],op+a,[]))
for n_index, n in enumerate(['0','2','9007199254740993x']):
    for op_index, op in enumerate(['+','-','*','%','=','<','>']):
        for side, source in [('left', n+op+'2r3'),('right','2r3'+op+n)]:
            RATIONAL_CASES.append((f'qmix_{n_index}_{op_index}_{side}',[],source,[]))
RATIONAL_CASES += [
    ('qop_prefix',[], '1 2+2 2$1r2 2r3 3r4 4r5', []),
    ('qop_empty',[], '1r2+0$1', []),
    ('qdiv_empty',[], '(0$1r2)%0r1', []),
    ('qeq_empty',[], '(0$1r2)=1r2', []),
    ('qop_length',[], '1r2 2r3+1 2 3', []),
    ('qop_large',[], '('+'9'*200+'r7%'+'9'*200+'r3)*7r3', []),
    ('qop_direct',['saved=:9007199254740993r2','f=:{{local=.saved\nlocal+1r3}}'],'f 0',['saved']),
    ('qop_explicit',['saved=:9007199254740993r2',"f=:3 : 'local=.saved\nlocal+1r3'"],'f 0',['saved']),
    ('qop_alias',['saved=:2r3','alias=:saved'],'saved=:saved+1r2',['saved','alias']),
    ('qop_failed_assignment',['saved=:2r3'],'saved=:1r0-1r0',['saved']),
    ('qop_catch',['saved=:2r3','f=:{{try. saved=:1r0-1r0 catch. saved end.}}'],'f 0',['saved']),
]

# N3d: ExtendedInt division chooses one result family for the whole noun.
for a_index, a in enumerate(['0x','1x','_1x','4x','_6x','9007199254740993x','9'*200+'x']):
    for b_index, b in enumerate(['0x','1x','_1x','2x','_2x','3x','9007199254740993x']):
        for context, source in [('scalar',a+'%'+b),('first','('+a+' 4x)%'+b),('last','(4x '+a+')%'+b)]:
            RATIONAL_CASES.append((f'xdiv_{a_index}_{b_index}_{context}',[],source,[]))
    RATIONAL_CASES.append((f'xreciprocal_{a_index}',[],'%'+a,[]))
for index, source in enumerate(['4x%2','4%2x','1x%2','1%2x','1x%0','_1x%0','0%0x','1x%0$2x','%(0$2x)','%1x _1x','%1x _2x 0x','(2 2$4x 6x 1x 3x)%2 3','1x%(2 0$2)','4x 1x 6x%2','0x 2x 0x%0 0 0']):
    RATIONAL_CASES.append((f'xdiv_special_{index}',[],source,[]))
RATIONAL_CASES += [
    ('xdiv_direct',['saved=:4x 1x 6x','f=:{{local=.saved\nlocal%2}}'],'f 0',['saved']),
    ('xdiv_explicit',['saved=:4x 1x 6x',"f=:3 : 'local=.saved\nlocal%2'"],'f 0',['saved']),
    ('xdiv_alias',['saved=:4x 1x 6x','alias=:saved'],'saved=:saved%2',['saved','alias']),
    ('xdiv_failed',['saved=:4x 1x 6x'],'saved=:saved%1 2',['saved']),
]

# N3e: right-fold semantics and zero-item vs zero-atom primitive dispatch.
for a_index, a in enumerate(['0r1','1r2','_2r3','1r0','_1r0']):
    for b_index, b in enumerate(['0r1','1r2','_2r3','1r0','_1r0']):
        for c_index, c in enumerate(['0r1','1r2','_2r3','1r0','_1r0']):
            for op_index, op in enumerate(['+','-','*','%']):
                RATIONAL_CASES.append((f'qreduce_{a_index}_{b_index}_{c_index}_{op_index}',[],op+'/'+a+' '+b+' '+c,[]))
for op_index, op in enumerate(['+','-','*','%']):
    for shape_index, shape in enumerate(['0','0 2','0 0','0 2 3','1 0','2 0','3 2 0','4 0 2','1 2','3 2 2']):
        RATIONAL_CASES.append((f'qreduce_shape_{op_index}_{shape_index}',[],op+'/('+shape+'$2r3)',[]))
    for case_index, source in enumerate(['2r3','(,2r3)','1r2 2r3 3r4','10r1 3r1 2r1 1r1','9007199254740993r2 _9007199254740991r2','2 2$1r2 2r3 3r4 4r5','9'*200+'r3 _'+'9'*200+'r3']):
        RATIONAL_CASES.append((f'qreduce_special_{op_index}_{case_index}',[],op+'/'+source,[]))
RATIONAL_CASES += [
    ('qreduce_direct',['saved=:1r2 2r3 3r4','f=:{{local=.saved\n-/local}}'],'f 0',['saved']),
    ('qreduce_explicit',['saved=:1r2 2r3 3r4',"f=:3 : 'local=.saved\n-/local'"],'f 0',['saved']),
    ('qreduce_alias',['saved=:1r2 2r3 3r4','alias=:saved'],'saved=:+/saved',['saved','alias']),
    ('qreduce_failed',['saved=:2r3'],'saved=:+/1r0 _1r0',['saved']),
    ('qreduce_catch',['saved=:2r3','f=:{{try. saved=:+/1r0 _1r0 catch. saved end.}}'],'f 0',['saved']),
]

# N3f: cell assembly promotion order, exact payload, and typed empty frames.
for op_index, op in enumerate(['-', '|', '*', '%', '+']):
    for shape_index, shape in enumerate(['3', '2 3', '0', '0 2', '2 0']):
        RATIONAL_CASES.append((f'qrank_unary_{op_index}_{shape_index}',[],f'({op}"0)({shape}$1r2 _2r3 0r1)',[]))
for op_index, op in enumerate(['+', '-', '*', '%']):
    for shape_index, shape in enumerate(['2 3', '0 3', '0 1', '0 0', '2 0', '2 2 3']):
        RATIONAL_CASES.append((f'qrank_reduce_{op_index}_{shape_index}',[],f'({op}/"1)({shape}$1r2 2r3 3r4)',[]))
for i, source in enumerate(['(%"0)1x 2x 0x','(%"0)0x 2x 1x','(%"0)1x 1x 1x','(1x 2x 3x)(%"0)1x 4x 3x','(1r2 2r3 3r4)(+"0)1 2 3','(1 2 3)(+"0)1r2 2r3 3r4']):
    RATIONAL_CASES.append((f'qrank_mix_{i}',[],source,[]))

RATIONAL_CASES += [
    ('qrank_direct',['saved=:1r2 2r3','f=:{{local=.saved\n(-"0)local}}'],'f 0',['saved']),
    ('qrank_explicit',['saved=:1r2 2r3',"f=:3 : 'local=.saved\n(-\"0)local'"],'f 0',['saved']),
    ('qrank_alias',['saved=:1r2 2r3','alias=:saved'],'saved=:(-"0)saved',['saved','alias']),
    ('qrank_failed',['saved=:2r3'],'saved=:(1r0 1r2)(+"0)_1r0 2r3',['saved']),
]

for i, source in enumerate(['(0$1r2)(+"0)2r3','2r3(+"0)0$1r2','(0 2$1r2)(+"1)2r3 3r4','(2 0$1r2)(+"1)2 0$2r3','(%"0)'+('9'*200)+'x 2x 1x','(|"0)'+('9'*200)+'r7 1r2']):
    RATIONAL_CASES.append((f'qrank_fill_precision_{i}',[],source,[]))

for i, atom in enumerate(['1','01','123456789012345678901234567891x']):
    for j, order in enumerate(['0 1','1 0','0 0 1','1 0 0']):
        RATIONAL_CASES.append((f'qrank_definition_mix_{i}_{j}',[],f'({{{{if. y=0 do. {atom} else. 1r2 end.}}}}"0){order}',[]))


def probe(binary, setup, source, after):
    operations = [("E", s) for s in setup]
    operations += [(stage, source) for stage in ("F", "H", "P", "G", "L", "R")]
    operations += [("E", s) for s in after]
    data = "".join(f"{stage} {s.encode().hex()}\n" for stage, s in operations)
    result = subprocess.run([str(binary)], input=data, text=True, encoding="utf-8",
                            capture_output=True, timeout=30, check=True)
    records = [json.loads(line) for line in result.stdout.splitlines()]
    if len(records) != len(operations):
        raise RuntimeError("Incomplete audit stream")
    return records[:len(setup)], dict(zip(("F", "H", "P", "G", "L", "R"), records[len(setup):len(setup)+6])), records[len(setup)+6:]


def semantic_outcome(outcome):
    return {"error": outcome["error"]} if "error" in outcome else outcome


def same_outcome(actual, expected):
    """Typed result comparison including observable IEEE zero signs."""
    if isinstance(actual, dict) and isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(same_outcome(actual[k], expected[k]) for k in actual)
    if isinstance(actual, list) and isinstance(expected, list):
        return len(actual) == len(expected) and all(same_outcome(a, b) for a, b in zip(actual, expected))
    if isinstance(actual, (int, float)) and isinstance(expected, (int, float)) and actual == expected == 0:
        return math.copysign(1, actual) == math.copysign(1, expected)
    return actual == expected


INSPECTION_STAGES = {"F": "frontend", "H": "frontend-handoff", "P": "semantic-binding", "G": "j-graph", "L": "logical"}


def inspection_outcome(stage, value):
    if value.get("inspection_stage") != INSPECTION_STAGES[stage] or value.get("execution_performed") is not False:
        raise ValueError("Inspection stage must not claim execution")
    if "error" not in value:
        if value.get("ok") is not True:
            raise ValueError("Missing representation admission result")
        return "accepted-representation"
    if value.get("j_handler_eligible") is not False:
        raise ValueError("Read-only inspection cannot enter a J handler")
    category = value.get("category")
    if category not in {"j-language", "unsupported-capability", "verifier-defect", "backend-failure"}:
        raise ValueError("Unknown failure category")
    return category


def enforce_acceptance(report, strict_runtime=False):
    if any(category in {"verifier-defect", "backend-failure"}
           for counts in report["stage_admission_counts"].values() for category in counts):
        raise SystemExit("Internal stage failure must not be classified as a runtime gap")
    if strict_runtime and report["counts"].get("runtime_gap", 0):
        raise SystemExit("Strict numeric conformance difference")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--assets-root", type=Path, required=True)
    p.add_argument("--probe", type=Path, required=True)
    p.add_argument("--report", type=Path, required=True)
    selection = p.add_mutually_exclusive_group()
    selection.add_argument("--numeric-overflow-fixtures-only", action="store_true",
                   help="Strict bounded decimal-overflow corpus: any runtime difference fails")
    selection.add_argument("--integer-dtype-fixtures-only", action="store_true",
                           help="Strict bounded integer-spelling dtype corpus")
    selection.add_argument("--scientific-fixtures-only", action="store_true",
                           help="Strict bounded scientific real narrowing corpus")
    selection.add_argument("--ratio-fixtures-only", action="store_true", help="Strict decimal real-family ratio corpus")
    selection.add_argument("--extended-fixtures-only", action="store_true", help="Strict finite decimal extended integer corpus")
    selection.add_argument("--rational-fixtures-only", action="store_true", help="Strict exact rational literal and structural corpus")
    args = p.parse_args()
    if sys.platform != "win32":
        p.error("Native Windows audit only")
    rust = {}
    selected = (NUMERIC_OVERFLOW_CASES if args.numeric_overflow_fixtures_only else
                INTEGER_DTYPE_CASES if args.integer_dtype_fixtures_only else
                SCIENTIFIC_CASES if args.scientific_fixtures_only else
                RATIO_CASES if args.ratio_fixtures_only else
                EXTENDED_CASES if args.extended_fixtures_only else
                RATIONAL_CASES if args.rational_fixtures_only else CASES)
    cases = [(name, setup, source, [s if args.rational_fixtures_only else s + "+0" for s in after]) for name, setup, source, after in selected]
    for name, setup, source, after in cases:
        setup_rust, stages, post = probe(args.probe, setup, source, after)
        for value in stages.values():
            for key in ("context_hex", "render_hex", "names_hex", "capture_hex", "requirements_hex"):
                if key in value:
                    value[key.removesuffix("_hex")] = bytes.fromhex(value.pop(key)).decode()
            result = value.get("result", {})
            for key in ("context_hex", "render_hex"):
                if key in result:
                    result[key.removesuffix("_hex")] = bytes.fromhex(result.pop(key)).decode()
        for stage in INSPECTION_STAGES:
            inspection_outcome(stage, stages[stage])
        rust[name] = (setup_rust, stages, post)
    comparisons = []
    for variant in ("j.dll", "javx2.dll"):
        os.environ["J_LIBRARY"] = str(args.assets_root.resolve() / "target/cj-windows/j64" / variant)
        for name, setup, source, after in cases:
            oracle = Oracle()
            try:
                setup_results = [oracle.eval(s) for s in setup]
                if name == "complex_literal":
                    # The noun bridge does not serialize these families. Record
                    # C acceptance/type explicitly, never invent a value match.
                    reference = oracle.run("contractvalue=: " + source)
                    if reference is None:
                        reference = {"accepted_noun_type": oracle.eval("3!:0 contractvalue")["data"][0]}
                else:
                    reference = oracle.eval(source)
                post = [oracle.eval(s) for s in after]
            finally:
                oracle.close()
            setup_rust, stages, rust_post = rust[name]
            actual = stages["R"].get("result", stages["R"])
            matches = (same_outcome([semantic_outcome(x) for x in setup_rust], setup_results) and
                       same_outcome(semantic_outcome(actual), reference) and
                       same_outcome([semantic_outcome(x) for x in rust_post], post))
            comparisons.append({"case": name, "variant": variant, "source": source,
                                "setup": setup, "setup_reference": setup_results, "setup_rust": setup_rust,
                                "reference": reference, "stages": stages,
                                "post_sources": after, "post_reference": post, "post_rust": rust_post,
                                "status": "matched" if matches else "runtime_gap"})
    report = {"scope": "Bounded admission, handoff and error audit; gaps are findings, not passes. C diagnostic locations are not compared.",
              "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "rust_source_sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in sorted(Path("src").rglob("*.rs"))},
              "probe_sha256": hashlib.sha256(args.probe.read_bytes()).hexdigest(),
              "audit_source_sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                      for p in [Path(__file__), Path("tools/oracle.py"), Path("examples/frontend_contract_probe.rs")]},
              "reference_sha256": {name: hashlib.sha256((args.assets_root / "target/cj-windows/j64" / name).read_bytes()).hexdigest()
                                   for name in ("j.dll", "javx2.dll")},
              "source_revision": "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
              "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
              "fixture_set": ("numeric-overflow" if args.numeric_overflow_fixtures_only else
                              "integer-dtype" if args.integer_dtype_fixtures_only else
                              "scientific" if args.scientific_fixtures_only else
                              "real-ratio" if args.ratio_fixtures_only else
                              "extended-integer" if args.extended_fixtures_only else
                              "rational" if args.rational_fixtures_only else "frontend-boundaries"),
              "reference_exclusions": ({"sources": RATIONAL_REFERENCE_EXCLUSIONS, "reason": "Pinned j.dll traps in vq.c for a 200-digit numerator and zero denominator; excluded from both DLL comparison sets, not passes."} if args.rational_fixtures_only else {}),
              "cases": len(selected), "observations": len(comparisons),
              "counts": dict(Counter(r["status"] for r in comparisons)),
              "stage_admission_counts": {stage: dict(Counter(inspection_outcome(stage, values[1][stage]) for values in rust.values())) for stage in INSPECTION_STAGES},
              "records": comparisons}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ("cases", "observations", "counts", "stage_admission_counts")}))
    enforce_acceptance(report, args.numeric_overflow_fixtures_only or args.integer_dtype_fixtures_only
                       or args.scientific_fixtures_only or args.ratio_fixtures_only or args.extended_fixtures_only or args.rational_fixtures_only)


if __name__ == "__main__":
    main()

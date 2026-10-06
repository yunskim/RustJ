#!/usr/bin/env python3
"""Deterministic differential checks of the supported M1/M2 subset."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import random
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def modifier_trident_cases():
    """All cf.c derived trident POS productions with core operand samples.

    These are observable constructor/error comparisons, not ptcol trace proof.
    """
    spellings = {'N': '3', 'V': '+', 'A': '/', 'C': '@:'}
    adverbs = ['AAA', 'AVV', 'NCA', 'VCA', 'ACN', 'ACV']
    conjunctions = ['AAV', 'VVC', 'NVC', 'CVV', 'CVC', 'CAA', 'NCC',
                    'VCC', 'ACA', 'ACC', 'CCN', 'CCV', 'CCA', 'CCC']
    for pattern in adverbs + conjunctions:
        train = '(' + ' '.join(spellings[c] for c in pattern) + ')'
        for left in ['3', '+']:
            if pattern in adverbs:
                yield left + ' ' + train
            else:
                for right in ['3', '-']:
                    yield left + ' ' + train + ' ' + right


def compound_gerund_cases():
    """Stateful AR constructor/error fixtures; no 5!: execution shortcut."""
    out = []
    valid = [
        ",<'+'",
        "(<'/'),<(, <'+')",
        "(<'2'),<((<'+'),<'-')",
        "(<'3'),<((<'+'),(<'%'),<'#')",
        "(<'4'),<((<'+'),<'/')",
        "(<'@:') , <((<'+'),<'-')",
        "(<'\"'),<((<'+'),<((<'0'),<1))",
        "(<'3'),<((<((<'0'),<7)),(<'+'),<'*')",
        "(<((<'4'),<((<'/'),<'/'))),<(, <'+')",
        "(<((<'4'),<((<'/'),(<'/'),<'/'))),<(, <'+')",
    ]
    for value in valid:
        out += ['compoundar=: ' + value, 'compoundfn=: (,<compoundar)\\',
                'compoundfn=: (,<compoundar)"0']
    out += ['compoundkeep=:+']
    invalid = [
        "(0$<0)", "2 2$<0", "(<'+'),(<'+'),<'+'",
        "(<'2'),<(, <'+')", "(<'3'),<((<'+'),<'-')",
        "(<'/'),<((<'+'),<'-')", "(<'@:') , <(, <'+')",
        "(<'/'),<(, <'/')", "(<'3'),<((<'/'),(<'+'),<'*')",
        "(<'3'),<((<'+'),(<'/'),<'*')", "(<'3'),<((<'+'),(<'+'),<'/')",
        "(<'2'),<((<3),<'')", "(<'2'),<((<''),<3)",
        "(<'4'),<((<3),(<''),<3)",
        "(<'3'),<((<3),(<''),<'+')",
        "(<'2'),<(2 2$<'+')", "(<'2'),<1 2", "(<''),<1",
    ]
    for value in invalid:
        out += ['compoundar=: ' + value, 'compoundkeep=: (,<compoundar)\\',
                'compoundkeep i.3', 'compoundfn=: (,<compoundar)"0']
    out += ['rankauditnoun=:7',
            "rankauditar=:(<'3'),<((<'rankauditnoun'),(<''),<'-')",
            'rankauditkeep=:+']
    for right in ["'x'", '1 2 3 4', '(2 2$0)']:
        out += ['rankauditkeep=: (,<rankauditar)"' + right,
                'rankauditkeep i.3']
    for right in ['0', '_', '+']:
        out += ['rankauditfn=: (,<rankauditar)"' + right]
    return out


def gerund_name_cases():
    out = ["gerundnamefn=: (,<'gerund_future')\\", "gerundnamefn=: (,<'gerund_future')\"0",
           'gerund_future=:+', "gerundnamefn=: (,<'gerund_future')\\", 'gerund_future=:-',
           "gerundnamefn=: (,<'gerund_future')\\", 'gerund_alias=:gerund_future',
           'gerund_future=:1', "gerundnamefn=: (,<'gerund_alias')\\", 'gerund_name_keep=:+']
    for binding in ['gerund_future=:1', 'gerund_future=:/', 'gerund_future=:@:']:
        out += [binding, "gerund_name_keep=: (,<'gerund_future')\\", 'gerund_name_keep i.3',
                "gerundnamefn=: (,<'gerund_future')\"0"]
    for name in ['bad+', 'bad name', 'bad_', 'a@']:
        out += [f"gerund_name_keep=: (,<'{name}')\\", 'gerund_name_keep i.3']
    return out


def gerund_snapshot_cases():
    out = []
    for noun in ['7', '9', 'i.4', "'x'", '<1 2']:
        out += ['gssnapshot=:' + noun, "gsar=:(<'3'),<((<'gssnapshot'),(<'+'),<'-')",
                'gerundnamefn=:(,<gsar)\\', 'gerundnamefn=:(,<gsar)"0']
    out += ['gsrank=:0', "gsar=:(<'\"'),<((<'+'),<'gsrank')", 'gerundnamefn=:(,<gsar)\\',
            'gsrank=:2', 'gerundnamefn=:(,<gsar)\\', 'gssnapshot=:7',
            "gsar=:(<'3'),<((<'gssnapshot'),(<''),<'-')", 'gerund_name_keep=:+',
            'gerund_name_keep=:(,<gsar)\\', 'gerund_name_keep i.3', 'gerundnamefn=:(,<gsar)"0']
    return out


def constructor_call_cases():
    out = ['callkeep=:+']
    calls = [
        "(<'4'),<((<'-'),<((<'0'),<7))",
        "(<'4'),<((<((<'0'),<2)),(<'+'),<((<'0'),<3))",
        "(<'4'),<((<'#'),<((<'0'),<1 2 3))",
        "(<'4'),<((<((<'0'),<1 2)),(<'*'),<((<'0'),<3 4))",
        "(<'4'),<((<'-'),<((<'0'),<'x'))",
        "(<'4'),<((<((<'0'),<1 2)),(<'+'),<((<'0'),<1 2 3))",
    ]
    for inner in calls:
        out += ['callar=:' + inner, "callouterar=:(<'3'),<((<callar),(<'+'),<'-')",
                'callkeep=:(,<callouterar)BACKSLASH', 'callfn=:(,<callouterar)"0',
                'callfn=:(,<callar)"0', 'callkeep=:(,<callar)BACKSLASH',
                "callrankar=:(<'\"'),<((<'+'),<callar)", 'callfn=:(,<callrankar)BACKSLASH']
    out += ['callinput=:1 2 3', 'callverb=:+', "callar=:(<'4'),<((<'callverb'),<'callinput')",
            "callouterar=:(<'3'),<((<callar),(<'+'),<'-')", 'callfn=:(,<callouterar)BACKSLASH',
            'callinput=:4 5', 'callfn=:(,<callouterar)BACKSLASH']
    return [s.replace('BACKSLASH', chr(92)) for s in out]


def late_modifier_cases():
    out = ['lateadv=:/', 'latetrain=:lateadv /', 'latealias=:latetrain',
           'latefn=:+latetrain', 'lateadv=:BACKSLASH', 'latefn=:+latetrain',
           'latefn=:+latealias', 'latekeep=:+', 'lateadv=:@:',
           'latekeep=:+latetrain', 'latekeep i.3', 'lateadv=:1',
           'latekeep=:+latetrain', 'latekeep i.3', 'lateadv=:/',
           'latetriple=:lateadv / /', 'latefn=:+latetriple',
           'lateadv=:BACKSLASH', 'latefn=:+latetriple',
           'lateconj=:"', 'latebound=:lateconj 1', 'lateleft=:+lateconj',
           'latecj=:lateconj /', 'latecjcopy=:latecj', 'latefn=:+latebound',
           'latefn=:1 lateleft', 'latefn=:+latecj -', 'lateconj=:@:',
           'latekeep=:+latebound', 'latekeep i.3', 'latefn=:+latecjcopy -',
           'lateconj=:1', 'latekeep=:+latecj -', 'latekeep i.3']
    for binding in ['/', 'BACKSLASH']:
        out += ['lateadv=:' + binding,
                "latear=:(<((<'4'),<((<'lateadv'),<'/'))),<(,<'+')",
                'latefn=:(,<latear)BACKSLASH']
    for binding in ['"', '@:']:
        out += ['lateconj=:' + binding,
                "latear=:(<((<'4'),<((<'lateconj'),<'/'))),<((<'+'),<'-')",
                'latefn=:(,<latear)BACKSLASH']
    out += ['lateadv=:@:', "latear=:(<((<'4'),<((<'lateadv'),<'/'))),<(,<'+')",
            'latekeep=:(,<latear)BACKSLASH', 'latekeep i.3']
    out = [source for source in out if source != 'latekeep i.3']
    out += ['latebase=:+', 'dynamicadv=:latebase "', 'dynamictrain=:dynamicadv /',
            'latefn=:1 dynamictrain', 'dynamicadv=:/', 'latefn=:+dynamictrain',
            'latekeep=:+', 'dynamicadv=:1', 'latekeep=:+dynamictrain', 'latekeep i.3',
            'dynamicconj=:/ / latebase', 'dynamiccj=:dynamicconj /',
            'latefn=:+dynamiccj -', 'dynamicconj=:@:', 'latefn=:+dynamiccj -',
            'dynamicconj=:1', 'latekeep=:+dynamiccj -', 'latekeep i.3']
    return [source.replace('BACKSLASH', chr(92)) for source in out]


def modifier_inventory_cases():
    """All 16 bident and 64 trident ARs, including invalid productions.

    Boxed AR construction bypasses surface reduction ambiguity. Prefix's
    gerund audit requires a final Verb; an actual modifier/noun is rejected.
    """
    import itertools
    leaves = {'N': "((<'0'),<3)", 'V': "'+'", 'A': "'/'", 'C': "'@:'"}
    for arity in (2, 3):
        for parts in itertools.product('NACV', repeat=arity):
            args = ','.join('(<'+leaves[part]+')' for part in parts)
            yield "inventoryar=:(<'4'),<(" + args + ')'
            yield 'inventoryfn=:(,<inventoryar)' + chr(92)


def explicit_modifier_cases():
    return [
        "emglobal=:9", "emconstant=:1 : '42'", "emidentity=:1 : 'u'",
        "emsum=:2 : 'm+n'", "emright=:2 : 'v'", "emreduce=:1 : 'u/'",
        "emadverb=:1 : '/'", "emconjunction=:1 : '\"'", "emdirect=:{{u/}}",
        "+emconstant", "+emconstant + 1", "5 emidentity", "(i.2 3) emidentity",
        "(<1 2) emidentity", "(i.0) emidentity", "2 emsum 3", "1 2 emsum 3 4",
        "emresult=:+emidentity", "emresult 7", "emresult=:+emreduce", "emresult i.4",
        "emresult=:+emdirect", "emresult i.4", "emresult=: + emright -", "emresult 7",
        "emresult=:+emadverb", "+emresult i.4", "emresult=:+emconjunction", "+emresult 0 (7)",
        "emgerund=:1 : '(,<''u'')\\'", "emresult=:+emgerund",
        "emlate=:1 : 'emglobal+u'", "3 emlate", "emglobal=:20", "3 emlate",
        "emalias=:emconstant", "emresult=:+emalias", "emresult",
        "emconstant=:1 : '17'", "+emalias", "emresult",
        "emkeep=:+", "emfailure=:1 : 'u+1 2 3'", "emkeep=:1 2 emfailure", "emkeep 7",
        "emdomain=:1 : 'u+1'", "emkeep=:'x' emdomain", "emkeep 7",
    ]


def modifier_scope_cases():
    return [
        "smt=:90", "smcount=:0", "smkeep=:+", "smlocal=:1 : 'smt=.u+1'",
        "4 smlocal", "smt", "7 smlocal", "smt", "smglobal=:1 : 'smcount=:smcount+u'",
        "3 smglobal", "smcount", "smcopy=:1 : 'smt=.smt+u'", "2 smcopy", "3 smcopy", "smt",
        "smidentity=:1 : 'smt=.u'", "(i.2 3) smidentity", "(<1 2) smidentity",
        "smfn=:1 : 'smf=.u'", "smresult=:+smfn", "smresult 7",
        "smoperand=:1 : 'u=:9'", "smkeep=:+smoperand", "smkeep 7",
        "smrhsfail=:1 : 'smcount=:1 2+1 2 3'", "+smrhsfail", "smcount",
        "smreplace=:1 : 'u=.m+1'", "4 smreplace", "smcount", "smt",
    ]


def vocabulary_binding_cases():
    return ['a.', 'a:', '#a.', '>a:',
            'nv2verb=:c.', 'nv2alias=:nv2verb', 'nv2verb=:+', 'nv2alias 7',
            'nv2mod=:t.', 'nv2saved=:nv2mod', 'nv2mod=:"',
            "nv2decoded=: (, <'!') (\\ @: +)"]


def nuvoc_selector_cases():
    return ['2 [. 3', '2 ]. 3', '2 ]:', '2 [. +', '+ ]. 3',
            '(1+2) [. (3*4)', '(1+2) ]. (3*4)', '((1+2) ]. (3*4)) ]:',
            '(+ [. -) 7', '(+ ]. -) 7', '(+ ]:) 7', '3 (+ [. -) 7',
            'nsright=:-', 'nsf=:+ ]. nsright', 'nsf 7', 'nsright=:+', 'nsf 7',
            'nsright=:9', 'nsf 7', 'nsnoun=:9', 'nsg=:nsnoun [. +', 'nsnoun=:3', 'nsg',
            'nscount=:0', '2 [. (nscount=:3)', 'nscount', '2 [. (1 2+1 2 3)', 'nscount',
            'nsc=:(]: [.)', 'nsf=:+nsc-', 'nsf 7', 'nsc=:(]: ].)', 'nsf=:+nsc-', 'nsf 7',
            'nsa=:(]: ]:)', 'nsf=:+nsa', 'nsf 7']


VERB_RANK_PRIMITIVES = ['+', '-', '*', '%', '$', '$.', '#', ',', '=', '<', '>', '{', '|',
                        'i.', '|.', '|:', '{.', '}.', 'i:', 'I.', 'e.', 'E.', 'u.', 'v.', '[:']
VERB_RANK_DERIVED = ['+/', '+\\', '+"_1', '+"_1 0 1', '+@:-', '+ -', '+ % #', '[: + -',
                     '3 + -', ',"+', '3"+', '(,<\'+\')"0', "3 : 'y'", '(+ ("-))']


def verb_rank_cases():
    out = []
    for primitive in VERB_RANK_PRIMITIVES:
        for arg in ['i.2 3', '1 2 3', "'ab'"]:
            out.append('(,"' + primitive + ') ' + arg)
    out += ['vr=:+', 'vralias=:vr', 'vr=:#', 'vrf=:,"vralias', 'vrf i.2 3',
            'vralias=:#', 'vrf i.2 3', 'vralias=:9', 'vrf i.2 3', 'vralias=:/', 'vrf i.2 3',
            'vrf=:,"(+"_1)', 'vrf i.2 3', 'vrf=:,"(+"1 0 1)', 'vrf i.2 3',
            'vrf=:,"vrfuture', 'vrf i.2 3', 'vrfuture=:+', 'vrf i.2 3',
            'vrleft=:-', 'vrf=:vrleft"+', 'vrf 7', 'vrleft=:+', 'vrf 7', 'vrleft=:9', 'vrf 7',
            '(,"+) i.0 3', '(,"+) 0 3$\'x\'', '(+ ("-)) i.4',
            '1 2 (+"{) i.2 3', '1 2 3 (+"(+"0 0 1)) i.2 3', '1 2 (+"{) i.2 3', '1 2 (+"i.) i.2 3',
            "vra=:1 : ',\"u y'", 'vrf=:+vra', 'vrf i.2 3',
            "vra=:1 : ',\"u. y'", 'vrf=:+vra', 'vrf i.2 3']
    out += [
            "vre=:1 : ',\"u.'", 'vref=:+vre', 'vref i.2 3', '(,"vref) i.2 3',
            "vre=:1 : ',\"u'", 'vreg=:+vre', 'vreg i.2 3',
            "vre=:1 : '+\"u.'", 'vreh=:-vre', 'vreh 7']
    return out


def verb_rank_multiline_cases():
    return ['vrcount=:0', 'vrop=:1 : 0\nvrcount=:vrcount+1\nu y\n)', 'vrr=:+vrop',
            'vrf=:,"vrr', 'vrf i.2 3', 'vrcount',
            'vrinner=:1 : 0\nvrr=.u\n(,"vrr) y\n)', 'vrf=:+vrinner', 'vrf i.2 3',
            'vrr=:#', 'vra=:1 : 0\nvrr=.u\nvrtemp=.,"vrr\nvrr=.#\nvrtemp y\n)',
            'vrf=:+vra', 'vrf i.2 3', 'vrr i.2 3']


def capped_fork_cases():
    return ["[: 7", "3 [: 7", "[: 'x'", "[: i.0", "([: + -) 7", "3 ([: + -) 7",
            "cf=:([: + -)", "cf 1 2 3", "cf i.0", "3 cf 7",
            "cname=:[:", "cf=:cname + -", "cf 7", "cname=:+", "cf 7",
            "cname=:9", "cf 7", "cname=:/", "cf 7",
            "cname=:[:", "calias=:cname", "cchain=:calias + -", "cchain 7",
            "cname=:+", "cchain 7", "cname=:9", "cchain 7", "cf 7",
            "ch=:-", "cg=:+", "cf=:[: cg ch", "cf 7", "ch=:+", "cf 7",
            "cg=:#", "cf 1 2 3", "cg=:9", "cf 7", "ch=:9", "cf 7",
            "ca=:1 : '(u + -) y'", "cf=:[:ca", "cf 7",
            "ca=:1 : 'x ([: + u.) y'", "cf=:-ca", "3 cf 7",
            "cname=:[:", "car=:(<'3'),<((<'cname'),(<'+'),<'-')", "cf=:(,<car)\\",
            "cname=:+", "cf=:(,<car)\\"]


def capped_fork_multiline_cases():
    return ["ccount=:0", "cinner=:1 : 0\nccount=:(ccount*10)+1\nu y\n)",
            "couter=:1 : 0\nccount=:(ccount*10)+2\nu y\n)",
            "ch=:-cinner", "cg=:+couter", "cf=:[: cg ch", "cf 7", "ccount",
            "ccount=:0", "ch=:[:cinner", "cf 7", "ccount", "ch=:-cinner",
            "cg=:[:couter", "cf 7", "ccount", "cg=:+couter", "cf 7", "ccount",
            "cname=:+", "ca=:1 : 0\ncname=.u\n(cname + -) y\n)", "cf=:[:ca", "cf 7", "cname 7"]


def noun_fork_cases():
    return ["nfcomputed=:(1+2) + *", "nfcomputed 4", "(3 + -) 7", "3 (3 + +) 7", "(3 + -) 1 2 3", "(3 + -) i.0",
            "nfnoun=:1 2 3", "nff=:(nfnoun + -)", "nfnoun=:9", "nff 1 2 3", "nff 1 2", "nff i.0",
            "nfright=:-", "nff=:(3 + nfright)", "nff 7", "nfright=:+", "nff 7", "nfright=:9", "nff 7",
            "nfa=:1 : '(3 + u.) y'", "nff=:-nfa", "nff 7", "nff i.0",
            "nfd=:1 : 'x (3 + u.) y'", "nff=:+nfd", "3 nff 7"]


def noun_fork_multiline_cases():
    return ["nfcount=:0", "nfop=:1 : 0\nnfcount=:nfcount+1\nu y\n)", "nfr=:-nfop",
            "nff=:(1 2 3 + nfr)", "nff 1 2", "nfcount", "nfr 7", "nfcount",
            "nfinner=:1 : '(3 + u.) 7'",
            "nfouter=:1 : 0\nnfn=.-\nnfn nfinner\n)", "+nfouter"]


def empty_scope_cases():
    out = ["esa=:1 : 'u./ y'"]
    for op in ['+', '-', '*', '%']:
        out += ["esf=:"+op+"esa", "esf i.0", "esf i.0 3", "esf 0 2 3$1", "esf 0$'x'", op+"/i.0"]
    out += ["esfn=:+", "esf=:esfn esa", "esfn=:*", "esf i.0 3",
            "esa=:1 : 'u.\"0 y'", "esf=:,esa", "esf i.0 3", "esf 0 2 3$1", "esf 0 3$'x'",
            "esg=:,\"0", "esg i.0 3", "esg 0 3$'x'", "esg=:,\"1", "esg i.0 3",
            "esg=:,\"_1", "esg i.2 0 3"]
    return out


def empty_scope_multiline_cases():
    return ["esfn=:+", "esinner=:1 : '(u.\"1) i.0 3'",
            "esouter=:1 : 0\nesfn=.,\nesfn esinner\n)", "+esouter", "esfn 7",
            "esinner=:1 : 'u./i.0 3'",
            "esouter=:1 : 0\nesfn=.*\nesfn esinner\n)", "+esouter", "esfn 7"]


def implicit_wrapper_cases():
    return [
        "iwa=:1 : 'u./ y'", "iwf=:-iwa", "iwf 1 2 3 4", "iwf 7", "iwf ,7", "iwf i.3 2",
        "iwa=:1 : 'u.\"0 y'", "iwf=:,iwa", "iwf 1 2 3", "iwf i.2 3",
        "iwa=:1 : '(u. + u.) y'", "iwf=:-iwa", "iwf 7", "iwf 1 2 3", "iwf i.0 3",
        "iwa=:1 : '(u. u.) y'", "iwf=:-iwa", "iwf 7",
        "iwa=:1 : '(u. @: u.) y'", "iwf=:-iwa", "iwf 7",
        "iwc=:2 : '(u. + v.) y'", "iwf=:+iwc -", "iwf 7",
        "iwd=:1 : 'x (u. + u.) y'", "iwf=:+iwd", "3 iwf 7", "1 2 iwf 3 4",
        "iwd=:1 : 'x (u. u.) y'", "iwf=:+iwd", "3 iwf 7",
        "iwd=:1 : 'x (u.\"0) y'", "iwf=:+iwd", "3 iwf 1 2 3", "1 2 iwf 3 4", "1 2 iwf 3 4 5",
    ]


def implicit_wrapper_multiline_cases():
    return [
        "iwfn=:+", "iwinner=:1 : 'u./ 1 2 3'",
        "iwouter=:1 : 0\niwfn=.-\niwfn iwinner\n)", "+iwouter", "iwfn 7",
        "iwinner=:1 : '(u.\"0) 1 2 3'", "+iwouter",
        "iwcount=:0", "iwleft=:1 : 0\niwcount=:(iwcount*10)+1\nu y\n)",
        "iwright=:1 : 0\niwcount=:(iwcount*10)+2\nu y\n)", "iwl=:+iwleft", "iwr=:-iwright",
        "(iwl + iwr) 7", "iwcount", "iwcount=:0",
        "iwbad=:1 : 0\niwcount=:(iwcount*10)+3\nu y+1 2 3\n)", "iwb=:+iwbad",
        "(iwl + iwb) 1 2", "iwcount", "iwl 8", "iwcount",
    ]


def implicit_call_cases():
    return [
        "u. 7", "v. 7", "icu=:1 : 'u. y'", "icf=:-icu", "icf 7", "icf i.4", "icf i.0 3", "3 icf 7",
        "icc=:2 : 'u. y+v. y'", "icg=:+icc -", "icg 7", "icg i.4",
        "icd=:1 : 'x u. y'", "icdyad=:+icd", "3 icdyad 7", "1 2 icdyad 3 4", "icdyad 7",
        "icbad=:3 icu", "icbad 7", "icmissing=:1 : 'v. y'", "icbad=:+icmissing", "icbad 7", "icg 7",
    ]


def implicit_call_multiline_cases():
    return [
        "icfn=:+", "icinner=:1 : 'u. 7'",
        "icouter=:1 : 0\nicfn=.-\nicfn icinner\n)", "+icouter", "icfn 7",
        "icinner=:1 : 'u 7'", "+icouter",
        "icinner=:1 : 0\nu=.-\nu. 7\n)", "+icinner",
        "icpublish=:1 : 0\nicpub=:u.\nu. 7\n)", "-icpublish", "icpub 7",
        "icinner=:1 : 'u. 7'",
        "icouter=:1 : 0\nicfn=.-\nictmp=.icfn icinner\nicfn 8\n)", "+icouter", "icfn 7",
        "iccount=:0", "icfail=:1 : 'u y+1 2 3'",
        "icinner=:1 : 0\niccount=:iccount+1\nu. 1 2\n)",
        "icouter=:1 : 0\nicfn=:+icfail\nicfn icinner\n)", "+icouter", "iccount", "icg 7", "icfn 7",
    ]


def implicit_operand_cases():
    return [
        "ia=:1 : 'u.'", "if=:-ia", "if 7", "3 if 7", "if i.4", "if i.0 3",
        "ir=:1 : 'u./'", "isum=:+ir", "isum 1 2 3", "isum i.0",
        "ic=:2 : 'v.'", "ig=:+ic -", "ig 7", "3 ig 7",
        "iname=:+", "il=:iname ia", "iname=:-", "il 7", "iname=:0", "il 7",
        "ikeep=:+", "ikeep=:3 ia", "ikeep 7",
        "id=:{{u.}}", "if=:-id", "if 7", "idc=:{{v.}}", "ig=:+idc -", "ig 7",
    ]


def implicit_operand_multiline_cases():
    return [
        "ia=:1 : 0\nu=.-\nu.\n)", "if=:+ia", "if 7",
        "ia=:1 : 0\nipub=:u.\nu.\n)", "if=:-ia", "if 7",
        "ia=:1 : 0\nu=.3\nu.\n)", "if=:+ia", "if 7",
        "ia=:1 : 'u. + u.'", "if=:-ia",
        "ia=:1 : 'u.\"0'", "if=:+ia", "if 1 2 3",
        "if=:,ia", "if 1 2 3",
    ]


def operator_definition_cases():
    return [
        "oa=:1 : 'u y'", "of=:+oa", "of 7", "of i.4", "of i.0 3", "3 of 7",
        "oc=:2 : 'u y+v y'", "og=:+oc -", "og 7", "og i.4", "3 og 7",
        "on=:1 : 'm+y'", "ondata=:3", "oh=:ondata on", "oh 7", "oh i.4",
        "ondata=:9", "oh 7",
        "od=:1 : 'x u y'", "oi=:+od", "oi 7", "3 oi 7", "1 2 oi 3 4",
        "olate=:+", "ol=:(olate oa)", "ol 7", "olate=:-", "ol 7", "olate=:0", "ol 7",
        "okeep=:+", "oe=:1 : 'u y+1 2 3'", "obad=:+oe", "okeep=:obad 1 2", "okeep 7",
        "odirect=:{{u y}}", "of=:-odirect", "of 7",
    ]


def operator_definition_multiline_cases():
    return [
        "opcounter=:0", "opkeep=:+",
        "opdelayed=:1 : 0\nopcounter=:opcounter+1\nu y\n)",
        "opverb=:-opdelayed", "opcounter", "opverb 7", "opcounter", "opverb 8", "opcounter",
        "opdelayed=:1 : 'm+y'", "opverb 9", "opcounter",
        "opboth=:1 : 0\nu y\n:\nx u y\n)", "opverb=:+opboth", "opverb 7", "3 opverb 7",
        "opfail=:1 : 0\nopcounter=:opcounter+1\noptmp=.y\n1 2+1 2 3\n)",
        "opbad=:+opfail", "opkeep=:opbad 7", "opcounter", "opkeep 7",
        "opnonnoun=:1 : 0\ny\nu\n)", "opbad=:+opnonnoun", "opbad 7",
        "opconstant=:1 : 0\ny\nm\n)", "opnoun=:i.8", "opverb=:opnoun opconstant",
        "opnoun=:0", "opverb 1",
    ]


def ordinary_reference_cases():
    return [
        "srfunc=:+", "srself=:1 : 'srfunc=.srfunc'",
        "srres=:-srself", "srres 7",
        "srcollision=:1 : 'srfunc=.u'", "srcopy=:srfunc srcollision",
        "srfunc=:-", "srres 7", "srcopy 7",
        "srfunc=:9", "srres 7", "srcopy 7", "-srself", "srfunc",
        "srundefined=:1 : 'srfuture=.srfuture'", "srres=:+srundefined",
        "srres 7", "srfuture=:-", "srres 7", "srfuture=:0", "srres 7",
    ]


def ordinary_reference_multiline_cases():
    return [
        "srfunc=:+", "srkeep=:+",
        "srinner=:1 : 'u 7'", "srreturn=:1 : 'u'",
        "srcross=:1 : 0\nsrfunc=.u\nsrfunc srinner\n)", "-srcross",
        "srescape=:1 : 0\nsrfunc=.u\nsrfunc srreturn\n)",
        "srres=:-srescape", "srres 7",
        "srpublish=:1 : 0\nsrfunc=.u\nsrexport=:srfunc/\n)",
        "srres=:-srpublish", "srexport 1 2 3",
        "srfunc=:-", "srres 1 2 3", "srexport 1 2 3",
        "srfunc=:+",
        "srfail=:1 : 0\nsrfunc=.u\nsrexport=:srfunc/\n1 2+1 2 3\n)",
        "srkeep=:-srfail", "srexport 1 2 3", "srkeep 7", "srfunc 7",
        "srfunc=:0", "srexport 1 2 3", "srres 1 2 3",
    ]


def modifier_scope_multiline_cases():
    # Physical newlines belong to the stage probe, never the line CLI corpus.
    cases = [
        "smt=:90", "smf=:99", "smcount=:0", "smkeep=:+",
        "smmulti=:1 : 0\nsmt=.u+1\nsmt*2\n)", "4 smmulti", "7 smmulti", "smt",
        "smdirect=:{{smt=.u+1\nsmt*2}}", "4 smdirect", "smt",
        "smconj=:2 : 0\nsmt=.m+n\nsmt+1\n)", "3 smconj 4", "smt",
        "smcall=:1 : 0\nsmf=.u\nsmf 7\n)", "-smcall", "smf",
        "smescape=:1 : 0\nsmf=.u\nsmg=.smf/\nsmf=.-\nsmg\n)",
        "smresult=:+smescape", "smresult 1 2 3", "smf",
        "smadvlocal=:1 : 0\nsma=./\nu sma\n)", "smresult=:+smadvlocal", "smresult i.4",
        "smfail=:1 : 0\nsmcount=:smcount+1\nsmt=.u+1\n1 2+1 2 3\n)",
        "smkeep=:4 smfail", "smcount", "smt", "smkeep 7",
        "smcollision=:1 : 0\nsmt=:1\nsmt=.2\nsmt\n)", "+smcollision", "smt",
        "smcollision2=:1 : 0\nsmt=.1\nsmt=:2\nsmt\n)", "+smcollision2", "smt",
        "smnonnoun=:1 : 0\n+\nsmcount=:9\n7\n)", "+smnonnoun", "smcount",
        "smlast=:1 : 0\nsmt=.u\nsmt=.smt+1\n)", "5 smlast", "smt",
    ]

    return cases


def definition_code_cases():
    """Construction only: bodies must stay unevaluated and names unresolved."""
    return [
        'defcounter=:0',
        'defcode=:{{y+1}}', 'defalias=:defcode',
        'defcode=:{{x+y}}', 'defalias=:defcode',
        'defcode=:{{u y}}', 'defcode=:{{u v y}}',
        'defcode=:{{u x+y}}', 'defcode=:{{u}}', 'defcode=:{{v}}',
        'defcode=:{{}}', "defcode=:3 : ''",
        "defcode=:3 : 'futuredef+y'",
        "defcode=:4 : 'x+y'",
        "defcode=:1 : 'u y'", "defcode=:2 : 'u v y'",
        'defcode=:{{defcounter=:defcounter+y}}', 'defcounter',
        "defcode=:3 : 'defcounter=:99+y'", 'defcounter',
        "defcode=:1 : 'defcounter=:99+u y'", 'defcounter',
        "defcode=:2 : 'defcounter=:99+u v y'", 'defcounter',
        'defcode=:{{defcounter=:99+y}}', 'defcounter',
        'defcode=:{{defcounter=:99+u y}}', 'defcounter',
        'defcode=:{{defcounter=:99+u v y}}', 'defcounter',
        'defkeep=:+', "defkeep=:3 : 'if. y do.' 1 2+1 2 3", 'defkeep 7',
        "defkeep=:3 : 'for_. y do. y end.'", 'defkeep 7',
        "defkeep=:3 : 'for_1a. y do. y end.'", 'defkeep 7',
    ]


def definition_flow_bodies():
    return [
        'if. y do. 1 end.', 'if. y do. 1 else. 0 end.',
        'if. y do. 1 elseif. x do. 2 else. 3 end.',
        'while. y do. y end.', 'whilst. y do. break. end.',
        'for_i. y do. continue. break. end.',
        'while. y do. for_i. y do. continue. break. end. break. end.',
        'if. if. y do. z end. do. a end.',
        'assert. y', '1 return.', 'throw.',
        'goto_done. y label_done. y', 'label_again. y goto_again.',
        'goto_. label_. y', 'goto_1x. label_1x. y',
        'label_same. label_same. y',
        'goto_same. label_same. label_same.', 'goto_missing.',
        'goto_x. label_xx.', 'goto_a. if. y do. label_a. y end.',
        'if. y do. goto_a. end. label_a. y',
        'goto_a. label_a.\n:\ngoto_a. label_a.',
        'goto_a.\n:\nlabel_a.',
        'try. y catchd. goto_a. label_a. y catcht. y end.',
        '1 while. y do. 2 end.', 'while. y do. 2 end. 3',
        '1 if. y do. 2 end. return.', 'if. y do. y end. 3',
        'try. y catch. y end. 3', 'select. y case. 1 do. y end. 3',
        'select. y case. 1 do. 2 end.',
        'select. y case. 1 do. 2 fcase. 3 do. 4 case. 5 do. 6 end.',
        'while. y do. select. y case. 1 do. break. 0 end. end.',
        'while. y do. select. y case. 1 do. select. z case. 2 do. continue. 0 end. 0 end. end.',
        'if. do. else. if. end.',
        'select. y end.', 'case. y do. 1 end.', 'select. y do. 1 end.',
        'try. y catch. 1 end.', 'try. y catchd. 2 catch. 1 catcht. 3 end.',
        'try. try. y catchd. 1 end. throw. catch. 2 end.', 'try. y catcht. 3 end.',
        'try. end.', 'catch. end.', 'try. catch. catch. end.', 'try. if. y catch. end.',
        'if.', 'if. y end.', 'do.', 'else.', 'elseif.', 'end.',
        'break.', 'continue.', 'assert.', 'assert. do.', 'assert. assert. y',
        'if. y do. else. else. end.', 'while. y end.',
        'for_i. y do. if. y do. 1 end.', 'if. y do. 1 2e',
    ]


def control_sequence_matrix():
    """Bounded exhaustive structural states, including legal empty test bodies."""
    import itertools
    words=['if.','do.','else.','elseif.','end.','while.','whilst.','for.']
    return [' '.join(parts) for size in range(1,4) for parts in itertools.product(words,repeat=size)]


def goto_position_matrix():
    """All insertion gaps in the three templates from pinned test/ggoto.ijs.

    Compare construction only; do not execute the intentionally cyclic bodies.
    """
    templates = [
        ['select.', 'if.', 'y', 'do.', '1', 'else.', '0', 'end.',
         'fcase.', '0', 'do.', "'zero'", 'case.', '1', 'do.', "'one'", 'end.'],
        ['while.', 'if.', 'y', 'do.', '1', 'elseif.', '2', 'do.', '3', 'end.',
         'do.', '4', 'try.', '5', 'catch.', '6', 'end.', 'end.'],
        ['0', 'if.', 'y', 'do.', 'for.', '1', 'do.', '2', 'end.', 'else.',
         'whilst.', '3', 'do.', '4', 'end.', 'end.'],
    ]
    bodies = []
    for template in templates:
        for label in range(len(template)+1):
            labeled = template[:label] + ['label_it.'] + template[label:]
            for jump in range(len(labeled)+1):
                bodies.append('\n'.join(labeled[:jump] + ['goto_it.'] + labeled[jump:]))
    return bodies


def definition_flow_cases():
    return ['flowcounter=:0', "flowfn=:3 : 'if. y do. flowcounter=:99 else. 7 end.'", 'flowcounter'] + [
        sentence for body in definition_flow_bodies() if '\n' not in body
        for sentence in ["flowfn=:3 : '" + body + "'", 'flowcounter']]


def entity_boundary_cases():
    """JE0 RHS transport, snapshots and expected-POS baseline; no new syntax."""
    out = []
    for name, rhs, use in [('jenoun', '7', 'jenoun'), ('jeverb', '+', 'jeverb 3'),
                           ('jeadv', '/', '+jeadv i.4'), ('jeconj', '"', '+jeconj 0 i.4')]:
        out += [name + '=:' + rhs, name + 'copy=:' + name, use,
                'jeouter=:jeinner=:' + name]
    out += ['jevalue=:(jenoun=:9)', 'jenouncopy',
            'jevalue=:(jeverb=:-) 3', 'jevalue',
            'jevalue=: +(jeadv=:/) i.4', 'jevalue',
            'jevalue=: +(jeconj=:")0 i.4', 'jevalue',
            'jeverb=:+', 'jealias=:jeverb', 'jeverb=:-', 'jealias 3', 'jeguard=:+']
    for rhs in ['1', '/', '@:']:
        out += ['jeverb=:' + rhs, 'jeguard=:jealias 3', 'jeguard 3']
    out += ['jelarge=:i.65', 'jefork=:jelarge + -', 'jelarge=:0',
            "jedef=:3 : 'y+1'", 'jedefcopy=:jedef',
            "jeexplicitadv=:1 : 'u y'", 'jeexplicitadv_copy=:jeexplicitadv',
            "jeexplicitconj=:2 : 'u y'", 'jeexplicitconj_copy=:jeexplicitconj',
            'jeadv=:/', 'jeadvcopy=:jeadv', 'jeadv=:BACKSLASH', '+jeadvcopy i.4']
    for name, primitive, call in [('jeexplicitadv', '/', '+jeexplicitadv_copy i.4'),
                                  ('jeexplicitconj', '"', '+jeexplicitconj_copy 0 i.4')]:
        out += [name+'chain=:'+name+'_copy', name+'=: '+primitive, call,
                name+'=:1', 'jeguard=: '+call, 'jeguard 3']
    out += ['jebase=:+', 'jebound=:jebase "', 'jeboundcopy=:jebound',
            'jebound=:/', '+jeboundcopy i.4', 'jebound=:1',
            'jeguard=:+jeboundcopy i.4', 'jeguard 3']
    out += ['jenswap=:i.65', 'jensave=:jenswap']
    for rhs in ['+', '/', '"']:
        out += ['jenswap=:'+rhs, 'jensave']
    out += ['jensave=:0', 'jenswap=:1+i.65', 'jenswap']
    out += ['jecomputed_outer=:jecomputed_inner=:1+2',
            'jecomputed_inner', 'jecomputed_outer',
            'jecomputed_outer=:(jecomputed_inner=:1+i.65)',
            'jecomputed_inner', 'jecomputed_outer']
    return [source.replace('BACKSLASH', chr(92)) for source in out]


def noun_direct_cases():
    out = ['nounraw_counter=:0']
    bodies = ['', 'a', 'abc', "'broken", 'NB. if. {{', 'counter=:99',
              '한글', 'x', ')', '  raw  ', 'a\tb']
    rng=random.Random(20261004)
    alphabet="abc 'NB.:{()=+0129"
    bodies += [''.join(rng.choice(alphabet) for _ in range(rng.randrange(33))).replace('}}','} }') for _ in range(64)]
    for body in bodies:
        out += ['nounraw=:{{)n'+body+'}}', 'nounraw']
    out += ['nounraw_counter', 'nounraw=:{{)na}} {{)nb}}', 'nounraw',
            'nounraw_copy=:nounraw', 'nounraw=:{{)nchanged}}', 'nounraw_copy',
            "nounmixed=:{{)n'broken}} + {{y}}",
            'nounmixed=:{{y}} + ({{)nabc}} + {{y}})',
            'nounraw=:{{)nabc}} NB. }} opaque', 'nounraw']
    return out


def multiple_definition_cases():
    return [
        'multikeep=:+', 'multi_counter=:0',
        'multiroot=:{{y+1}} + {{y-1}}',
        'multiroot=:{{y}} {{x+y}}',
        'multiroot=:{{y}} + {{y}} + {{y}}',
        'multiroot=:{{y}} - {{y}}',
        "multiroot=:{{ '}}' + y}} + {{y}}",
        'multiroot=:{{multi_counter=:99+y}} + {{futuremulti+y}}',
        'multi_counter',
        'multikeep=:{{if.}} + {{y}}', 'multikeep 7',
        'multikeep=:{{y}} + {{if.}}', 'multikeep 7',
        'multikeep=:{{goto_a.}} + {{y}}', 'multikeep 7',
    ]


def cases():
    fixed = [
        'snap=:1', 'copy=:snap', 'snap=:2', 'copy',
        'fn=:+', 'alias=:fn', 'alias 3', 'fn=:*', 'alias 3', '2 alias 3',
        'deferred=:futureverb', 'deferred 3', 'futureverb=:-', 'deferred 3',
        'fn=:7', 'alias 3', 'fn=:+', 'alias 3',
        'summation=:+/', 'summation 1 2 3', 'fn/1 2 3',
        'fn=:absentverb 3', 'alias 3',
        'parenverb=:(+)', 'parenverb 3',
        "('a'+1)+(1 2+1 2 3)", "(1 2+1 2 3)+('a'+1)",
        'missing + (1 2+1 2 3)', 'missing + )',
        'errorhold=:1 2 3', "errorhold=:('a'+1)+(1 2+1 2 3)",
        'errorhold', "errorhold=:(1 2+1 2 3)+('a'+1)", 'errorhold',
        '1 NB.. 2', '1 NB.: 2', '0', '1', '2', '_3', '1 0 1', '1 2 3', '1.5 2 3', '_', '__', '_.',
        "'a'", "'abc'", "''", "'it''s'", "'NB. text'",
        '10 - 3 - 2', '(10 - 3) - 2', '2 * 3 + 4', '- 1 2 3', '% 1 2 4',
        '| _2 0 3', '* _2 0 3', '0 % 0', '1 % 0', '_1 % 0', '0 * _',
        '1 2 3 = 1 0 3', '1 2 3 < 2', '1 2 3 > 2', "'a' = 'b'", "'a' = 2",
        '0 + 0', '1 + 0', '1 * 0', '1 - 0', '+ 1', '+/ 1 0 1', '*/ 1 0 1',
        'i. 0', 'i. 1', 'i. 5', 'i. _4', 'i. 2 3', 'i. 2 _3', 'i. _2 3',
        '$ 3', '# 3', ', 3', '$ i. 2 3', '# i. 2 3', ', i. 2 3',
        '2 3 $ 1 2', '0 $ 3', '2 0 $ 3', '2 3 $ i. 0',
        '0 { 10 20 30', '_1 { 10 20 30', '2 0 { 10 20 30', '1 { i. 2 3',
        '1 2 , 3 4', "'ab' , 'cd'", '1 , 2.5',
        '+/ i. 5', '-/ 1 2 3', '+/ i. 2 3', '+/ i. 0', '*/ i. 0',
        '+/"1 i. 2 3', '$"1 i. 2 3', '#"0 i. 2 3', '+/"_1 i. 2 3',
        '10 20 + i. 2 3', '(i. 2 3) + 10 20',
        '9223372036854775807 + 1', '_9223372036854775808 - 1',
        '9223372036854775807 = 9223372036854775806',
        '1 2 + 1 2 3', "'abc' + 2", '10 { 1 2', 'missingname + 1',
        'a =: i. 4', 'b =: a', 'a =: a + 10', 'a', 'b',
        'a =: 1 2 + 3 4 5', 'a',
    ]
    for rank in ['0','1','2','3','_1','_2']:
        for verb in ['+','-','*','%','=','<','>']:
            fixed.extend([
                f'(i.2 3) {verb}"{rank} (10+i.2 3)',
                f'(i.3) {verb}"{rank} (i.2 3)',
                f'2 {verb}"{rank} (i.2 3)',
                f'(i.2 3) {verb}"{rank} (2)',
            ])
    fixed.extend(['(i.2 3) +"1 (i.2 4 3)', '(i.2 4 3) -"1 (i.2 3)',
                  '(i.2 3) +"1 (i.3 3)', '(i.2 0) +"1 (i.2 0)',
                  '(2 2 $ 1 2 9223372036854775807 4) +"1 (2 2 $ 1)',
                  "'ab' =\"0 'ac'"])
    for ranks in ['0 1','1 0','1 2','2 1','0 0 1','2 0 1','_1 1','1 _1']:
        fixed.extend([f'(i.2 3) +"{ranks} (i.2 3)',
                      f'(i.2 3) -"{ranks} (i.2 3 4)',
                      f'10 +"{ranks} (i.2 3)',
                      f'#"{ranks} i.2 3 4', f'+/"{ranks} i.2 3 4'])
    fixed.extend(['#"1 2 3 4 i.2 3', '#"1.5 2 i.2 3'])
    for noun in ['i.0', 'i.1', 'i.5', 'i.2 3', 'i.2 0', 'i.0 3',
                 'i.2 3 4', "'abc'", '1 0 1', '0.5+i.3', '7']:
        fixed.extend([f'|.({noun})', f'|:({noun})'])
        for verb in ['|.', '{.', '}.']:
            for n in ['0','1','_1','2','_2','7','_7']:
                fixed.append(f'{n} {verb} ({noun})')
    for a in ['3 1 3 2', "'abca'", 'i.2 3', 'i.0', 'i.2 0', '3', '0.0 1.0 2.0']:
        for b in ['3 4 1', "'acx'", 'i.2 3', 'i.0', 'i.3 0', '3']:
            for verb in ['i.', 'i:']:
                fixed.append(f'({a}) {verb} ({b})')
    fixed.extend(['i:0','i:3','i:_3','i:2.5','i:_2.5','i:2.2',
                  'I.0 1 0 1','I.2 0 3','I.3','I.i.0','I._1 2','I.1.5',
                  '(9223372036854775807 9223372036854775806) i.9223372036854775806',
                  '(1.0 2.0) i.(1.0+1e_15)'])
    for a in ['i.0','1','1 2','2 2',"'ana'","''",'1.0 2.0']:
        for b in ['i.0','1','1 2 1 2 2',"'banana'",'0.0 1.0 2.0']:
            for verb in ['e.','E.']:
                fixed.append(f'({a}) {verb} ({b})')
    fixed.extend(['(i.2 3)e.(i.2 3)', '(i.2 3)e.1 2 3', '(i.2 0)e.(i.3 0)'])
    rng = random.Random(20260926)
    for _ in range(200):
        n = rng.randint(1, 16)
        def nums():
            return ' '.join(str(rng.randint(-1000, 1000)).replace('-', '_') for _ in range(n))
        fixed.append(f'({nums()}) {rng.choice(["+", "-", "*", "=", "<", ">"])} ({nums()})')
    for rows in range(5):
        for cols in range(5):
            fixed.extend([f'i. {rows} {cols}', f'3 + i. {rows} {cols}', f'+/ i. {rows} {cols}'])
    # SIMD-sized arrays and scalar tails; compare actual J promotion and values.
    for n in [63,64,65,127,128,129,1025]:
        fixed.extend([f'a =: {n} $ 9223372036854775807 _9223372036854775808 9007199254740993 2 _2',
                      'a + 2', '2 + a', 'a - 2', '2 - a', 'a + a', 'a - a', 'a * 3',
                      f'(i. {n}) + 9223372036854775807',
                      f'0.5 + i. {n}', f'(0.5 + i. {n}) + 2.5'])
    # M2: streaming cell assembly, borrowed accumulator promotion and compact
    # literal tokens. Unsupported rank padding remains a separate Rust test.
    fixed.extend([
        '1e_2', '1  2\t3', '1 _ 3',
        '+/"1 (3 2 $ 2 3 9223372036854775807 1 4 5)',
        '-/"1 (2 3 $ 10 3 1 20 5 2)',
        ',"1 (2 0 $ 1)', '#"1 (2 3 $ \'abcdef\')',
        '$"1 (2 0 $ 1)', '*"0 (2 3 $ _2 0 3)',
    ])
    for cols in [1,64,65,129]:
        fixed.extend([
            f'+/ (3 {cols} $ 2 9223372036854775807 1)',
            f'-/ (3 {cols} $ 2 9223372036854775807 _1)',
            f'+/"1 (4 {cols} $ i. 17)',
            f'+/ (3 {cols} $ 0.5 2.5 4.5)',
        ])
    fixed.extend([
        '< 3', '< i.2 3', '< < 42', '> < i.2 3', '> > < < 42',
        ">(<1 2),<'ab'", '(<1 2),<3 4', '>(<1 2),<3 4',
        '>(<1 2),<3.5 4.5', '2 2$<1 2 3', '>2 2$<1 2 3',
        '>2$<i.0', '>0$<3', '<"0 i.4', '>"0 <"0 i.4',
        'boxsource=:i.4', 'boxsaved=:<boxsource',
        'boxsource=:boxsource+10', '>boxsaved',
        'boxopened=:>boxsaved', 'boxopened=:boxopened+20', '>boxsaved',
    ])
    # Sparse results are inspected through dense component queries so the
    # oracle need not interpret the C engine's private sparse headers.
    fixed.extend([
        'spcheck=:$.2 3$0 1 0 2 0 3', '$spcheck', '#spcheck',
        '2$.spcheck', '3$.spcheck', '4$.spcheck', '5$.spcheck', '7$.spcheck',
        '0$.spcheck', 'spalias=:spcheck', 'spvalues=:5$.spcheck',
        'spvalues=:spvalues+10', '5$.spalias',
        'spcheck=:1$.2 3', '2$.spcheck', '3$.spcheck', '4$.spcheck',
        '5$.spcheck', '0$.spcheck', '$.42', '0$.$.0 1 0 1',
        '0$.$.0.0 1.5 0.0', '0$.$.i.0', '3$.1 2', '6$.spcheck',
        '1$._1 2', '1$.i.0',
    ])
    # F1/P3/P5: construction errors are observed before a derived verb runs.
    fixed.extend(['3/', '+@:3', '3@:+', '(+"((1))) 3'])
    for rank in ['_', '__', '_.', '1e100', '_1e100',
                 '1.00000000000001', '_1.00000000000001',
                 '0 1 _', '1 _1', '_ 0 1', '1.5', "'a'", '1 2 3 4']:
        fixed.extend([f'rankfn=:+"{rank}', 'rankfn 3', 'rankfn i.2 3'])
    for noun in ['1 1$0', '2 2$0', '0 4$0', "1 1$'a'", '0$0', '4$0']:
        fixed.extend([f'rankarg=:{noun}', 'rankfn=:+"rankarg', 'rankfn 3'])
    # F2: stack-entry noun snapshots and completed named modifier/train boundaries.
    # These compare supported observable results/errors, not a C stack trace.
    fixed.extend([
        'entrynoun=:1 2 3', 'entrynoun+entrynoun', 'entryverb=:+',
        'entryverb/entrynoun', 'entrynoun+entryverb/entrynoun',
        '(entryverb/ % #) entrynoun',
        "entrynoun (+\"'bad') entrynoun", "((entryverb/))\"'bad'",
        'entrycopy=:entrynoun', 'entrynoun=:9', 'entrycopy',
        'entryverb=:*', '(entryverb/ % #) entrycopy',
    ])
    # FW-01 / JX-10: source-derived Mean Fork is only an analysis
    # candidate, never an enabled fast path. Compare baseline Rust execution
    # with pinned J C for short, empty, and framed monadic inputs.
    fixed.extend([
        '(+/ % #) 1 2 3 4',
        '(+/ % #) i. 1',
        '(+/ % #) i. 0',
        '(+/ % #) i. 2 3',
    ])

    # P2/P3: concrete row results are available to subsequent constructors.
    fixed.extend([
        'computedrank=:+"(1+0)', 'computedrank i.2 3',
        'computedrank=: +"(#1 2)', 'computedrank i.2 3',
        'computedrank=:+"(0$0)', 'computedrank i.2 3',
        "computedrank=:+\"('a'+1)", 'computedrank i.2 3',
        'computedrank=:+"(1 2+1 2 3)', 'computedrank i.2 3',
        'computedfork=:(1+2) + *',
        'computedrank=:+"(1+0.5)', 'computedrank i.2 3',
    ])
    # P4: parser-time assignment, right-to-left lookup and committed early effects.
    fixed.extend([
        'mid=:0', 'mid+(mid=:2)', 'mid', 'mid+(mid=:mid+1)', 'mid',
        'outer=:99', "outer=:'a'+(mid=:2)", 'outer', 'mid',
        'mida=:midb=:1', 'mida', 'midb', '(mid=.4)', 'mid',
        'midarr=:i.65', 'midalias=:midarr', 'midarr+(midarr=:midarr+1)',
        'midarr', 'midalias', 'midfn=:+', 'midfn (midfn=:2)', 'midfn',
        '(midfn=:+) 3', 'midfn 3',
        '(midfn=:-) 3', 'midfn 3', '+(midadv=:/) i.3', '+midadv i.3',
        '+(midconj=:")0 i.3', '+midconj 0 i.3',
    ])
    fixed.extend([
        'parenwrite=:0', '(parenwrite=:2', 'parenwrite',
        'parenwrite=:0', '((parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', 'parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', ')+(parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', 'parenwrite+(parenwrite=:2', 'parenwrite',
        'parenwrite=:0', "'bad'+(parenwrite=:2", 'parenwrite',
        'parenwrite=:0', "(parenwrite=:2)+('a'+1", 'parenwrite',
    ])
    fixed.extend([
        'modadv=:/', 'modalias=:modadv', 'modf=:+modalias', 'modadv=:1', 'modf i.3',
        '+modalias i.3', 'modadv=:/', '3 modadv',
        'modconj=:\"', 'modg=:+modconj 0', 'modconj=:1', 'modg i.3',
    ])
    # P3: constructing a modifier train does not apply it. Function/POS
    # representation is independently compared by the frontend stage suite.
    fixed.extend('train' + str(i) + '=: ' + expression for i, expression in enumerate([
        '+"', '"1', '3"', '/+', '/\\', '/@:', '@:/', '@:@:',
        '/ / /', '/ / +', '/ + *', '@: + *', '+ @: /', '+ @: @:',
        '(/ /) /', '"1 2', '3 @:', '"+', '@: 3', '(1+2) "',
    ]))
    fixed.extend(['trainerr=: (1 2+1 2 3) "', 'trainerr=: (/3)',
                  'trainitems=:i.65', 'trainarray=:trainitems "',
                  'trainitems=:trainitems+1', 'trainitems'])
    fixed.extend([
        'boundright=:"1', 'boundcopy=:boundright', 'boundright=:1',
        'boundfn=: - boundcopy', 'boundfn i.2 3', '(- ("1)) i.2 3',
        '(- ("1 2)) i.2 3', '(+ ("-)) i.4', '(+ (@:-)) i.4',
        'boundbad=:"\'a\'', 'boundkeep=:+', 'boundkeep=: - boundbad', 'boundkeep i.3',
        'boundkeep=: - ("1 2 3 4)', 'boundkeep i.3', 'boundkeep=: - (@:3)',
        'boundcopy=:"2', 'boundfn i.2 3', 'boundfn=: - boundcopy', 'boundfn i.2 3',
    ])
    fixed.extend([
        'leftbind=:-"', 'leftalias=:leftbind', 'leftbind=:1', 'leftfn=:1 leftalias',
        'leftfn i.2 3', '(1 (-")) i.2 3', 'leftfn=:(1+0) leftalias', 'leftfn i.4',
        'seqfn=: + (/ /)', 'seqfn=: + (/ +)', 'seqfn=: + (/ / /)',
        'leftkeep=:+', 'leftkeep=: \'a\' (-")', 'leftkeep i.3',
        'leftkeep=:1 2 3 4 (-")', 'leftkeep i.3', 'leftkeep=: \'a\' ((-") /)',
        'leftkeep i.3', 'leftkeep=:3 (/@:)', 'leftkeep i.3',
    ])
    fixed.extend([
        'tacfn=: + (/@:)', 'tcafn=: + (@:/) -', 'tccfn=: + (@:@:) -',
        'taavfn=: + (/ / +) *', 'tcanested=: + ((@:/) -)',
        'tcadef=:@:/', 'tcaalias=:tcadef', 'tcadef=:1', 'tcafn=: + tcaalias -',
        'tcckeep=:+', 'tcckeep=: + (" @:) 1 2 3 4', 'tcckeep i.3',
        'tcckeep=: + (@: ") 1 2 3 4', 'tcckeep i.3',
        'tcckeep=: 3 (/ / +) 3', 'tcckeep i.3',
    ])
    fixed.extend([
        'tridentfn=: + (+ + @:) -', 'tridentfn=: + (3 + @:) -',
        'tridentfn=: + (@: + @:) -', 'tridentfn=: + (@: / /) -',
        'tridentfn=: + (+ @: /)', 'tridentfn=: + (+ @: @:) -',
        'tridentfn=: + (/ + -)', 'tridentfn=: + (/ @: -)',
        'tridentfn=: + (/ @: /) -', 'tridentfn=: + (/ @: @:) -',
        'tridentfn=: + (@: + -) *', 'tridentfn=: + (@: @: -) *',
        'tridentfn=: + (@: @: /) -',
        'tridentconj=:@: + @:', 'tridentalias=:tridentconj', 'tridentconj=:1',
        'tridentfn=: + tridentalias -',
        'tridentkeep=:+', 'tridentkeep=: + (" + @:) 1 2 3 4', 'tridentkeep i.3',
        'tridentkeep=: + (@: + ") 1 2 3 4', 'tridentkeep i.3',
        'tridentkeep=: 3 (+ @: /)', 'tridentkeep i.3',
        'tridentkeep=: 3 (/ @: /) +', 'tridentkeep i.3',
        'tridentkeep=: 3 (/ @: @:) 1 2 3 4', 'tridentkeep i.3',
        'tridentkeep=: + (" @: /) 1 2 3 4', 'tridentkeep i.3',
    ])
    fixed.extend('matrixfn=: ' + expression for expression in modifier_trident_cases())
    fixed.extend([
        'constantfn=: 3"0', 'constantfn=: 1 2 3"1 2', "constantfn=: 'abc'\"_",
        'constantfn=: 3"+', 'constantfn=: 3 (" /) 1', 'constantfn=: 3 (+ ")',
        'constantfn=: 3 (+ " /)', 'constantfn=: (i.4)"1',
        'constantkeep=:+', "constantkeep=: 3\"'a'", 'constantkeep i.3',
        'constantkeep=: 3"1 2 3 4', 'constantkeep i.3',
        'constantkeep=: 3"(2 2$0)', 'constantkeep i.3',
        "constantkeep=: (,<'bad')\"1 2 3 4", 'constantkeep i.3',
    ])
    fixed.extend([
        'prefixkeep=:+', 'prefixkeep=: 3\\', 'prefixkeep i.3',
        "prefixkeep=: 'abc'\\", 'prefixkeep i.3',
        'prefixkeep=: (0$0)\\', 'prefixkeep i.3', "prefixkeep=: ''\\", 'prefixkeep i.3',
        'prefixkeep=: (2 2$0)\\', 'prefixkeep i.3', 'prefixkeep=: (0 2$0)\\', 'prefixkeep i.3',
        'prefixkeep=: (0$<0)\\', 'prefixkeep i.3', 'prefixkeep=: (2 2$<0)\\', 'prefixkeep i.3',
    ])
    fixed.extend([
        "gerundfn=: (, <'+')\\", "gerundfn=: ((<'+'),<'-')\\",
        "gerundfn=: (, <'+')\"0", "constantfn=: (, <3)\"0",
        "constantfn=: (, <'')\"0", "constantfn=: (, <'/')\"0",
        'gerundkeep=:+', "gerundkeep=: (, <3)\\", 'gerundkeep i.3',
        "gerundkeep=: (, <'')\\", 'gerundkeep i.3',
        "gerundkeep=: (, <'/')\\", 'gerundkeep i.3', "gerundkeep=: (, <'@:')\\", 'gerundkeep i.3',
        "gerundkeep=: (, <(2 2$'+'))\\", 'gerundkeep i.3',
        "gerundkeep=: ((<3),<'')\\", 'gerundkeep i.3', "gerundkeep=: ((<''),<3)\\", 'gerundkeep i.3',
    ])
    fixed.extend(compound_gerund_cases())
    fixed.extend(gerund_name_cases())
    fixed.extend(gerund_snapshot_cases())
    fixed.extend(constructor_call_cases())
    fixed.extend(late_modifier_cases())
    fixed.extend(modifier_inventory_cases())
    fixed.extend(definition_code_cases())
    fixed.extend(explicit_modifier_cases())
    fixed.extend(modifier_scope_cases())
    fixed.extend(ordinary_reference_cases())
    fixed.extend(operator_definition_cases())
    fixed.extend(implicit_operand_cases())
    fixed.extend(implicit_call_cases())
    fixed.extend(implicit_wrapper_cases())
    fixed.extend(empty_scope_cases())
    fixed.extend(noun_fork_cases())
    fixed.extend(capped_fork_cases())
    fixed.extend(verb_rank_cases())
    fixed.extend(nuvoc_selector_cases())
    fixed.extend(vocabulary_binding_cases())
    fixed.extend(definition_flow_cases())
    fixed.extend(multiple_definition_cases())
    fixed.extend(noun_direct_cases())
    fixed.extend(entity_boundary_cases())
    fixed.extend("matrixflow=:3 : '"+body+"'" for body in control_sequence_matrix())
    fixed.extend("gotomatrix=:3 : '"+body.replace("\n", " ").replace("'", "''")+"'" for body in goto_position_matrix())
    return fixed

# Exact newly exercised runtime coverage gaps. Parser correctness is checked
# separately; these are neither conformance passes nor C baseline deviations.
RUNTIME_COVERAGE_BOUNDARIES = {

}

def runtime_coverage_boundary(source, reference, actual):
    if (source in RUNTIME_COVERAGE_BOUNDARIES and 'data' in reference
            and actual == {'error': 'unsupported'}):
        return RUNTIME_COVERAGE_BOUNDARIES[source]
    return None

def equal(a, b):
    if a.keys() != b.keys():
        return False
    if 'data' not in a:
        return a == b
    if a['type'] != b['type'] or a['shape'] != b['shape'] or len(a['data']) != len(b['data']):
        return False
    if a['type'] == 32:
        return all(isinstance(x, dict) and isinstance(y, dict) and equal(x, y)
                   for x, y in zip(a['data'], b['data']))
    return all((x == y and not (a['type'] == 8 and isinstance(x, (int,float)) and isinstance(y, (int,float)) and x == 0 and math.copysign(1,x) != math.copysign(1,y))) or (a['type'] == 8 and isinstance(x, (int,float)) and isinstance(y, (int,float)) and x != 0 and y != 0 and math.isclose(x, y, rel_tol=1e-14, abs_tol=0)) for x,y in zip(a['data'], b['data']))

def generated(seed, rounds):
    rng = random.Random(seed)
    out = []
    for _ in range(rounds):
        n = rng.choice([0, 1, 2, 3, 63, 64, 65, 127, 128, 129, 257])
        k = rng.randint(-100, 100)
        literal = str(k).replace('-', '_')
        op = rng.choice(['+', '-', '*'])
        # A stateful transaction: alias, update, failed assignment, then inspect.
        out.extend([f'qa=:i.{n}', 'qb=:qa', f'qa=:qa {op} {literal}',
                    'qa', 'qb', 'qa=:1 2 + 1 2 3', 'qa', 'qb'])
    return out


def validate_cli_corpus(corpus):
    """Each case must be one CLI sentence; multiline source uses stage probes."""
    for index, source in enumerate(corpus):
        if '\n' in source or '\r' in source:
            raise ValueError(f'CLI case {index} contains a physical line break; use a stage probe')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--semantic-reference', action='store_true')
    parser.add_argument('--parser-capture', action='store_true', help='Use capture_probe as binary; verify runtime provenance')
    parser.add_argument('--reference-revision', help='Verified source revision of the supplied C library')
    parser.add_argument('--seed', type=int, default=20260926)
    parser.add_argument('--rounds', type=int, default=100)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/rustj')
    parser.add_argument('--report', type=Path, default=ROOT / 'reports/conformance.json')
    parser.add_argument('--allow-known-j64', action='store_true')
    args = parser.parse_args()
    if args.rounds < 0: parser.error('rounds must be nonnegative')
    if args.parser_capture and args.semantic_reference: parser.error('choose one execution path')
    args.report.parent.mkdir(parents=True, exist_ok=True)
    corpus = cases() + generated(args.seed, args.rounds)
    library = Path(os.environ.get('J_LIBRARY', str(ROOT / '.reference/bin/linux/j64/libj.so')))
    report = {'reference_revision': args.reference_revision, 'platform': os.name, 'rust_path': 'parser-capture' if args.parser_capture else ('semantic-reference' if args.semantic_reference else 'direct'), 'seed': args.seed, 'rounds': args.rounds, 'cases': len(corpus),
              'reference_library': str(library),
              'reference_sha256': hashlib.sha256(library.read_bytes()).hexdigest(),
              'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'scope': 'supported subset; upstream suite NOT executed',
              'passed': 0, 'known_deviations': [], 'coverage_boundaries': [], 'failures': []}
    try:
        validate_cli_corpus(corpus)
        oracle = subprocess.run([sys.executable, str(ROOT / 'tools/oracle.py')], input=''.join(json.dumps(s)+'\n' for s in corpus), text=True, encoding='utf-8', capture_output=True, timeout=120)
        rust = subprocess.run([str(args.binary), '--json'] + (['--semantic-reference'] if args.semantic_reference else []), input='\n'.join(corpus)+'\n', text=True, encoding='utf-8', capture_output=True, timeout=120)
        if oracle.returncode != 0 or rust.returncode not in (0,1):
            raise RuntimeError(f'process failure: oracle={oracle.returncode}, rust={rust.returncode}\n{oracle.stderr}\n{rust.stderr}')
        if args.parser_capture:
            report['graph_coverage_boundaries'] = []
            for line in rust.stderr.splitlines():
                for prefix, reason in [
                    ('ordered-effect graph boundary: ', 'capture needs ordered assignment/effect graph'),
                    ('modifier-value graph boundary: ', 'captured modifier value graph lowering'),
                    ('explicit-invocation graph boundary: ', 'explicit modifier body graph requires invocation scope'),
                ]:
                    if line.startswith(prefix):
                        index = int(line[len(prefix):])
                        report['graph_coverage_boundaries'].append({
                            'index': index, 'source': corpus[index], 'reason': reason})
        expected = [json.loads(s) for s in oracle.stdout.splitlines()]
        actual = [json.loads(s) for s in rust.stdout.splitlines()]
        if len(expected) != len(corpus) or len(actual) != len(corpus):
            raise RuntimeError(f'incomplete output: cases={len(corpus)}, oracle={len(expected)}, rust={len(actual)}')
        for i, (source, a, b) in enumerate(zip(corpus, expected, actual)):
            if equal(a,b):
                report['passed'] += 1
                continue
            item = {'index': i, 'source': source, 'reference': a, 'rust': b}
            boundary = runtime_coverage_boundary(source, a, b)
            if boundary is not None:
                report['coverage_boundaries'].append({**item, 'reason': boundary})
                continue
            # Narrow, explicit baseline discrepancy, never a blanket dtype waiver.
            known = (args.allow_known_j64 and library.parent.name == 'j64'
                     and source == '(i.2 3) -"1 0 (i.2 3 4)'
                     and a.get('type') == 8 and b.get('type') == 4
                     and a.get('shape') == b.get('shape') == [2,3,4,3]
                     and a.get('data') == b.get('data'))
            report['known_deviations' if known else 'failures'].append(item)
        if not report['failures']:
            args.report.with_suffix('.repro.ijs').unlink(missing_ok=True)
        if report['failures']:
            # Replay the full prefix: stateful failures cannot be reproduced by one line.
            end = report['failures'][0]['index'] + 1
            args.report.with_suffix('.repro.ijs').write_text('\n'.join(corpus[:end])+'\n')
    except (RuntimeError, ValueError, subprocess.TimeoutExpired) as error:
        report['harness_error'] = str(error)
    report['failed'] = len(report['failures'])
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2), newline='\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ['failures','known_deviations']}, indent=2))
    print('known deviations:', len(report['known_deviations']))
    return bool(report['failures'] or report.get('harness_error'))

if __name__ == '__main__':
    sys.exit(main())

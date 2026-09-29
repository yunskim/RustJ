# Direct / explicit definition parsing audit — 2026-09-29

## Finding

RustJ does **not** parse or execute J function definitions yet. Recognizing words
in the scanner does not establish support in the lexer, semantic tree, binder or
runtime. Existing named aliases to primitive verbs are not explicit definitions.

| Form | Current result |
| --- | --- |
| `{{ y+1 }}`, dyadic and nested direct definitions | scanner recognizes delimiters; semantic parse rejects |
| `{{)n ... }}` noun direct definition | unsupported |
| `3 : 'y+1'`, `4 : 'x+y'` | colon conjunction unsupported |
| `3 : 0` / body / `)` | no definition input collector; unsupported |
| Monad/dyad sections separated by `:` | unsupported |
| `1 : ...`, `2 : ...` adverb/conjunction definitions | unsupported |
| `=.`, local scopes, x/y/u/v operands, control words | no definition binder/statement parser |

`semantic::VerbTarget` currently contains only Primitive and Named. There is no
Definition node, body AST, local frame or compiled function body. `Program` holds
one expression and an optional top-level global assignment. CLI script and stdin
input are processed sentence-by-sentence, not by a definition-aware input reader.

## Confirmed bug and containment fix

Windows stdin repro before the fix:

```
f=:{{
leaked=:99
}}
leaked
```

The opener failed, but the body executed as a global assignment and the last line
printed 99. `f=:3 : 0` with the same body exhibited the same defect. Script-file
mode already stopped at the first error; stdin continued intentionally for ordinary
sentence errors, exposing this definition boundary bug.

The CLI now stops stdin processing after a failed line containing definition
syntax (`{{`, `}}`, `:`, or the `define` name). It does not execute later body lines.
This is a containment guard, **not** definition parsing or multiline support.
Strings and comments are excluded using scanner words, including an unfinished
quoted word following an opener. Strict lexing still rejects unfinished literals.
Valid uses of a name `define` are unaffected; a failure containing that exact name
conservatively stops the stream. Ordinary errors such as length mismatch still
permit following independent sentences as before.

## Source grounding

Pinned C source e75016ca74b5e595dd323226e6a4990172f72ec6:

- `jsrc/w.c`: word formation and enqueue distinguish execution environments.
- `jsrc/cx.c:1326`: ddtokens collects unfinished definitions, handles nested DDs,
  and converts their source before ordinary parsing; EOF behavior is explicit.
- `jsrc/cx.c:796` and `:1234`: explicit body lines, DD preprocessing and definition
  compilation are separate stages; local symbol tables are prepared for execution.
- `test/g020a.ijs:162`: multiline direct definition example.
- `test/g13x.ijs`: multiline explicit verbs, local assignments and return control.

These source reads and Rust tests are not a native C execution comparison.

## Implementation order

1. Definition-aware input framing: strings, doubled quotes, comments, nested DDs,
   noun DD text, explicit `: 0` blocks, EOF, CRLF and complete byte spans.
2. Definition AST: kind, parameters, monad/dyad sections and statement sequences.
   Preserve control structure; do not flatten body lines into one expression.
3. Statement/control parser and validation: if/elseif/else/end, loops, select,
   try/catch, return/break/continue; precise rejection of unsupported subforms.
4. Local/global binding and execution frames: parameter/local slots and deferred
   global lookups. Do not snapshot a function body's globals at definition time.
   Preserve existing noun-by-value / named-verb-at-call semantics where applicable.
5. Lower to compiler IR and execute the supported subset. Test recursion, nested
   definitions, rebinding, local isolation and failed-definition transactions.
6. Run native Windows C differential cases before claiming J compatibility.

Regression tests cover unsupported forms without changing bindings, word-aware
input guards, both evaluator paths, and preventing execution of definition body
lines. These are negative/containment tests, not positive function-parsing tests.

Latest validation: native Windows MSVC default/portable each passed 75 tests
plus one doctest; fmt and Clippy all-targets with warnings denied passed.
Python harness: 6 tests passed. No Linux tests or GitHub CI were run.

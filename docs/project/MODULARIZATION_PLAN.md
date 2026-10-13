# PROJECT document modularization plan and verification checklist

> Status: **Korean root split completed, English mirror migration pending** (2026-10-11). `PROJECT.ko.md` now indexes 40 modules; the English integrated `PROJECT.md` is unchanged. No implementation or CI acceptance status changes.
> Execution tracker: [#64 — PROJECT document split](https://github.com/yunskim/RustJ/issues/64). The Korean counterpart is the authoritative modularization plan; the issue tracks step-by-step evidence and execution status.

## Purpose and document authority

- Preserve the root paths `PROJECT.ko.md` and `PROJECT.md`, reducing both to **short project overviews, tables of contents, and key navigation links**.
- Move detailed contracts, rationale, roadmap, validation policy, and checklists into **subject-specific Markdown files** under `docs/project/`. Each `*.ko.md` is authoritative and the same relative path with `*.md` is its English mirror.
- Prefer **one document / one responsibility** over rigid length limits. Aim for 200–400 lines where practical, allowing exceptions for cohesive contracts, tables, and evidence.
- Use plain GitHub Markdown relative links. Do not require embedded Markdown, include processors, static-site builders, or a duplicated giant generated Markdown as another source of truth.
- Keep `FOUNDATIONS.ko.md` as the mandatory architectural guardrail and `LICENSE`/`COPYING` as governing license documents; do not independently redefine their rules elsewhere.
- **Transition rule:** content not yet moved remains authoritative in the existing `PROJECT.ko.md`. Each moved section becomes authoritative only at its new location. Root documentation must clearly distinguish migrated and unmigrated sections until cutover.

## Confirmed baseline

| Item | Observation from `main` on 2026-10-11 |
| --- | --- |
| Korean project document | `PROJECT.ko.md`: 1,516,273 bytes |
| English project document | `PROJECT.md`: 945,743 bytes; roughly 6,244 lines |
| Existing documentation layout | No subject-specific Markdown modules under `docs/project/` |
| Current policy | `AGENTS.md` designates a single `PROJECT.ko.md` authority and prohibits new design/progress report Markdown files |
| Evidence gap | Full two-language section and checklist inventory is not yet complete |

## Proposed destinations (finalize after P1 source mapping)

| Existing content | Expected Korean canonical destination (paired English file) |
| --- | --- |
| Introduction, goal, high-level architecture, route boundaries | `docs/project/architecture/{overview,route-boundaries}.ko.md` |
| Part I — frontend compatibility, names, parser, definitions | `docs/project/frontend/{pipeline,names-and-definitions}.ko.md` |
| Part II — FunctionEntity/JEntity/extensions | `docs/project/semantics/{entities,extension-primitives}.ko.md` |
| Part III — rank, CellApply, logical/physical array | `docs/project/semantics/{rank-and-cells,array-model}.ko.md` |
| Part IV — J Graph IR / graph basis | `docs/project/ir/graph.ko.md` |
| Parts V–VII — rewrites, facts and resources | `docs/project/optimization/{rewrites,facts-and-resources}.ko.md` |
| Parts VIII–X — logical execution, targets, storage | `docs/project/execution/{logical-lowering,target-planning,flow-and-storage}.ko.md` |
| Part XI — differential and schema/validation policy | `docs/project/validation/policy.ko.md` |
| Parts XII–XIII — support status, milestones, roadmap | `docs/project/status/{implementation,roadmap,checklists}.ko.md` (split further when useful) |
| Part XIV — JAXA history and source/optimization research | `docs/project/research/{jaxa-history,optimization-sources}.ko.md` |
| Part XV — documentation/licensing policy | Reconcile ownership with `AGENTS.md`, `LICENSE`, and `FOUNDATIONS.ko.md`; link to governing policy |

These destinations are proposals until every original section and checklist has been inventoried and mapped. Do not independently re-state a technical contract in several modules.

## Execution checklist

### P0 — Baseline inventory
- [x] Inspect both root files, their sizes, and the current Markdown layout on GitHub `main`
- [x] Identify the existing single-authority and no-new-report policy
- [ ] Pin pre-migration `main` commit SHA, both document blob SHAs, and complete section trees
- [ ] Inventory Korean and English headings, code blocks, tables, checkboxes, links, footnotes, and sources independently
- [ ] Identify inbound file/anchor links in `README`, `AGENTS`, `FOUNDATIONS`, source code, and tooling

### P1 — Section ownership and mapping
- [ ] Assign every migrated unit a `source path/heading/anchor/line range → destination file/anchor` mapping
- [ ] Assign exactly one authority per checklist and maintain its implementation/acceptance evidence
- [ ] Record bilingual omissions, ordering and status mismatches without folding semantic edits into document moves
- [ ] Compare potential duplicates with the existing `FOUNDATIONS` and other governing documents; replace duplicates with references where lossless

### P2 — Policy and documentation scaffolding
- [ ] Update `AGENTS.md` with the transitional and final root-index + `docs/project/**/*.ko.md` authority rules
- [ ] Fix naming conventions, bilingual pairing, links, checklist ownership, and table-of-contents ownership
- [ ] Author this plan and its Korean canonical counterpart; review in a repository PR

### P3 — Lossless incremental splitting
- [x] Extract all 14,498 lines of Korean `PROJECT.ko.md` into 40 linked modules (commit `b3bea373f4ff00d42225603932a0df531e60a56b`)
- [ ] Extract English `PROJECT.md` into corresponding topic modules and verify bilingual parity
- [ ] Reconcile architecture, frontend, NAME and JEntity **bilingual pairs**
- [ ] Reconcile rank/array semantics and NN extension **bilingual pairs**
- [ ] Reconcile Graph IR, rewrites, resources, and logical/physical execution **bilingual pairs**
- [ ] Reconcile validation policy, current implementation status and active checklists **bilingual pairs**
- [ ] Reconcile JAXA history, research references and source audits **bilingual pairs**
- [x] Replace Korean `PROJECT.ko.md` with a short link-oriented index (257 lines)
- [ ] Replace English `PROJECT.md` with a short link-oriented index

### P4 — Verify, integrate and cut over
- [ ] Produce explicit preservation evidence for all headings, tables, code, checkboxes, references, and citations
- [x] Confirm 363 local file links across Korean modules/index resolve to existing repository paths (current branch)
- [x] Add root-level relocation links for 63 explicit historical anchors
- [ ] Verify all generated heading anchors and repository-wide inbound deep links, plus English-side links
- [ ] Compare Korean/English file and heading coverage, implementation state, and checklist semantics
- [ ] Add reproducible structural/link verification scripts and CI checks without disturbing existing CI policy
- [ ] Validate the live PR head, merge, independently recheck `main`, then record final cutover

## Mandatory acceptance gates

1. **Lossless:** preserve examples, tables, footnotes, sources, negative conditions, unresolved status, and every `[x]` / `[ ]`. Never promote historical passing evidence into current acceptance.
2. **Complete mapping:** each original section maps to exactly one destination or a documented governing-source reference. Zero missing sections; zero accidental duplicates.
3. **Semantic boundaries:** do not alter J semantics, Rank/NAME/effects, IR-layer ownership, CPU/GPU routing, or native/external route contracts.
4. **Links:** root/module links resolve on GitHub. GitHub Markdown does not automatically redirect external deep links; document affected paths/anchors and compatibility measures.
5. **Bilingual parity:** Korean canonical and English mirror agree on all gates, states and negative requirements; record preexisting divergences separately.
6. **Evidence:** add exact SHA and PASS/FAIL/UNRUN results to issue #64. Documentation updates do not imply passing Rust tests, CI, or semantic validation.

## Repeatable slice loop

`inventory → choose smallest slice → map source/destination → move Korean and English content → validate preservation/links → PR review → re-read main → update issue #64 → next slice`.

Do not redesign the compiler while splitting documents. Implementation and NN extension acceptance remain governed by the corresponding original implementation/verification gates until explicitly relocated.

## 2026-10-13 PR integration validation

Regenerate the 40 Korean modules from the current integrated canonical source. `tools/check_project_docs.py` checks content preservation, explicit and generated-heading relocation anchors, and repository inbound links; Linux CI reruns it. `reports/project-modularization.json` records the source commit and section hashes. English modularization and bilingual semantic acceptance remain incomplete. Runtime semantics and implementation acceptance are unchanged.

Run `python3 tools/check_project_docs.py --verify-source` for the one-time content-preservation audit. Ongoing CI checks links, structure and pinned source evidence while permitting subsequent intentional documentation edits.

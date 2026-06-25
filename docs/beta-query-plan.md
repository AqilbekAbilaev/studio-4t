# Beta Read-Path Hardening Plan

**Goal:** filter / query / projection / sort (and the aggregation pipeline) work flawlessly
for the beta, accepting MongoDB shell syntax — the same syntax Compass and Studio-3T accept.

**Strategy:** parse shell syntax **on the frontend** with MongoDB's own query parser
(`@mongodb-js/mongodb-query-parser`) → emit canonical **Extended JSON** → the existing Rust
backend decodes that EJSON into BSON (it already does). Then **delete the regex parsing.**

---

## Why (the problem, with evidence)

The query bar currently turns user input into JSON with regexes:

- `toStrictJson` — quotes bare keys via `replace(/([{,]\s*)([a-zA-Z_$]…)\s*:/g, …)`
- `expandShellTypes` — rewrites `ObjectId("…")` → `{"$oid":"…"}`
- `toStrictPipeline` — the same two passes, for aggregation pipelines

A regex has no concept of "inside a string," so this approach is **structurally unfixable**.
Proven failures with the current code:

| Input | Current result |
|---|---|
| `{_id: ObjectId("507f…")}` | ✅ works (after the `expandShellTypes` patch) |
| `{age: {$gt: 18}}` | ✅ works |
| `{note: "hello, world: x"}` | ❌ corrupted → `{"note": "hello, "world": x"}` |
| `{name: /^jo/i}` | ❌ regex literals unsupported |
| `{created: ISODate("2024-01-01")}` | ❌ dates unsupported |
| `{label: "ObjectId(\"507f…\")"}` | ❌ `ObjectId()` *inside a string* wrongly rewritten |

Each regex patch fixes one case and risks adding another (the `ObjectId-in-a-string`
false-positive is itself a bug introduced by a patch). A real parser fixes all of these at
once **and** cannot corrupt string values.

---

## Scope

**In:** the read path — query-bar parsing (filter / sort / projection), aggregation-pipeline
parsing, find/aggregate execution, result rendering, pagination, sort/projection correctness.

**Out (explicit non-goals — logged in ROADMAP, not touched here):** Visual Query Builder,
Tree View, write-path changes beyond existing inline-edit, IntelliShell, index/import/export.

---

## Phase 0 — Spike / de-risk (no app code)

Prove the two assumptions the whole design rests on. Throwaway code only.

- Install `@mongodb-js/mongodb-query-parser` + `bson` (JS) in a scratch area.
- **Assumption A — round-trip:** parse each shell input → `EJSON.stringify` (canonical) →
  feed that exact string into a Rust test asserting `serde_json::from_str::<bson::Bson>`
  yields the right BSON type. Cases: `ObjectId("…")`, `ISODate("…")`, `/^jo/i`,
  `{age:{$gt:18}}`, `{_id:{$in:[ObjectId(…)]}}`, and a pipeline `[{$match:{…}},{$group:{…}}]`.
- **Assumption B — runs under Tauri's CSP:** confirm the parser executes inside the Tauri
  webview. *This is the real unknown* — older parser versions used a sandboxed `eval`/`vm`
  that a strict Content-Security-Policy can block. Check `tauri.conf.json` CSP and the
  installed parser version (acorn-based = fine; eval-based = needs a pinned version or a
  scoped CSP change).

**Gate:** all round-trip cases pass **and** the parser runs in-app. Bring results +
the chosen EJSON dialect before any app code. **If B fails, stop and rethink — do not start Phase 1.**

---

## Phase 1 — Frontend parser integration (the core fix)

**Files:** new `src/utils/queryParser.js`; `QueryWorkspace.vue`.

- `queryParser.js`: `parseField(raw) → { ok, ejson, error }` and `parsePipeline(raw) → { ok, ejson, error }`.
  Empty/`{}`/`[]` → identity. Wraps the library; normalizes its error messages.
- In `runQuery` / `runAggregate`: parse through it; if any field fails, **do not call the
  backend** — set an inline error instead.
- **Inline error UX:** red message under the offending field
  (e.g. "Invalid filter: unexpected token at position 12"); Run disabled while invalid.
  This replaces today's *silent corruption* with a clear, actionable error.
- **Delete** `toStrictJson`, `toStrictPipeline`, `expandShellTypes`, and `sanitizeQuotes`.

**Acceptance — all must hold:**

| Input | Expected |
|---|---|
| `{name: "John"}` | works |
| `{_id: ObjectId("507f…")}` | works |
| `{note: "hello, world: x"}` | **works, not corrupted** |
| `{age: {$gt: 18}}` | works |
| `{name: /^jo/i}` | works (regex) |
| `{created: ISODate("2024-01-01")}` | works |
| `{label: "ObjectId(\"507f…\")"}` | treated as a literal string, **not rewritten** |
| `[{$match:{x:1}},{$group:{_id:"$y"}}]` | pipeline works |
| `{bad json` | clear inline error, **no backend call** |

---

## Phase 2 — Backend confirm + cleanup

**Files:** `commands.rs` (+ tests).

- Confirm `parse_filter` decodes the Phase-1 EJSON unchanged.
- Rename `parse_filter` → `parse_ejson_document` (it parses filter / projection / sort /
  insert-doc / `_id` filter — the current name has misled us repeatedly).
- Decide on `normalize_smart_quotes`: keep as a paste-safety backstop (lean: keep — cheap insurance).
- Add Rust tests: ObjectId / date / regex / nested-operator EJSON → correct BSON.
- Honor the repo Rust rules: no `?` (expand to `match`), no field shorthands.

**Acceptance:** `cargo build` + `cargo test` green; new tests cover the tricky types.

---

## Phase 3 — Read-path verification pass

Non-obvious correctness items; each gets a test or a fix.

1. **Sort key order** — `{a:1, b:-1}` must preserve order end-to-end (JS parse →
   `EJSON.stringify` → Rust `str→bson::Document`, which is order-preserving). The link
   I'm least sure of; prove with a test.
2. **Result render round-trip** — confirm `find_documents` output EJSON shape
   (`$oid`/`$date`/`$numberLong`/`$numberDecimal`) matches what `formatCell` renders.
   Fix any mismatch in one place.
3. **Pagination edges** — disable Next when `results.length < limit` (no empty trailing
   page); clamp `limit: 0` (Mongo reads 0 as "no limit").
4. **Projection conflict** — inclusion+exclusion mix surfaces Mongo's error cleanly.

**Acceptance:** each row has a passing test or a recorded confirmed-correct manual check.

---

## Phase 4 — Regression guard (recommended, optional)

- Stand up **Vitest** and test `queryParser.js` (a pure function — the highest-value,
  lowest-friction first frontend test). Pins the beta-critical parsing against regressions.
- Opt-in: adds dev tooling, but the parser is exactly what must not silently break post-beta.

---

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Parser blocked by Tauri CSP (eval) | **Phase 0 gate** — checked before any app code |
| JS-EJSON ↔ Rust-BSON dialect mismatch | Phase 0 round-trip proof |
| Sort order lost in round-trip | Phase 3.1 explicit test |
| Aggregation pipeline parsing differs from find | covered — same parser, `parsePipeline` |
| Bundle size | non-issue — desktop app, bundled not networked |

---

## Sequencing

Phase 0 → **stop, review results** → 1 → 2 → 3, each its own commit. Phase 4 anytime after 1.
One logical change per session (per repo workflow). User commits; agent explains and waits.

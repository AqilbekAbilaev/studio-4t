# Remaining Menu Items

Every native-menu item is now a working, context-gated action **except the five below**.
Each is a substantial new subsystem — deliberately deferred to its own focused session
(they can't be built *and* verified in a shared batch without an unreviewable diff).

They remain in `src-tauri/src/menu.rs` as `Spec::Placeholder` entries, so they render
present-but-disabled until implemented.

## How a menu item gets wired (the pattern to follow)

Every item implemented this session followed the same path — reuse it:

1. **`src-tauri/src/menu.rs`** — flip the `Spec::Placeholder` to a
   `Spec::Action { id, label, accel: None, gate: Some(Gate::…) }`. Pick the gate for the
   selection level the action needs (`Connection` / `Database` / `Collection`).
2. **`src/App.vue` → `handleMenuAction(id)`** — add a `case` that either:
   - opens a modal directly (set a `…Target` ref), or
   - routes through `menuNode('<Action Label>', '<level>')`, which resolves the sidebar/tab
     target and calls `handleContextAction`, where you set the modal's target ref.
3. **Backend (if needed)** — add `#[tauri::command]` fns in a **new module** under
   `src-tauri/src/commands/` (avoids colliding with in-flight refactors), using the
   `client_for(&pool, &storage, &id)` helper. Register in `commands/mod.rs`
   (`pub mod x; pub use x::*;`) and in `lib.rs`'s `generate_handler!`.
4. **UI** — a modal component in `src/components/`, following the shared dialog markup
   (`.overlay` / `.dialog` / `.dlg-title` + a single close ✕, no traffic lights) and CSS
   custom-property tokens from `theme.css`.
5. **Verify** — `cargo test menu`, `npm test`, `npx vite build`. On Linux use the rustup
   toolchain: `source ~/.cargo/env && RUST_MIN_STACK=16777216 cargo build`.

Cross-component actions (menu → an already-open modal/panel) use a one-shot
`{ action, nonce }` "request" ref watched by the target component — see `docMenuRequest`
(ResultsPanel), `historyRequest`/`saveRequest` (QueryBar), and `gridfsRequest` (GridFsModal).

---

## 1. `view:split_v` / `view:split_h` — Split Vertically / Horizontally

**What it is:** split the workspace/results area into two independently-scrolling panes so
two collections (or two views of one) can be seen side by side or stacked.

**Why it's big:** needs a real pane-layout engine — a splittable container, a draggable
divider, and two independent `ResultsPanel`/tab hosts with their own active-tab state. Today
`App.vue` owns a single `tabs` list and one active tab; this becomes a tree/pair of panes,
each with its own active tab. Persisting the split across restarts (session restore) adds more.

**Sketch:**
- Introduce a `panes` model (e.g. `[{ id, tabs, activeTabId }]`) or a split wrapper around the
  existing single-pane state; `view:split_v/h` splits the active pane and assigns orientation.
- A `SplitContainer.vue` with a draggable gutter; render one `QueryWorkspace` per pane.
- Route new-tab/close-tab/activate to the focused pane.
- Highest value of the five, but the largest refactor of `App.vue` tab state.

## 2. `coll:reschema` — Reschema…

**What it is:** restructure a collection's documents — rename/move/retype fields, split or
merge collections — as a repeatable transformation (Studio-3T's "Reschema").

**Why it's big:** a transformation-builder UI plus an execution engine. Needs a mapping model
(source field → target field/type/expression), a preview over sampled documents, and a bulk
apply (aggregation `$project`/`$out` or batched updates) with progress + error handling.

**Sketch:**
- Backend: a `reschema_preview` (run the mapping pipeline with a `$limit`) and a `reschema_apply`
  (`$out`/`$merge` or batched `bulkWrite`).
- UI: a mapping editor (rows of source → target + type/expression), a sampled before/after
  preview, and an apply step with a confirm + progress.
- Consider building on the existing aggregation/pipeline parsing (`parse_pipeline`).

## 3. `file:manage_sql` — Manage SQL Connections

**What it is:** create/edit/remove saved **SQL** source connections (used by the SQL → MongoDB
migration feature) — a parallel to the MongoDB Connection Manager.

**Why it's big:** the app currently stores/manages MongoDB connections only. This needs a
separate SQL connection config type, persisted store, and a management UI, plus SQL-driver
connectivity (test-connection).

**Sketch:**
- Backend: a `SqlConnectionConfig` + a JSON store (mirror `storage.rs`), and
  list/save/delete/test commands. Check what `MigrationModal`/`SqlModal` already assume about
  SQL connections and reuse it.
- UI: a `SqlConnectionManager.vue` mirroring `ConnectionManager.vue` + a new/edit form.

## 4. `file:tasks` — Open Tasks

**What it is:** a panel listing long-running background operations (exports/imports, copies,
map-reduce, reschema) with status/progress and cancel.

**Why it's big:** there is no task subsystem today — long operations run inline and report only
a final toast. Needs a task registry (id, kind, state, progress), backend commands that report
progress (events) and support cancellation, and a Tasks panel that subscribes.

**Sketch:**
- Backend: a task manager (spawn + track handles), emit `task-progress`/`task-done` events,
  and a `cancel_task` command. Retrofit the bulk ops (db export/import, copy, reschema) to
  register tasks and stream progress instead of blocking.
- UI: a `TasksPanel.vue` listening to task events, with per-task progress + cancel.
- Note: `kill_query` already shows the cancellation pattern for a single query.

---

## Status snapshot

- Implemented this session: File, Edit, Database, Collection, Index, Document, GridFS, View,
  and Help menus — including GridFS file/bucket ops, Manage Users/Roles, Stored Functions,
  Map-Reduce, Copy/Paste, db-level Export/Import, Add View, Validator, Database/Server stats,
  Current Operations, Server Status Charts, tab/view-mode/drill/history View actions, and the
  Help links (default to the GitHub repo; retarget in `App.vue`'s `HELP_URLS`).
- Remaining: the five subsystems above.

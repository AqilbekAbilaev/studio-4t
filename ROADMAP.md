# Studio-4T Roadmap

## Done ✅

### Foundation
- Native system menu bar (File, Edit, Database, Collection, Index, GridFS, View, Help)
- Window opens maximized by default
- Async connection pool — one `Client` per connection, reused across operations; concurrent
  connections don't block each other
- Fast TCP probe — instant "connection refused" feedback before the MongoDB handshake
- Connection URI assembled in Rust from stored config (`uri::build_uri`) — the backend is the
  source of truth; the frontend never passes credentials
- Rust backend modules: `error`, `storage`, `pool`, `uri`, `keychain`, `commands`, `menu`,
  `persist`, `shell`, `ssh`
- Unit tests — 50 covering storage, URI building, pool state, and the shell arg→BSON converter
- **Backend hardening** — atomic JSON writes (temp-file + rename) and per-store read-modify-write
  locking so a crash mid-write can't corrupt a file and concurrent commands can't lose updates;
  one IntelliShell worker thread per session (a slow eval can't stall other shells); a `maxTimeMS`
  cap on find/aggregate/count so a runaway query aborts server-side instead of hanging the UI;
  central error logging with error categories (`AppError::code`)
- Stable rendering on Linux/WebKitGTK — DMABUF renderer disabled + repaint-on-refocus so the
  result grid doesn't go blank / freeze scroll after switching windows
- Working text-editing shortcuts on Linux/WebKitGTK — the native Edit menu is left empty on Linux
  (its predefined accelerators were swallowing Ctrl+C/V/X/A in inputs); and since WebKitGTK has no
  native undo for form fields, a small JS shim (`utils/inputUndo.js`) gives every input/textarea
  Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y, installed in both webviews

### Connections
- Connection Manager — modal grid (Name / DB Server / Security / Last Accessed) with filter row
- New / **Edit** Connection dialog — structured fields (host, port, connection type, replica set,
  username, auth source, **auth mechanism**), Test Connection, Save & Connect
- **Password in OS keychain** — never written to disk; macOS Keychain / Linux Secret Service /
  Windows Credential Manager via the `keyring` crate, fetched at connect time
- **TLS / SSL** — enable TLS with a custom CA file, a client certificate (`tlsCAFile` /
  `tlsCertificateKeyFile`), and an "allow invalid certificates" option for self-signed deployments;
  configured in the connection dialog's SSL tab and applied on Test Connection + connect
- **SSH tunnel** — connect through a bastion: password or private-key auth (+ passphrase), a local
  port forwarded to the remote MongoDB host (pure-Rust `russh`), torn down on disconnect; configured
  in the SSH Tunnel tab. Standalone host only for now (no replica-set/SRV over SSH). Host-key
  verification with an **interactive trust-on-first-use** flow — on first contact the user is shown
  the bastion's `SHA256` fingerprint and must approve it before the key is recorded
  (`known_hosts.json`); the key is verified on every later connect; a changed key is hard-rejected
  with a warning and a "forget saved key" action to re-trust after a legitimate key rotation
- **Structured connection storage** — `ConnectionConfig` holds individual fields, not a raw URI
- Color-tagging (red/blue/green/purple/none), honored through the tree subtree
- **Open-connection persistence** — the sidebar shows only the *open* connections (the full saved
  list lives in the Manager); the open set is persisted, so only those re-open after a restart
- Connection tree — collapsible Connection → Database → Collection, live data on expand
- Tree context menu: Open Collection, Copy Name, Disconnect / Others / All, Refresh / Refresh All
- **Per-connection status indicator** — a status dot on each connection row in the tree
  (connected / loading / error / idle), derived from the tree's own load state

### Query workspace
- Multiple tabs, each bound to a collection with its own query state; auto-run on open
- **Double-click to open** a collection (single click only selects/highlights, Studio-3T style);
  the same collection can be opened in **several tabs** at once
- **Tab overflow** — when tabs exceed the strip width they collapse into a "+N" button with a
  dropdown of the hidden tabs (no scrollbar); the active tab is always kept visible
- **Tab right-click menu** — Close (Tab / Others / Left / Right / All), Duplicate Tab (clones the
  collection + full query state), Move Tab to the Front, Rename Tab, and Choose Color (per-tab
  color tag); custom titles and colors persist across restarts
- Query bar: filter / sort / projection / skip / limit with syntax-colored JSON; Ctrl/Cmd+Enter to run
- **ObjectId helpers** — accepts shell-style `ObjectId("…")`, and pasting a bare 24-hex id
  auto-builds `{ _id: ObjectId("…") }`
- Result paging: first / prev / next, page-size picker (10/25/50/100/200)
- View modes: **Table View**, **JSON View** (syntax-highlighted, selectable, text cursor), and
  **Tree View** (expandable Key / Value / Type rows, EJSON-aware — renders ObjectId / Date / number
  wrappers as scalars)
- "Query Code" sub-tab — the equivalent `db.collection.find(...)` shell command
- Result table — zebra rows, type-colored cells, row/cell selection, resizable columns
- **Inline cell editing** (double-click a primitive; commits via `replace_document`)
- **Nested object & array drill-down** with a breadcrumb path
- Cell/row context menu: Copy Value / Copy as JSON / Copy Document
- Document CRUD — insert / replace / delete via `DocumentModal.vue`
- Large-result rendering memoized (no O(n²) column scan); typing in the query bar no longer leaks
  into grid key-navigation
- **Aggregation pipeline** runner — a Find/Aggregate mode toggle on the collection tab; pipeline
  editor (`run_aggregate`) reusing the existing Table / JSON result views
- **Explain** sub-tab — server `explain` (executionStats) for the current query, with a
  stage / docs-examined / keys-examined / time summary (`explain_query`)
- **Query history** per collection — last 50 queries persisted in `history.json`; dropdown on the
  toolbar button; restores filter / sort / projection / skip / limit and auto-runs; deduplicates by
  content (move-to-top); covers both Find and Aggregate modes; pagination runs (prev/next/first)
  are excluded from history
- **Save / Load query (Query Browser)** — "Save query" toolbar popover prompts for a name and
  persists to `saved_queries.json` (global, not per-collection); "Load query" opens a modal with
  search, Name / Mode / Saved columns, a preview panel, and Delete + Load actions
- **Visual Query Builder** — a panel that generates the actual filter/sort/projection
  (`utils/vqbGenerator.js`); result-grid cells are draggable any time and a drag opens the panel
  automatically (and closes it again if the drag is released outside it); a dropped cell pastes
  both its field *and* value into the Query section; pressing Enter in a condition input applies
  and runs the query; `$and` generates a flat implicit-AND object (only wrapping in `$and` when two
  conditions target the same field); operator dropdown styled to match the design system; the panel
  is resizable with a sidebar-style handle and remembers its width
- **Default query per collection** — set / clear a default filter for a collection
  (`default_queries.rs`, persisted) that auto-loads when the collection is opened
- **Copy / paste query between tabs** — copy a tab's full query state and paste it into another tab
- **Tab persistence** — open tabs (and their query state) are saved and restored across app restarts
- **IntelliShell** — an embedded, mongosh-style JavaScript shell (a shell tab opened from a
  database/connection node). A pure-Rust `boa_engine` runs on a dedicated worker thread (one
  persistent JS context per session, so variables survive across submissions); native calls bridge
  `db.<coll>.<method>(…)` to the driver. Supports CRUD, `aggregate`, `runCommand`, a chainable
  cursor (`limit/skip/sort/projection` + `toArray/forEach/map/…`), index/admin helpers, and EJSON
  constructors (`ObjectId`, `ISODate`, `NumberLong/Int/Decimal`). Output is pretty-printed +
  syntax-highlighted; command history persists per connection (`shell_history.json`)

### Collection & database
- Create a collection (`create_collection`) and drop a database (`drop_database`, confirm dialog)
- **Drop a collection** (`drop_collection`, confirm dialog) and **rename a collection**
  (`rename_collection`, admin `renameCollection`)
- **Add a database** (`create_database`) — name + required first collection
- **Index management** — list / create (keys, unique, optional name) / drop indexes on a
  collection (`list_indexes`, `create_index`, `drop_index`); the default `_id_` index is protected
- **Import / Export** a collection to/from JSON and CSV via native OS dialogs
  (`export_collection`, `import_collection`; `tauri-plugin-dialog`, Rust-side file I/O)
- **Count Documents & Last-page paging** — a filter-aware `count_documents` command backs both
  the footer "Count Documents" action and the result toolbar's Last-page button (jumps to the
  final page); the paging range is now skip-aware and shows "of N" once a count is taken

### Design system
- `BaseIcon.vue` inline SVG icon set (no icon fonts/images); theming via CSS custom properties
- `QueryWorkspace.vue` split into focused components (`TabBar`, `QuickstartPane`, `QueryBar`,
  `PipelineEditor`, `ResultsPanel`, `ResultTable`); the workspace is now a slim orchestrator that
  owns the parse + run pipeline

---

## Backlog by priority 📋

Most of these already have a button or menu item in the UI, currently disabled or showing a
"coming soon" / "coming to Studio-4T" stub.

### P1 — Medium — productivity & polish
- [x] **IntelliShell editor** — the shell's plain textarea is now a **CodeMirror 6** editor: live
  JavaScript syntax highlighting (themed with the app's token palette), line numbers, bracket
  matching, and a **Mongo-aware autocomplete** (collection names after `db.`, collection/cursor
  methods after a member access, shell globals otherwise). **Run current line** (⌘⇧⏎ / toolbar)
  joins Run-all (⌘⏎). CodeMirror is code-split (the shell view lazy-loads) so the initial bundle is
  unchanged. *Remaining: Save / Open shell scripts (needs a script store) — tracked as a follow-up.*
- [x] Connection **Duplicate / Import / Export / To-URI** (Manager toolbar) — `duplicate_connection`
  clones a saved connection (config + keychain secrets) under a "(copy)" name; `connection_uri`
  copies the connection string to the clipboard (password excluded); `export_connections` /
  `import_connections` back up all connections to a JSON file and restore them (credential-free —
  passwords live in the keychain, so imported connections need their password re-entered)
- [x] **Server status** panel (host, version, uptime, connections, memory) — admin `serverStatus`
  via a `server_status` command, shown in a modal (stat-card grid + collapsible raw JSON), opened
  from the connection tree's right-click *Server Info → Server Status*
- [x] **Preferences** window — opened from File → Preferences; a persisted `settings.json`
  (`get_settings` / `update_settings`) holds a **default query limit** that newly opened
  collection tabs adopt, plus a shortcut into the keyboard-shortcuts reference. *Theme toggle is
  tracked separately under P2.*

### P2 — Later — advanced / nice-to-have
- [ ] **GridFS** — browse buckets, upload / download files
- [ ] **Schema** inference & visualization
- [ ] **SQL → MQL** translator
- [ ] **Data masking** — export an obfuscated copy
- [ ] **SQL Migration** — generate CREATE TABLE + INSERT from a collection
- [ ] **Dark / light theme** toggle
- [x] **Keyboard shortcuts** reference — a read-only modal (Help → Keyboard Shortcuts) listing the
  shortcuts the app actually handles, grouped by area, with platform-aware modifier symbols.
  *Customization is still future work.*

### Hardening follow-ups (deferred from the backend pass)
- [x] **Cancel running query** — a real Cancel button. Each find/aggregate is tagged with a unique
  run id (as its `comment`); `kill_query` finds the op via `$currentOp` (own ops, no elevated
  privilege) and `killOp`s it. The cancelled run renders a calm "Query cancelled" state, and a
  server that refuses the kill surfaces a clear message. Complements the existing `maxTimeMS` cap.
- [x] **Structured errors to the frontend** — `AppError` now serializes as `{ code, message }`,
  with Mongo errors classified into auth / tls / network sub-categories; the `errMessage` /
  `errCode` helpers read that shape at every call site, and the shared `StateMessage` placeholder
  branches on the code to show an actionable hint (auth vs network vs TLS) + Retry
- [x] **SSH known-hosts / trust-on-first-use** — interactive fingerprint prompt on first contact
  (user approves before the key is recorded); verified thereafter; changed keys hard-rejected with a
  warning + a "forget saved key" recovery action. *Remaining gap:* no full known-hosts manager
  screen (forget is per-incident, from the changed-key dialog)
- [x] **Integration tests** against a live MongoDB — env-gated (`STUDIO4T_TEST_MONGODB=host[:port]`)
  tests that build the URI the pool feeds the driver (`uri::build_uri` + `with_timeout`) and exercise
  the real driver paths the commands wrap: paging `find` (sort/skip/limit), `count_documents`, and
  `aggregate`, against a throwaway DB that's dropped after. They skip cleanly when the var is unset,
  so default `cargo test` stays green. *(Broader command-level coverage can grow from here.)*

---

> Note: an experimental `studio3t-parity` branch contains unmerged prototypes of several
> backlog items (GridFS, server status, preferences, SQL→MQL, schema, masking).
> Treat it as a reference, not shipped behavior.

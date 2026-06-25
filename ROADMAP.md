# Studio-4T Roadmap

## Done ✅

### Foundation
- Native system menu bar (File, Edit, Database, Collection, Index, GridFS, View, Help)
- Async connection pool — one `Client` per connection, reused across operations; concurrent
  connections don't block each other
- Fast TCP probe — instant "connection refused" feedback before the MongoDB handshake
- Connection URI assembled in Rust from stored config (`uri::build_uri`) — the backend is the
  source of truth; the frontend never passes credentials
- Rust backend modules: `error`, `storage`, `pool`, `uri`, `keychain`, `commands`, `menu`
- Unit tests — 32 covering storage, URI building, and pool state
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
- **Structured connection storage** — `ConnectionConfig` holds individual fields, not a raw URI
- Color-tagging (red/blue/green/purple/none), honored through the tree subtree
- **Open-connection persistence** — the sidebar shows only the *open* connections (the full saved
  list lives in the Manager); the open set is persisted, so only those re-open after a restart
- Connection tree — collapsible Connection → Database → Collection, live data on expand
- Tree context menu: Open Collection, Copy Name, Disconnect / Others / All, Refresh / Refresh All

### Query workspace
- Multiple tabs, each bound to a collection with its own query state; auto-run on open
- Query bar: filter / sort / projection / skip / limit with syntax-colored JSON; Ctrl/Cmd+Enter to run
- **ObjectId helpers** — accepts shell-style `ObjectId("…")`, and pasting a bare 24-hex id
  auto-builds `{ _id: ObjectId("…") }`
- Result paging: first / prev / next, page-size picker (10/25/50/100/200)
- View modes: **Table View** and **JSON View** (syntax-highlighted, selectable)
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
  content (move-to-top); covers both Find and Aggregate modes
- **Save / Load query (Query Browser)** — "Save query" toolbar popover prompts for a name and
  persists to `saved_queries.json` (global, not per-collection); "Load query" opens a modal with
  search, Name / Mode / Saved columns, a preview panel, and Delete + Load actions

### Collection & database
- Create a collection (`create_collection`) and drop a database (`drop_database`, confirm dialog)
- **Drop a collection** (`drop_collection`, confirm dialog) and **rename a collection**
  (`rename_collection`, admin `renameCollection`)
- **Add a database** (`create_database`) — name + required first collection
- **Index management** — list / create (keys, unique, optional name) / drop indexes on a
  collection (`list_indexes`, `create_index`, `drop_index`); the default `_id_` index is protected
- **Import / Export** a collection to/from JSON and CSV via native OS dialogs
  (`export_collection`, `import_collection`; `tauri-plugin-dialog`, Rust-side file I/O)

### Design system
- `BaseIcon.vue` inline SVG icon set (no icon fonts/images); theming via CSS custom properties

---

## Backlog by priority 📋

Most of these already have a button or menu item in the UI, currently disabled or showing a
"coming soon" / "coming to Studio-4T" stub.

### P1 — Medium — productivity & polish
- [ ] **Visual Query Builder** — wire the panel to generate/sync the actual filter/sort/projection
- [ ] **Tree View** result mode (Key / Value / Type, expandable)
- [ ] **Tab persistence** across app restarts
- [ ] **Per-connection status** indicator in the tree (connected / loading / error)
- [ ] Connection **Duplicate / Import / Export / To-URI** (Manager toolbar stubs)
- [ ] **Last-page** paging button (currently disabled)
- [ ] **Server status** panel (host, version, uptime, connections, memory)
- [ ] **Preferences** window (theme, default query limit, shortcuts list)
- [ ] **IntelliShell** — embedded `db.<coll>.<method>(…)` console

### P2 — Later — advanced / nice-to-have
- [ ] **GridFS** — browse buckets, upload / download files
- [ ] **Schema** inference & visualization
- [ ] **SQL → MQL** translator
- [ ] **Data masking** — export an obfuscated copy
- [ ] **SQL Migration** — generate CREATE TABLE + INSERT from a collection
- [ ] **Dark / light theme** toggle
- [ ] **Keyboard shortcuts** reference / customization

---

> Note: an experimental `studio3t-parity` branch contains unmerged prototypes of several
> backlog items (Tree View, GridFS, server status, preferences, SQL→MQL, schema, masking).
> Treat it as a reference, not shipped behavior.

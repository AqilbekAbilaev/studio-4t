# Studio-4T Roadmap

## Done ✅

### Foundation
- Native macOS menu bar (File, Edit, Database, Collection, Index, GridFS, View, Help)
- Connection pool — one `Client` per connection, reused across operations
- Async MongoDB commands — concurrent connections don't block each other
- Fast TCP probe — instant "connection refused" feedback before the MongoDB handshake
- Rust backend module structure: `error`, `storage`, `pool`, `uri`, `commands`
- Unit tests — 22 tests covering storage, URI utilities, and pool state

### Connections
- Connection Manager redesign — modal with grid (Name / DB Server / Security / Last Accessed),
  filter row, toolbar (New/Edit/Delete/Duplicate/Import/Export/To URI stubs wired for New/Edit/Delete)
- New Connection dialog — Server tab (host/port/auth) + URI tab, Test Connection, Save & Connect
- Persist connections to disk (`app_data_dir/connections.json`)
- Connection color-tagging (red/blue/green/purple/none), honored through the tree subtree
- Connection tree — collapsible Connection → Database → Collection nodes, live data on expand
- Tree context menu: Open Collection, Copy Name, Disconnect / Disconnect Others / Disconnect All,
  Refresh / Refresh All — all wired to real behavior
- `update_last_accessed`, `set_connection_tag`, `delete_connection` backend commands
- **Structured connection storage** — `ConnectionConfig` stores individual fields (host, port,
  connection type, replica set name, username, auth DB) instead of a raw URI string; URI is
  assembled in Rust at connect time via `uri::build_uri()`
- **Password in OS keychain** — passwords never touch `connections.json`; stored in macOS Keychain /
  Linux Secret Service / Windows Credential Manager via the `keyring` crate; fetched at connect time
- New connection auto-expands in sidebar after save + connect

### Query workspace
- Multiple tabs, each bound to a collection + its own query state
- Auto-run query on collection open
- Query bar: filter (JSON), sort, projection, skip, limit fields with syntax-colored JSON
- Skip/Limit as steppers (click +/- or type directly)
- Ctrl+Enter / Cmd+Enter to run
- Result paging: first/prev/next/last, page-size picker (20/50/100/...)
- View modes: **Table View** and **JSON View** (with syntax highlighting + text selection) —
  **Tree View** is in the menu but not yet implemented (falls through to "coming soon")
- "Query Code" sub-tab — renders the equivalent `db.collection.find(...)` shell command
- "Explain" sub-tab is a placeholder ("coming soon")
- Result table redesign — zebra rows, `_id`/number/string cell coloring, row selection,
  resizable columns (no layout jump, centered handle)
- **Inline cell editing** — double-click a primitive cell to edit in place; commits via `replace_document`
- **Nested object & array drill-down** — double-click an object/array cell to drill in,
  with a breadcrumb path (collection ▸ field ▸ …) to navigate back out
- Right-click cell/row context menu: Copy Value, Copy as JSON, Copy Document
- Document CRUD: insert (`insert_document`), edit/replace (`replace_document`),
  delete (`delete_document`) via `DocumentModal.vue`
- Compact result-toolbar menus (page size, view mode)

### Collection & database management
- Create a new collection — `create_collection` backend command + dialog, wired through the tree
- Drop a database — `drop_database` backend command + confirmation modal (no longer a toast stub)

### Design system
- `BaseIcon.vue` — inline SVG icon set, no external icon fonts/images
- Theming via CSS custom properties in `theme.css`

---

## Up Next 📋

### Query workspace gaps
- [ ] Tree View result mode (menu entry exists, not implemented)
- [ ] Explain sub-tab (currently a "coming soon" placeholder)
- [ ] Visual Query Builder — toggle now opens `VisualQueryBuilder.vue` as a side panel, but it
      doesn't yet generate/sync the actual filter/sort/projection
- [ ] Load query from file / Save query to file
- [ ] Query history (recent queries per collection)
- [ ] Tab persistence across app restarts

### Connection management
- [ ] Duplicate connection, Import/Export, "To URI" — present in Connection Manager toolbar but
      not all wired yet (verify each individually)
- [ ] Per-connection status indicator (connected / disconnected / error) in the tree

### Collection & database management
- [ ] Create a new database
- [ ] Drop a collection (menu item exists, not wired — shows toast stub)
- [ ] Rename a collection (menu item exists, not wired — shows toast stub)

### IntelliShell
- [ ] Embedded MongoDB shell (toolbar/menu entries exist, not implemented — toast stub)

### Index management
- [ ] List indexes on a collection
- [ ] Create an index
- [ ] Drop an index

### Data import / export
- [ ] Export collection to JSON / CSV
- [ ] Import JSON / CSV into a collection

### GridFS
- [ ] Browse GridFS buckets
- [ ] Upload / download files

### Server & preferences
- [ ] Server status panel (connections, memory, uptime)
- [ ] Preferences window (theme, default query limit, etc.)

---

## Nice to Have 💡

- [ ] SQL to MQL translator (like Studio-3T's SQL query feature) — toolbar entry exists as stub
- [ ] Schema visualization — infer and display the shape of a collection — toolbar entry exists as stub
- [ ] Aggregation pipeline builder — toolbar entry exists as stub
- [ ] Data masking — toolbar entry exists as stub
- [ ] SQL Migration — menu entry exists as stub
- [ ] Dark / light theme toggle
- [ ] Keyboard shortcuts reference / customization

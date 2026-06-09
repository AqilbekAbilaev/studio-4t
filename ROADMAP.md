# Studio-4T Roadmap

## Done ✅

- Basic UI layout — split pane (sidebar + dashboard), draggable divider
- Native macOS menu bar (File, Edit, Database, Collection, Index, GridFS, View, Help)
- New Connection dialog — Server tab (host/port/auth fields) + URI tab, Test Connection, Save & Connect
- Persist connections to disk (`app_data_dir/connections.json`)
- Connection tree — collapsible Connection → Database → Collection nodes
- Live MongoDB data — real databases and collections loaded on expand
- Connection pool — one `Client` per connection, reused across operations (no reconnect per call)
- Async MongoDB commands — concurrent connections don't block each other
- Fast TCP probe — instant "connection refused" feedback before invoking the MongoDB driver
- Rust backend module structure: `error`, `storage`, `pool`, `uri`, `commands`
- Unit tests — 22 tests covering storage, URI utilities, and pool state

---

## In Progress 🔧

- Query tab UI — toolbar exists (Run, Load query, Save query, Visual Query Builder, Query input) but nothing is wired up yet

---

## Up Next 📋

### Core query workflow
- [ ] Execute a query (Run button) — send MQL to the selected collection, get documents back
- [ ] Display query results — document list / table view in the dashboard panel
- [ ] View a single document in a readable format (JSON tree or formatted view)
- [ ] Edit a document inline and save changes
- [ ] Insert a new document
- [ ] Delete a document

### Connection management
- [ ] Edit an existing connection (rename, change URI)
- [ ] Delete a connection from the sidebar
- [ ] Disconnect / reconnect a connection manually
- [ ] Connection status indicator (connected / disconnected / error)

### Collection & database management
- [ ] Create a new database
- [ ] Drop a database
- [ ] Create a new collection
- [ ] Drop a collection
- [ ] Rename a collection

### Tabs
- [ ] Multiple tabs in the dashboard — each tab tied to a collection + query
- [ ] Tab persistence across sessions

### Query tools
- [ ] Load query from file
- [ ] Save query to file
- [ ] Query history (recent queries per collection)
- [ ] Visual Query Builder — GUI for building MQL filters without typing

### IntelliShell
- [ ] Embedded MongoDB shell (run raw JS/MQL commands)

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

- [ ] SQL to MQL translator (like Studio-3T's SQL query feature)
- [ ] Schema visualization — infer and display the shape of a collection
- [ ] Aggregation pipeline builder
- [ ] Dark / light theme toggle
- [ ] Keyboard shortcuts for common actions
